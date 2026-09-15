//! Node-local admission accounting. Closing admission does not prove runtime
//! cleanup, durable drain intent, or permission to terminate the host.

use std::sync::{Arc, Mutex};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AdmissionStatus {
    pub closed: bool,
    pub in_flight: usize,
}

#[derive(Clone, Default)]
pub(super) struct NodeAdmission {
    state: Arc<Mutex<AdmissionState>>,
    marker: Option<PathBuf>,
}

#[derive(Default)]
struct AdmissionState {
    status: AdmissionStatus,
    /// Changes on every admitted start, even if it finishes during an idle
    /// inventory read. Zero in-flight alone cannot detect that race.
    grants: u64,
}

pub(super) struct AdmissionPermit(Arc<Mutex<AdmissionState>>);

impl NodeAdmission {
    pub fn persistent(marker: PathBuf) -> io::Result<Self> {
        let closed = match fs::read_to_string(&marker) {
            Ok(id) => {
                validate_drain_id(&id)?;
                true
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error),
        };
        Ok(Self {
            state: Arc::new(Mutex::new(AdmissionState {
                status: AdmissionStatus {
                    closed,
                    in_flight: 0,
                },
                grants: 0,
            })),
            marker: Some(marker),
        })
    }

    /// A successful return means the marker and containing directory were
    /// synced. Any write failure leaves in-process admission closed. A partial
    /// marker makes the next startup fail closed rather than reopening the node.
    pub fn drain(&self, id: &str) -> io::Result<AdmissionStatus> {
        validate_drain_id(id)?;
        let mut state = self.state.lock().expect("node admission poisoned");
        self.persist_drain(id, &mut state)
    }

    /// Snapshot before an asynchronous inventory read. The final conditional
    /// close checks this generation while holding the same lock as acquire.
    pub fn idle_epoch(&self) -> Option<u64> {
        let state = self.state.lock().expect("node admission poisoned");
        (state.status.in_flight == 0).then_some(state.grants)
    }

    /// None is a busy rejection with no admission or persistence change.
    /// Replaying a completed drain still validates the exact durable ID.
    pub fn drain_if_unchanged(&self, id: &str, epoch: u64) -> io::Result<Option<AdmissionStatus>> {
        validate_drain_id(id)?;
        let mut state = self.state.lock().expect("node admission poisoned");
        if !state.status.closed && (state.grants != epoch || state.status.in_flight != 0) {
            return Ok(None);
        }
        self.persist_drain(id, &mut state).map(Some)
    }

    fn persist_drain(&self, id: &str, state: &mut AdmissionState) -> io::Result<AdmissionStatus> {
        state.status.closed = true;
        let marker = self.marker.as_ref().ok_or_else(|| {
            io::Error::other("durable node drain requires a persistent admission store")
        })?;
        let parent = marker
            .parent()
            .ok_or_else(|| io::Error::other("missing marker parent"))?;
        // The caller provisions the node's durable state directory before
        // starting service. Never acknowledge a marker on an absent mount.
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(marker)
        {
            Ok(mut file) => {
                file.write_all(id.as_bytes())?;
                file
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if fs::read_to_string(marker)? != id {
                    return Err(io::Error::other(
                        "node already has a different drain identity",
                    ));
                }
                fs::OpenOptions::new().write(true).open(marker)?
            }
            Err(error) => return Err(error),
        };
        file.flush()?;
        file.sync_all()?;
        fs::File::open(parent)?.sync_all()?;
        Ok(state.status)
    }

    pub fn acquire(&self) -> Option<AdmissionPermit> {
        let mut state = self.state.lock().expect("node admission poisoned");
        if state.status.closed {
            return None;
        }
        state.grants = state
            .grants
            .checked_add(1)
            .expect("admission generation overflow");
        state.status.in_flight = state
            .status
            .in_flight
            .checked_add(1)
            .expect("admission overflow");
        Some(AdmissionPermit(Arc::clone(&self.state)))
    }

    /// Irreversible for this service instance. Existing permits remain counted
    /// until the actual cancellation-safe operation finishes, including rollback.
    #[cfg(test)]
    pub fn close(&self) -> AdmissionStatus {
        let mut state = self.state.lock().expect("node admission poisoned");
        state.status.closed = true;
        state.status
    }

    pub fn status(&self) -> AdmissionStatus {
        self.state.lock().expect("node admission poisoned").status
    }
}

impl Drop for AdmissionPermit {
    fn drop(&mut self) {
        let mut state = self.0.lock().expect("node admission poisoned");
        state.status.in_flight = state
            .status
            .in_flight
            .checked_sub(1)
            .expect("admission underflow");
    }
}

