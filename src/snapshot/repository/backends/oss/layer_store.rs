//! Minimal object-store surface used by managed-layer leases and GC.
//!
//! The lease protocol and the GC pass only need list-with-`LastModified`,
//! get, put, stat and delete. Keeping them behind this trait lets the
//! protocol run unchanged against [`OssClient`] in production and against an
//! in-memory store with a controllable clock, an operation log, failure
//! injection and pause hooks in tests.

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;

pub(crate) use super::client::StoredObject;
use super::client::{OssClient, OssUploadArtifact};

#[async_trait]
pub(crate) trait LayerStore: Send + Sync + 'static {
    /// List every object under `prefix` with size and `LastModified`.
    async fn list_objects(&self, prefix: &str) -> Result<Vec<StoredObject>>;
    /// Read a small object; `None` when it does not exist.
    async fn get_object(&self, key: &str) -> Result<Option<Bytes>>;
    /// Write a small object.
    async fn put_object(&self, key: &str, data: Bytes) -> Result<()>;
    /// Stat one object; `None` when it does not exist.
    async fn stat_object(&self, key: &str) -> Result<Option<StoredObject>>;
    /// Delete one object. Deleting a missing object succeeds.
    async fn delete_object(&self, key: &str) -> Result<()>;
}

#[async_trait]
impl LayerStore for OssClient {
    async fn list_objects(&self, prefix: &str) -> Result<Vec<StoredObject>> {
        self.list_objects_recursive(prefix).await
    }

