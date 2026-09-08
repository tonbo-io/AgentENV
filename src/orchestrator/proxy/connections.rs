//! Route-scoped transports. Retirement rejects new connects, shuts down every
//! registered socket, and waits for in-flight connect futures to release their
//! sockets before the runtime may release its guest network address.
use std::{
    collections::HashMap,
    io,
    net::{Shutdown, SocketAddr},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
    sync::{watch, Notify},
};

#[derive(Debug, Default)]
struct State {
    retired: bool,
    pending: usize,
    next_id: u64,
    sockets: HashMap<u64, std::net::TcpStream>,
    shutdown_error: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RouteConnections {
    state: Mutex<State>,
    retired: watch::Sender<bool>,
    changed: Notify,
}

impl Default for RouteConnections {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            retired: watch::channel(false).0,
            changed: Notify::new(),
        }
    }
}

fn retired_error() -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionAborted, "runtime route retired")
}

impl RouteConnections {
    pub(crate) async fn connect(self: &Arc<Self>, addr: SocketAddr) -> io::Result<RouteStream> {
        let mut retired = self.retired.subscribe();
        {
            let mut state = self.state.lock().unwrap();
            if state.retired {
                return Err(retired_error());
            }
            state.pending += 1;
        }
        let _pending = PendingConnect(self.clone());
        let stream = tokio::select! {
            biased;
            _ = async {
                while !*retired.borrow_and_update() {
                    if retired.changed().await.is_err() { break; }
                }
            } => return Err(retired_error()),
            stream = TcpStream::connect(addr) => stream?,
        };
        stream.set_nodelay(true)?;
        let stream = stream.into_std()?;
        let control = stream.try_clone()?;
        let mut state = self.state.lock().unwrap();
        if state.retired {
            // Drop both descriptors before PendingConnect acknowledges completion.
            drop(control);
            drop(stream);
            return Err(retired_error());
        }
        let stream = TcpStream::from_std(stream)?;
        let id = state.next_id;
        state.next_id = state
            .next_id
            .checked_add(1)
            .expect("route socket identity exhausted");
        state.sockets.insert(id, control);
        Ok(RouteStream {
            stream,
            _registration: Registration {
                owner: self.clone(),
                id,
            },
        })
    }

    pub(crate) fn begin_retire(&self) {
        let mut state = self.state.lock().unwrap();
        state.retired = true;
        state.shutdown_error = None;
        self.retired.send_replace(true);
        // shutdown affects every descriptor referring to the socket, even if a
        // response body or upgraded WebSocket is retained without being polled.
        let sockets = std::mem::take(&mut state.sockets);
        for (id, socket) in sockets {
            if let Err(error) = socket.shutdown(Shutdown::Both) {
                if error.kind() != io::ErrorKind::NotConnected {
                    state.shutdown_error = Some(error.to_string());
                    state.sockets.insert(id, socket);
                }
            }
        }
    }

    pub(crate) async fn wait_retired(&self) -> io::Result<()> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let state = self.state.lock().unwrap();
                if !state.retired {
                    return Err(io::Error::other("route is not retired"));
                }
                if let Some(error) = &state.shutdown_error {
                    return Err(io::Error::other(error.clone()));
                }
                if state.pending == 0 {
                    return Ok(());
                }
            }
            changed.await;
        }
    }
}

struct PendingConnect(Arc<RouteConnections>);
impl Drop for PendingConnect {
    fn drop(&mut self) {
        self.0.state.lock().unwrap().pending -= 1;
        self.0.changed.notify_waiters();
    }
}

#[derive(Debug)]
struct Registration {
    owner: Arc<RouteConnections>,
    id: u64,
}
impl Drop for Registration {
    fn drop(&mut self) {
        self.owner.state.lock().unwrap().sockets.remove(&self.id);
    }
}

#[derive(Debug)]
pub(crate) struct RouteStream {
    // Drop the actual transport before its registration is removed.
    stream: TcpStream,
    _registration: Registration,
}
impl AsyncRead for RouteStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}
impl AsyncWrite for RouteStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
impl hyper_util::client::legacy::connect::Connection for RouteStream {
    fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
        hyper_util::client::legacy::connect::Connected::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        time::{timeout, Duration},
    };

    #[tokio::test]
    async fn retirement_shuts_down_unpolled_transport_and_rejects_old_resolution() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let route = Arc::new(RouteConnections::default());
        let mut client = route.connect(addr).await.unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        client.write_all(b"a").await.unwrap();
        let mut byte = [0];
        server.read_exact(&mut byte).await.unwrap();
        route.begin_retire();
        route.wait_retired().await.unwrap();
        assert_eq!(
            timeout(Duration::from_secs(1), server.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        assert!(client.write_all(b"stale").await.is_err());
        assert!(route.connect(addr).await.is_err());
        assert!(route.state.lock().unwrap().sockets.is_empty());
    }

    #[tokio::test]
    async fn completed_transports_release_registration_before_retirement() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let route = Arc::new(RouteConnections::default());
        let client = route.connect(listener.local_addr().unwrap()).await.unwrap();
        assert_eq!(route.state.lock().unwrap().sockets.len(), 1);
        drop(client);
        assert!(route.state.lock().unwrap().sockets.is_empty());
        route.begin_retire();
        route.wait_retired().await.unwrap();
    }
    #[tokio::test]
    async fn retirement_acknowledgement_waits_for_pending_connect_ownership() {
        let route = Arc::new(RouteConnections::default());
        route.state.lock().unwrap().pending = 1;
        let pending = PendingConnect(route.clone());
        route.begin_retire();
        let waiting = route.clone();
        let mut acknowledgement = tokio::spawn(async move { waiting.wait_retired().await });
        assert!(timeout(Duration::from_millis(20), &mut acknowledgement)
            .await
            .is_err());
        drop(pending);
        timeout(Duration::from_secs(1), acknowledgement)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn concurrent_connect_and_retirement_leave_no_open_socket_authority() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        for _ in 0..32 {
            let route = Arc::new(RouteConnections::default());
            let connecting = route.clone();
            let attempt = tokio::spawn(async move { connecting.connect(addr).await });
            tokio::task::yield_now().await;
            route.begin_retire();
            timeout(Duration::from_secs(1), route.wait_retired())
                .await
                .unwrap()
                .unwrap();
            let _ = attempt.await.unwrap();
            let state = route.state.lock().unwrap();
            assert_eq!(state.pending, 0);
            assert!(state.sockets.is_empty());
        }
    }
}

/// Hyper connector bound to one route publication, never a process-global pool.
#[derive(Clone)]
pub(crate) struct RouteConnector {
    pub(crate) connections: Arc<RouteConnections>,
    pub(crate) timeout: std::time::Duration,
}
impl tower::Service<http::Uri> for RouteConnector {
    type Response = hyper_util::rt::TokioIo<RouteStream>;
    type Error = io::Error;
    type Future = Pin<Box<dyn std::future::Future<Output = io::Result<Self::Response>> + Send>>;
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, uri: http::Uri) -> Self::Future {
        let connections = self.connections.clone();
        let timeout = self.timeout;
        Box::pin(async move {
            let addr: SocketAddr = uri
                .authority()
                .ok_or_else(|| io::Error::other("missing route authority"))?
                .as_str()
                .parse()
                .map_err(|_| io::Error::other("route authority must be an IP and port"))?;
            let stream = tokio::time::timeout(timeout, connections.connect(addr))
                .await
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::TimedOut, "route connection timeout")
                })??;
            Ok(hyper_util::rt::TokioIo::new(stream))
        })
    }
}