fn validate_drain_id(id: &str) -> io::Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    {
        return Err(io::Error::other("invalid durable drain identity"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "agentenv-admission-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn marker(&self) -> PathBuf {
            self.0.join("drain")
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn conditional_drain_rejects_in_flight_and_completed_intervening_starts_without_side_effects() {
        let directory = TestDirectory::new();
        let gate = NodeAdmission::persistent(directory.marker()).unwrap();
        let epoch = gate.idle_epoch().unwrap();
        let permit = gate.acquire().unwrap();
        assert!(gate.idle_epoch().is_none());
        assert_eq!(gate.drain_if_unchanged("idle:1", epoch).unwrap(), None);
        assert!(!gate.status().closed);
        assert!(!directory.marker().exists());
        drop(permit);
        // A quick start could have completed while the inventory was read.
        assert_eq!(gate.status().in_flight, 0);
        assert_eq!(gate.drain_if_unchanged("idle:1", epoch).unwrap(), None);
        assert!(!gate.status().closed);
        assert!(!directory.marker().exists());
        assert!(gate.acquire().is_some());
        let current = gate.idle_epoch().unwrap();
        assert!(
            gate.drain_if_unchanged("idle:1", current)
                .unwrap()
                .unwrap()
                .closed
        );
        let restarted = NodeAdmission::persistent(directory.marker()).unwrap();
        assert!(restarted.acquire().is_none());
        assert!(restarted
            .drain_if_unchanged("idle:1", current)
            .unwrap()
            .is_some());
        assert!(restarted.drain_if_unchanged("foreign:1", current).is_err());
        assert_eq!(fs::read_to_string(directory.marker()).unwrap(), "idle:1");
    }

    #[test]
    fn conditional_close_and_new_admission_have_only_one_winner() {
        let directory = TestDirectory::new();
        let gate = NodeAdmission::persistent(directory.marker()).unwrap();
        let epoch = gate.idle_epoch().unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let (result, received) = mpsc::channel();
        let (release, finish) = mpsc::channel();
        let worker = {
            let gate = gate.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let permit = gate.acquire();
                result.send(permit.is_some()).unwrap();
                finish.recv().unwrap();
                drop(permit);
            })
        };
        barrier.wait();
        let closed = gate.drain_if_unchanged("idle:1", epoch).unwrap().is_some();
        let admitted = received.recv().unwrap();
        assert_ne!(closed, admitted);
        assert_eq!(directory.marker().exists(), closed);
        assert_eq!(gate.status().closed, closed);
        release.send(()).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn conditional_persistence_failure_keeps_admission_closed() {
        let directory = TestDirectory::new();
        let gate = NodeAdmission::persistent(directory.0.join("missing").join("drain")).unwrap();
        assert!(gate
            .drain_if_unchanged("idle:1", gate.idle_epoch().unwrap())
            .is_err());
        assert!(gate.status().closed);
        assert!(gate.acquire().is_none());
    }

    #[test]
    fn durable_drain_survives_restart_and_rejects_identity_replacement() {
        let directory = TestDirectory::new();
        let gate = NodeAdmission::persistent(directory.marker()).unwrap();
        let permit = gate.acquire().unwrap();
        assert_eq!(gate.drain("node-uid:1").unwrap().in_flight, 1);
        assert!(gate.acquire().is_none());
        drop(permit);
        let restarted = NodeAdmission::persistent(directory.marker()).unwrap();
        assert!(restarted.status().closed);
        assert!(restarted.acquire().is_none());
        assert_eq!(restarted.drain("node-uid:1").unwrap().in_flight, 0);
        assert!(restarted.drain("node-uid:2").is_err());
        assert_eq!(
            fs::read_to_string(directory.marker()).unwrap(),
            "node-uid:1"
        );
    }

    #[test]
    fn failed_persistence_does_not_reopen_and_corrupt_marker_blocks_startup() {
        let directory = TestDirectory::new();
        let missing = directory.0.join("absent").join("drain");
        let gate = NodeAdmission::persistent(missing).unwrap();
        assert!(gate.drain("node:1").is_err());
        assert!(gate.acquire().is_none());
        fs::write(directory.marker(), "").unwrap();
        assert!(NodeAdmission::persistent(directory.marker()).is_err());
        fs::write(directory.marker(), "node:1\ninvalid").unwrap();
        assert!(NodeAdmission::persistent(directory.marker()).is_err());
    }

    #[test]
    fn closure_accounts_for_work_until_executor_releases_it() {
        let gate = NodeAdmission::default();
        let permit = gate.acquire().unwrap();
        let (finish, wait) = mpsc::channel();
        let executor = std::thread::spawn(move || {
            let _permit = permit;
            wait.recv().unwrap();
        });
        assert_eq!(
            gate.close(),
            AdmissionStatus {
                closed: true,
                in_flight: 1
            }
        );
        assert!(gate.acquire().is_none());
        assert_eq!(gate.close().in_flight, 1);
        finish.send(()).unwrap();
        executor.join().unwrap();
        assert_eq!(
            gate.status(),
            AdmissionStatus {
                closed: true,
                in_flight: 0
            }
        );
        assert!(gate.acquire().is_none());
    }

    #[test]
    fn concurrent_admission_cannot_cross_a_completed_close() {
        let gate = NodeAdmission::default();
        let barrier = Arc::new(std::sync::Barrier::new(9));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let gate = gate.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..1000 {
                        let _permit = gate.acquire();
                        std::thread::yield_now();
                    }
                })
            })
            .collect();
        barrier.wait();
        gate.close();
        for _ in 0..1000 {
            assert!(gate.acquire().is_none());
        }
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(
            gate.status(),
            AdmissionStatus {
                closed: true,
                in_flight: 0
            }
        );
    }
}