    async fn get_object(&self, key: &str) -> Result<Option<Bytes>> {
        match self.get_bytes(key).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if OssClient::is_not_found_error(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn put_object(&self, key: &str, data: Bytes) -> Result<()> {
        self.put_bytes(key, data, OssUploadArtifact::LayerGc).await
    }

    async fn stat_object(&self, key: &str) -> Result<Option<StoredObject>> {
        self.stat(key).await
    }

    async fn delete_object(&self, key: &str) -> Result<()> {
        self.delete(key).await
    }
}

#[cfg(test)]
pub(crate) mod fake {
    //! In-memory [`LayerStore`] for protocol tests.
    //!
    //! Its clock follows Tokio's clock (so `start_paused` tests control it)
    //! plus a manual offset, and every write stamps `LastModified` with it.
    #![allow(dead_code)]

    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use anyhow::{bail, Result};
    use async_trait::async_trait;
    use bytes::Bytes;
    use tokio::sync::oneshot;

    use super::{LayerStore, StoredObject};

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum OpKind {
        List,
        Get,
        Put,
        Stat,
        Delete,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct Op {
        pub(crate) kind: OpKind,
        pub(crate) key: String,
    }

    /// Mutable store contents handed to `before_op` hooks.
    #[derive(Default)]
    pub(crate) struct FakeObjects {
        pub(crate) objects: BTreeMap<String, (Bytes, SystemTime)>,
        pub(crate) now: Option<SystemTime>,
    }

    impl FakeObjects {
        /// Write an object stamped with the store's current time.
        pub(crate) fn put(&mut self, key: &str, data: impl Into<Bytes>) {
            let now = self.now.expect("hook runs with the store clock set");
            self.objects.insert(key.to_string(), (data.into(), now));
        }

        pub(crate) fn contains(&self, key: &str) -> bool {
            self.objects.contains_key(key)
        }

        pub(crate) fn get(&self, key: &str) -> Option<Bytes> {
            self.objects.get(key).map(|(data, _)| data.clone())
        }

        pub(crate) fn remove(&mut self, key: &str) {
            self.objects.remove(key);
        }

        pub(crate) fn keys_with_prefix(&self, prefix: &str) -> Vec<String> {
            self.objects
                .keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect()
        }
    }

    type BeforeOpHook = Box<dyn FnMut(&Op, &mut FakeObjects) + Send>;

    struct Pause {
        kind: OpKind,
        prefix: String,
        reached: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    }

    /// Handle for one paused operation: wait until it is reached, then release.
    pub(crate) struct PauseHandle {
        reached: Option<oneshot::Receiver<()>>,
        release: oneshot::Sender<()>,
    }

    impl PauseHandle {
        pub(crate) async fn reached(&mut self) {
            if let Some(reached) = self.reached.take() {
                reached.await.expect("paused operation is reached");
            }
        }

        pub(crate) fn release(self) {
            let _ = self.release.send(());
        }
    }

    #[derive(Default)]
    struct State {
        contents: FakeObjects,
        offset: Duration,
        ops: Vec<Op>,
        failures: Vec<(OpKind, String)>,
        pauses: Vec<Pause>,
        latencies: Vec<(OpKind, String, Duration)>,
        /// Seeded per-operation random latency in `0..=max`.
        jitter: Option<(u64, Duration)>,
        before_op: Option<BeforeOpHook>,
    }

    /// SplitMix64 step; deterministic so seeded interleavings replay.
    pub(crate) fn split_mix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = *state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    pub(crate) struct FakeLayerStore {
        base: SystemTime,
        started: tokio::time::Instant,
        state: Mutex<State>,
    }

    impl FakeLayerStore {
        pub(crate) fn new() -> Self {
            Self {
                base: UNIX_EPOCH + Duration::from_secs(1_800_000_000),
                started: tokio::time::Instant::now(),
                state: Mutex::new(State::default()),
            }
        }

        fn now_locked(&self, state: &State) -> SystemTime {
            self.base + self.started.elapsed() + state.offset
        }

        /// Current object-store time.
        pub(crate) fn s3_now(&self) -> SystemTime {
            let state = self.state.lock().unwrap();
            self.now_locked(&state)
        }

        /// Move the object-store clock forward without moving Tokio's clock.
        pub(crate) fn advance_s3_clock(&self, by: Duration) {
            self.state.lock().unwrap().offset += by;
        }

        pub(crate) fn insert(&self, key: &str, data: impl Into<Bytes>) {
            let mut state = self.state.lock().unwrap();
            let now = self.now_locked(&state);
            state
                .contents
                .objects
                .insert(key.to_string(), (data.into(), now));
        }

        pub(crate) fn insert_at(
            &self,
            key: &str,
            data: impl Into<Bytes>,
            last_modified: SystemTime,
        ) {
            self.state
                .lock()
                .unwrap()
                .contents
                .objects
                .insert(key.to_string(), (data.into(), last_modified));
        }

        pub(crate) fn remove(&self, key: &str) {
            self.state.lock().unwrap().contents.objects.remove(key);
        }

        pub(crate) fn contains(&self, key: &str) -> bool {
            self.state
                .lock()
                .unwrap()
                .contents
                .objects
                .contains_key(key)
        }

        pub(crate) fn read(&self, key: &str) -> Option<Bytes> {
            self.state
                .lock()
                .unwrap()
                .contents
                .objects
                .get(key)
                .map(|(data, _)| data.clone())
        }

        pub(crate) fn keys(&self, prefix: &str) -> Vec<String> {
            self.state.lock().unwrap().contents.keys_with_prefix(prefix)
        }

        pub(crate) fn ops(&self) -> Vec<Op> {
            self.state.lock().unwrap().ops.clone()
        }

        pub(crate) fn clear_ops(&self) {
            self.state.lock().unwrap().ops.clear();
        }

        /// Fail every `kind` operation on keys starting with `prefix`.
        pub(crate) fn fail(&self, kind: OpKind, prefix: &str) {
            self.state
                .lock()
                .unwrap()
                .failures
                .push((kind, prefix.to_string()));
        }

        pub(crate) fn clear_failures(&self) {
            self.state.lock().unwrap().failures.clear();
        }

        /// Delay every `kind` operation on keys starting with `prefix`.
        pub(crate) fn set_latency(&self, kind: OpKind, prefix: &str, latency: Duration) {
            self.state
                .lock()
                .unwrap()
                .latencies
                .push((kind, prefix.to_string(), latency));
        }

        /// Delay every operation by a seeded random latency in `0..=max`.
        pub(crate) fn set_random_latency(&self, seed: u64, max: Duration) {
            self.state.lock().unwrap().jitter = Some((seed, max));
        }

        /// Pause the next `kind` operation on a key starting with `prefix`
        /// before it takes effect.
        pub(crate) fn pause(&self, kind: OpKind, prefix: &str) -> PauseHandle {
            let (reached_tx, reached_rx) = oneshot::channel();
            let (release_tx, release_rx) = oneshot::channel();
            self.state.lock().unwrap().pauses.push(Pause {
                kind,
                prefix: prefix.to_string(),
                reached: reached_tx,
                release: release_rx,
            });
            PauseHandle {
                reached: Some(reached_rx),
                release: release_tx,
            }
        }

        /// Run `hook` at the start of every operation, before it takes effect.
        pub(crate) fn set_before_op(
            &self,
            hook: impl FnMut(&Op, &mut FakeObjects) + Send + 'static,
        ) {
            self.state.lock().unwrap().before_op = Some(Box::new(hook));
        }

        async fn begin(&self, kind: OpKind, key: &str) -> Result<()> {
            let op = Op {
                kind,
                key: key.to_string(),
            };
            let (pause, latency) = {
                let mut state = self.state.lock().unwrap();
                state.ops.push(op.clone());
                let pause = state
                    .pauses
                    .iter()
                    .position(|pause| pause.kind == kind && key.starts_with(&pause.prefix))
                    .map(|index| state.pauses.remove(index));
                let latency = state
                    .latencies
                    .iter()
                    .find(|(latency_kind, prefix, _)| {
                        *latency_kind == kind && key.starts_with(prefix.as_str())
                    })
                    .map(|(_, _, latency)| *latency)
                    .or_else(|| {
                        state.jitter.as_mut().map(|(seed, max)| {
                            let permille = split_mix(seed) % 1001;
                            max.mul_f64(permille as f64 / 1000.0)
                        })
                    });
                (pause, latency)
            };
            if let Some(pause) = pause {
                let _ = pause.reached.send(());
                let _ = pause.release.await;
            }
            if let Some(latency) = latency {
                tokio::time::sleep(latency).await;
            }
            let mut state = self.state.lock().unwrap();
            let now = self.now_locked(&state);
            state.contents.now = Some(now);
            if let Some(mut hook) = state.before_op.take() {
                hook(&op, &mut state.contents);
                state.before_op = Some(hook);
            }
            if state
                .failures
                .iter()
                .any(|(failure_kind, prefix)| *failure_kind == kind && key.starts_with(prefix))
            {
                bail!("injected {kind:?} failure for '{key}'");
            }
            Ok(())
        }
    }

    #[async_trait]
    impl LayerStore for FakeLayerStore {
        async fn list_objects(&self, prefix: &str) -> Result<Vec<StoredObject>> {
            self.begin(OpKind::List, prefix).await?;
            let state = self.state.lock().unwrap();
            Ok(state
                .contents
                .objects
                .iter()
                .filter(|(key, _)| key.starts_with(prefix))
                .map(|(key, (data, last_modified))| StoredObject {
                    key: key.clone(),
                    size: data.len() as u64,
                    last_modified: *last_modified,
                })
                .collect())
        }

        async fn get_object(&self, key: &str) -> Result<Option<Bytes>> {
            self.begin(OpKind::Get, key).await?;
            Ok(self.read(key))
        }

        async fn put_object(&self, key: &str, data: Bytes) -> Result<()> {
            self.begin(OpKind::Put, key).await?;
            self.insert(key, data);
            Ok(())
        }

        async fn stat_object(&self, key: &str) -> Result<Option<StoredObject>> {
            self.begin(OpKind::Stat, key).await?;
            let state = self.state.lock().unwrap();
            Ok(state
                .contents
                .objects
                .get(key)
                .map(|(data, last_modified)| StoredObject {
                    key: key.to_string(),
                    size: data.len() as u64,
                    last_modified: *last_modified,
                }))
        }

        async fn delete_object(&self, key: &str) -> Result<()> {
            self.begin(OpKind::Delete, key).await?;
            self.remove(key);
            Ok(())
        }
    }
}
