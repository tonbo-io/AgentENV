//! Per-process managed-layer leases.
//!
//! Every OSS-backed node process keeps one durable lease object,
//! `layer-gc/leases/{node}-{instance}.json`, listing every managed-layer
//! digest the process may read: layers of running and paused sandboxes (the
//! runtime set), layers held by active guards (restores in progress,
//! publications before their record is written, template builds through their
//! base snapshot), and a two-hour tail of digests that recently left either.
//!
//! Before a node trusts that `managed-layers/{d}` exists it makes `d` durable
//! in its lease, then lists GC deletion intents; when a live intent names
//! `d` it waits out the GC deletion window ([`INTENT_WAIT`]) and only then
//! lets the caller check `d`. The GC publishes its intent before its final
//! lease read, so either the GC sees the lease or the node sees the intent.
//! See `docs/src/internals/snapshot-layer-gc.md` for the protocol and proof.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

use super::layer_refs::{image_config_digests, raw_digest_tokens};
use super::layer_store::LayerStore;
use super::layout::{OssSnapshotArtifactLayout, LAYER_GC_INTENTS_PREFIX};
use crate::snapshot::repository::interfaces::SnapshotLayerRetention;

/// A lease protects its digests while its `LastModified` is at most this old
/// (compared with object-store time by the GC).
pub(crate) const LEASE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// A lease is rewritten at least this often even when nothing changed.
pub(crate) const LEASE_LIVENESS_REWRITE: Duration = Duration::from_secs(60 * 60);
/// A shrunk lease is rewritten after the live set stayed smaller this long.
pub(crate) const LEASE_SHRINK_DEBOUNCE: Duration = Duration::from_secs(5 * 60);
/// Refresher tick for liveness and shrink rewrites.
pub(crate) const LEASE_REFRESH_TICK: Duration = Duration::from_secs(60);
/// After this long without a successful write the owner rotates to a new
/// lease key, so a key the GC may have removed is never written again.
pub(crate) const LEASE_ROTATE_AFTER: Duration = Duration::from_secs(12 * 60 * 60);
/// Digests stay leased this long after their last holder released them.
pub(crate) const LEASE_TAIL: Duration = Duration::from_secs(2 * 60 * 60);
/// D_max: how long a node waits after seeing an intent that names a digest it
/// needs. Covers the GC's 45 s issue window, its 15 s request timeout twice,
/// and a 45 s margin.
pub(crate) const INTENT_WAIT: Duration = Duration::from_secs(120);
/// D_req: timeout of every lease, intent and GC delete request.
pub(crate) const STORE_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

const LEASE_STALE_ERROR_AGE: Duration = Duration::from_secs(60 * 60);
const INTENT_READ_CONCURRENCY: usize = 16;
const LAYER_CHECK_CONCURRENCY: usize = 16;
const LEASE_DOCUMENT_VERSION: u32 = 1;

/// Bound one object-store request by [`STORE_REQUEST_TIMEOUT`].
pub(crate) async fn with_timeout<T>(
    what: &str,
    future: impl Future<Output = Result<T>>,
) -> Result<T> {
    match tokio::time::timeout(STORE_REQUEST_TIMEOUT, future).await {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!(
            "{what} timed out after {}s",
            STORE_REQUEST_TIMEOUT.as_secs()
        )),
    }
}

/// Sleep for `duration` unless shutdown is requested first. Returns `true`
/// when shutdown was requested.
pub(crate) async fn sleep_or_shutdown(
    duration: Duration,
    shutdown: &mut watch::Receiver<bool>,
) -> bool {
    if *shutdown.borrow() {
        return true;
    }
    let sleep = tokio::time::sleep(duration);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            _ = &mut sleep => return false,
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return true;
                }
            }
        }
    }
}

/// Which node operation took the intent slow path (metric label).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProtectPath {
    Restore,
    Publish,
    Startup,
}

impl ProtectPath {
    fn as_str(self) -> &'static str {
        match self {
            Self::Restore => "restore",
            Self::Publish => "publish",
            Self::Startup => "startup",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WriteReason {
    Grow,
    Shrink,
    Liveness,
}

impl WriteReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Grow => "grow",
            Self::Shrink => "shrink",
            Self::Liveness => "liveness",
        }
    }
}

/// Serialized lease object. The GC reads it with the raw digest scan, so
/// later versions may add fields freely.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LeaseDocument {
    pub(crate) version: u32,
    pub(crate) node_id: String,
    pub(crate) instance: String,
    pub(crate) digests: Vec<String>,
}

struct RecordedConfig {
    digests: BTreeSet<String>,
    recorded_at: Instant,
}

struct LeaseState {
    generation: u32,
    generation_started: Instant,
    /// Digests read by running and paused sandboxes on this node.
    runtime: BTreeSet<String>,
    /// Digests of runtime image configs materialized by the resolver, so the
    /// runtime set does not depend on evictable cache files still existing.
    recorded_configs: HashMap<PathBuf, RecordedConfig>,
    /// Digests held by active guards, with their holder counts.
    guards: HashMap<String, usize>,
    /// Digests that left the runtime set or lost their last guard, with the
    /// time their tail ends.
    tail: HashMap<String, Instant>,
    /// Content of the last successful lease write under the current key.
    durable: Option<BTreeSet<String>>,
    /// Content of the lease write in flight, if any. Once it lands it
    /// replaces `durable`, so a digest only counts as durable when the
    /// in-flight write carries it too.
    inflight: Option<BTreeSet<String>>,
    /// Digests checked present after an intent check and continuously
    /// durable since.
    verified: HashSet<String>,
    last_success: Option<Instant>,
    shrink_pending_since: Option<Instant>,
}

impl LeaseState {
    fn new(now: Instant) -> Self {
        Self {
            generation: 0,
            generation_started: now,
            runtime: BTreeSet::new(),
            recorded_configs: HashMap::new(),
            guards: HashMap::new(),
            tail: HashMap::new(),
            durable: None,
            inflight: None,
            verified: HashSet::new(),
            last_success: None,
            shrink_pending_since: None,
        }
    }

    fn live_set(&mut self, now: Instant) -> BTreeSet<String> {
        self.tail.retain(|_, ends_at| *ends_at > now);
        let mut live = self.runtime.clone();
        live.extend(self.guards.keys().cloned());
        live.extend(self.tail.keys().cloned());
        live
    }

    fn durable_and_fresh(&self, digests: &BTreeSet<String>, now: Instant) -> bool {
        self.last_success
            .is_some_and(|last| now.duration_since(last) < LEASE_ROTATE_AFTER)
            && self
                .durable
                .as_ref()
                .is_some_and(|durable| digests.is_subset(durable))
            && self
                .inflight
                .as_ref()
                .is_none_or(|inflight| digests.is_subset(inflight))
    }

    fn start_tail(&mut self, digest: String, now: Instant) {
        self.tail.insert(digest, now + LEASE_TAIL);
    }
}

/// Lease registry of one node process.
pub(crate) struct LayerLeases {
    store: Arc<dyn LayerStore>,
    node_id: String,
    instance: String,
    owner: String,
    state: Mutex<LeaseState>,
    /// Serializes lease writes; callers queued behind a write re-check
    /// whether it already covered them (coalescing).
    writer: tokio::sync::Mutex<()>,
}

impl fmt::Debug for LayerLeases {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayerLeases")
            .field("owner", &self.owner)
            .finish_non_exhaustive()
    }
}

fn sanitize_key_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "node".to_string()
    } else {
        sanitized
    }
}

impl LayerLeases {
    pub(crate) fn new(store: Arc<dyn LayerStore>, node_id: &str) -> Arc<Self> {
        let instance = uuid::Uuid::now_v7().simple().to_string();
        let owner = format!("{}-{}", sanitize_key_component(node_id), instance);
        Arc::new(Self {
            store,
            node_id: node_id.to_string(),
            instance,
            owner,
            state: Mutex::new(LeaseState::new(Instant::now())),
            writer: tokio::sync::Mutex::new(()),
        })
    }

    fn lock_state(&self) -> MutexGuard<'_, LeaseState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// `{node}-{instance}`: identifies this process's lease, intents and run
    /// report.
    pub(crate) fn owner(&self) -> &str {
        &self.owner
    }

    pub(crate) fn store(&self) -> &Arc<dyn LayerStore> {
        &self.store
    }

    /// Key of the current lease object.
    pub(crate) fn lease_key(&self) -> String {
        let generation = self.lock_state().generation;
        OssSnapshotArtifactLayout::layer_lease_key(&self.owner, generation)
    }

    /// Lease `digests` until the returned guard is dropped (plus the tail).
    ///
    /// On return the digests are durable in this node's lease and either no
    /// live GC intent named them or the GC deletion window has passed, so the
    /// caller may now check that they exist. Unless the guard reports that
    /// the digests were already verified, the caller must check them and then
    /// call [`LayerLeaseGuard::mark_verified`]. Fails closed when the lease
    /// cannot be written or the intents cannot be read.
    pub(crate) async fn protect(
        self: &Arc<Self>,
        digests: BTreeSet<String>,
        path: ProtectPath,
    ) -> Result<LayerLeaseGuard> {
        let now = Instant::now();
        let fast = {
            let mut guard = self.lock_state();
            let state = &mut *guard;
            for digest in &digests {
                *state.guards.entry(digest.clone()).or_insert(0) += 1;
            }
            digests.iter().all(|digest| state.verified.contains(digest))
                && state.durable_and_fresh(&digests, now)
        };
        let mut guard = LayerLeaseGuard {
            leases: Arc::clone(self),
            digests,
            verified: false,
        };
        if fast || guard.digests.is_empty() {
            guard.verified = true;
            return Ok(guard);
        }
        self.ensure_durable(&guard.digests).await?;
        self.wait_for_intents(&guard.digests, path).await?;
        Ok(guard)
    }

    fn mark_verified(&self, digests: &BTreeSet<String>) {
        let mut guard = self.lock_state();
        let state = &mut *guard;
        if let Some(durable) = state.durable.as_ref() {
            state.verified.extend(
                digests
                    .iter()
                    .filter(|digest| durable.contains(*digest))
                    .cloned(),
            );
        }
    }

    fn release(&self, digests: &BTreeSet<String>) {
        let now = Instant::now();
        let mut guard = self.lock_state();
        let state = &mut *guard;
        for digest in digests {
            let released = match state.guards.get_mut(digest) {
                Some(count) => {
                    *count = count.saturating_sub(1);
                    *count == 0
                }
                None => false,
            };
            if released {
                state.guards.remove(digest);
                state.start_tail(digest.clone(), now);
            }
        }
    }

    /// Record which digests a materialized runtime image config names.
    pub(crate) fn record_image_config(&self, path: PathBuf, digests: BTreeSet<String>) {
        self.lock_state().recorded_configs.insert(
            path,
            RecordedConfig {
                digests,
                recorded_at: Instant::now(),
            },
        );
    }

    fn image_config_paths_digests(&self, paths: &[PathBuf]) -> BTreeSet<String> {
        let recorded = {
            let state = self.lock_state();
            paths
                .iter()
                .map(|path| {
                    state
                        .recorded_configs
                        .get(path)
                        .map(|recorded| recorded.digests.clone())
                })
                .collect::<Vec<_>>()
        };
        let mut digests = BTreeSet::new();
        for (path, recorded) in paths.iter().zip(recorded) {
            match image_config_digests(path) {
                Ok(found) => digests.extend(found),
                Err(error) => {
                    if recorded.is_none() {
                        metrics::counter!(
                            "agentenv_snapshot_layer_lease_unreadable_image_configs_total"
                        )
                        .increment(1);
                        debug!(
                            path = %path.display(),
                            error = %format!("{error:#}"),
                            "runtime image config unreadable and not recorded; its layers are not leased"
                        );
                    }
                }
            }
            if let Some(recorded) = recorded {
                digests.extend(recorded);
            }
        }
        digests
    }

    /// Replace the runtime set with the digests named by the image configs
    /// of every running and paused sandbox. Growth is written immediately;
    /// shrinkage is written by the refresher after a debounce.
    pub(crate) async fn set_runtime_image_configs(&self, paths: Vec<PathBuf>) {
        let digests = self.image_config_paths_digests(&paths);
        let now = Instant::now();
        let grew = {
            let mut guard = self.lock_state();
            let state = &mut *guard;
            let current = paths.iter().collect::<HashSet<_>>();
            state.recorded_configs.retain(|path, recorded| {
                current.contains(path) || now.duration_since(recorded.recorded_at) < LEASE_TAIL
            });
            let removed = state
                .runtime
                .difference(&digests)
                .cloned()
                .collect::<Vec<_>>();
            for digest in removed {
                state.start_tail(digest, now);
            }
            state.runtime = digests.clone();
            !state.durable_and_fresh(&digests, now)
        };
        if grew {
            if let Err(error) = self.ensure_durable(&digests).await {
                warn!(
                    error = %format!("{error:#}"),
                    "failed to write grown managed-layer lease; the refresher retries"
                );
            }
        }
    }

    /// Lease the layers of paused sandboxes restored from local persistence,
    /// before the node serves requests.
    pub(crate) async fn protect_persisted_image_configs(&self, paths: Vec<PathBuf>) -> Result<()> {
        let digests = self.image_config_paths_digests(&paths);
        if digests.is_empty() {
            return Ok(());
        }
        self.lock_state().runtime.extend(digests.iter().cloned());
        self.ensure_durable(&digests)
            .await
            .context("lease persisted paused sandbox layers")?;
        self.wait_for_intents(&digests, ProtectPath::Startup).await
    }

    async fn ensure_durable(&self, required: &BTreeSet<String>) -> Result<()> {
        if self
            .lock_state()
            .durable_and_fresh(required, Instant::now())
        {
            return Ok(());
        }
        let _writer = self.writer.lock().await;
        if self
            .lock_state()
            .durable_and_fresh(required, Instant::now())
        {
            return Ok(());
        }
        self.write_locked(WriteReason::Grow).await
    }

    /// Write the lease now and return its key. Used by the GC to obtain the
    /// object-store time at the start of a pass.
    pub(crate) async fn refresh_now(&self) -> Result<String> {
        let _writer = self.writer.lock().await;
        self.write_locked(WriteReason::Liveness).await?;
        Ok(self.lease_key())
    }

    /// One refresher tick: write when the lease was never written, misses a
    /// live digest, stayed larger than the live set past the debounce, or is
    /// older than the liveness interval.
    pub(crate) async fn maintain(&self) {
        let now = Instant::now();
        let reason = {
            let mut guard = self.lock_state();
            let state = &mut *guard;
            let live = state.live_set(now);
            let age = state.last_success.map(|last| now.duration_since(last));
            if let Some(age) = age {
                metrics::gauge!("agentenv_snapshot_layer_lease_age_seconds").set(age.as_secs_f64());
            }
            match state.durable.as_ref() {
                None => Some(WriteReason::Liveness),
                Some(durable) if !live.is_subset(durable) => Some(WriteReason::Grow),
                Some(durable) if live != *durable => {
                    let since = *state.shrink_pending_since.get_or_insert(now);
                    (now.duration_since(since) >= LEASE_SHRINK_DEBOUNCE)
                        .then_some(WriteReason::Shrink)
                        .or_else(|| {
                            age.is_none_or(|age| age >= LEASE_LIVENESS_REWRITE)
                                .then_some(WriteReason::Liveness)
                        })
                }
                Some(_) => age
                    .is_none_or(|age| age >= LEASE_LIVENESS_REWRITE)
                    .then_some(WriteReason::Liveness),
            }
        };
        if let Some(reason) = reason {
            let _writer = self.writer.lock().await;
            if let Err(error) = self.write_locked(reason).await {
                debug!(error = %format!("{error:#}"), "managed-layer lease refresh failed");
            }
        }
    }

    /// Keep the lease fresh until shutdown.
    pub(crate) async fn run_refresher(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        info!(lease = %self.lease_key(), "managed-layer lease refresher started");
        loop {
            self.maintain().await;
            if sleep_or_shutdown(LEASE_REFRESH_TICK, &mut shutdown).await {
                break;
            }
        }
        debug!("managed-layer lease refresher stopped");
    }

    /// Write the current live set. The caller holds `self.writer`.
    async fn write_locked(&self, reason: WriteReason) -> Result<()> {
        let now = Instant::now();
        let (key, digests, last_success) = {
            let mut guard = self.lock_state();
            let state = &mut *guard;
            let since = state.last_success.unwrap_or(state.generation_started);
            if now.duration_since(since) >= LEASE_ROTATE_AFTER {
                state.generation += 1;
                state.generation_started = now;
                state.durable = None;
                state.verified.clear();
                warn!(
                    owner = %self.owner,
                    generation = state.generation,
                    "managed-layer lease not written for half its TTL; rotating to a new lease key"
                );
            }
            let digests = state.live_set(now);
            state.inflight = Some(digests.clone());
            (
                OssSnapshotArtifactLayout::layer_lease_key(&self.owner, state.generation),
                digests,
                state.last_success,
            )
        };
        let document = LeaseDocument {
            version: LEASE_DOCUMENT_VERSION,
            node_id: self.node_id.clone(),
            instance: self.instance.clone(),
            digests: digests.iter().cloned().collect(),
        };
        let body = serde_json::to_vec(&document).context("serialize managed-layer lease")?;
        let result = with_timeout(
            "managed-layer lease write",
            self.store.put_object(&key, body.into()),
        )
        .await
        .with_context(|| format!("write managed-layer lease '{key}'"));
        let outcome = if result.is_ok() { "ok" } else { "error" };
        metrics::counter!(
            "agentenv_snapshot_layer_lease_writes_total",
            "reason" => reason.as_str(),
            "outcome" => outcome,
        )
        .increment(1);
        let mut guard = self.lock_state();
        let state = &mut *guard;
        state.inflight = None;
        match result {
            Ok(()) => {
                state.verified.retain(|digest| digests.contains(digest));
                metrics::gauge!("agentenv_snapshot_layer_lease_digests").set(digests.len() as f64);
                metrics::gauge!("agentenv_snapshot_layer_lease_age_seconds").set(0.0);
                debug!(
                    lease = %key,
                    reason = reason.as_str(),
                    digests = digests.len(),
                    "managed-layer lease written"
                );
                state.durable = Some(digests);
                state.last_success = Some(now);
                state.shrink_pending_since = None;
                Ok(())
            }
            Err(error) => {
                // A failed (for example timed-out) write may still land later
                // and replace the previous content, so nothing counts as
                // durable or verified until the next successful write.
                state.durable = None;
                state.verified.clear();
                drop(guard);
                let age = last_success.map(|last| now.duration_since(last));
                if age.is_none_or(|age| age >= LEASE_STALE_ERROR_AGE) {
                    error!(
                        lease = %key,
                        reason = reason.as_str(),
                        age_secs = age.map(|age| age.as_secs()),
                        error = %format!("{error:#}"),
                        "managed-layer lease write failed and the lease is stale"
                    );
                } else {
                    warn!(
                        lease = %key,
                        reason = reason.as_str(),
                        error = %format!("{error:#}"),
                        "managed-layer lease write failed"
                    );
                }
                Err(error)
            }
        }
    }

    /// List live GC intents and wait out the deletion window when one of
    /// them names any of `digests`.
    async fn wait_for_intents(&self, digests: &BTreeSet<String>, path: ProtectPath) -> Result<()> {
        let intents = with_timeout(
            "list managed-layer GC intents",
            self.store.list_objects(LAYER_GC_INTENTS_PREFIX),
        )
        .await
        .context("list managed-layer GC intents")?;
        let store = &self.store;
        let named = stream::iter(intents)
            .map(|intent| async move {
                let body = with_timeout(
                    "read managed-layer GC intent",
                    store.get_object(&intent.key),
                )
                .await
                .with_context(|| format!("read managed-layer GC intent '{}'", intent.key))?;
                // A vanished intent means its pass already finished deleting;
                // the caller's existence check observes the outcome.
                Ok::<bool, anyhow::Error>(body.is_some_and(|body| {
                    let tokens = raw_digest_tokens(&body);
                    digests.iter().any(|digest| tokens.contains(digest))
                }))
            })
            .buffer_unordered(INTENT_READ_CONCURRENCY)
            .try_collect::<Vec<bool>>()
            .await?;
        if named.into_iter().any(|named| named) {
            metrics::counter!(
                "agentenv_snapshot_layer_intent_waits_total",
                "path" => path.as_str(),
            )
            .increment(1);
            info!(
                path = path.as_str(),
                wait_secs = INTENT_WAIT.as_secs(),
                "managed layers are named by a GC deletion intent; waiting out its deletion window before checking them"
            );
            tokio::time::sleep(INTENT_WAIT).await;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn live_digests(&self) -> BTreeSet<String> {
        self.lock_state().live_set(Instant::now())
    }

    #[cfg(test)]
    pub(crate) fn instance(&self) -> &str {
        &self.instance
    }
}

/// Why [`protect_and_check`] refused to trust a set of managed layers.
#[derive(Debug)]
pub(crate) enum LayerCheckError {
    /// The lease could not be made durable or the intents could not be read.
    Protect(anyhow::Error),
    /// The existence check itself failed.
    Check {
        digest: String,
        error: anyhow::Error,
    },
    /// The layer does not exist (it was collected or never uploaded).
    Missing { digest: String },
}

impl fmt::Display for LayerCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protect(error) => write!(f, "lease managed layers: {error:#}"),
            Self::Check { digest, error } => {
                write!(f, "check managed layer '{digest}': {error:#}")
            }
            Self::Missing { digest } => write!(f, "managed layer '{digest}' is missing"),
        }
    }
}

/// Lease `digests`, then check that every one of them exists under
/// `managed-layers/` (always when `always_check`, otherwise only when they
/// were not already verified while continuously leased). This is rule N1 of
/// the GC protocol for callers that are about to trust the layers.
pub(crate) async fn protect_and_check(
    leases: &Arc<LayerLeases>,
    digests: BTreeSet<String>,
    path: ProtectPath,
    always_check: bool,
) -> std::result::Result<LayerLeaseGuard, LayerCheckError> {
    let mut guard = leases
        .protect(digests, path)
        .await
        .map_err(LayerCheckError::Protect)?;
    if always_check || guard.needs_verification() {
        let store = leases.store();
        let results = stream::iter(guard.digests().iter().cloned())
            .map(|digest| async move {
                let key = OssSnapshotArtifactLayout::managed_layer_key(&digest);
                let present = with_timeout("check managed layer", store.stat_object(&key)).await;
                (digest, present)
            })
            .buffer_unordered(LAYER_CHECK_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        for (digest, present) in results {
            match present {
                Ok(Some(_)) => {}
                Ok(None) => return Err(LayerCheckError::Missing { digest }),
                Err(error) => return Err(LayerCheckError::Check { digest, error }),
            }
        }
        guard.mark_verified();
    }
    Ok(guard)
}

/// Holds digests in the node lease until dropped; they then stay in the
/// lease for [`LEASE_TAIL`].
pub(crate) struct LayerLeaseGuard {
    leases: Arc<LayerLeases>,
    digests: BTreeSet<String>,
    verified: bool,
}

impl LayerLeaseGuard {
    /// Whether the caller must check the digests exist before trusting them.
    pub(crate) fn needs_verification(&self) -> bool {
        !self.verified
    }

    /// Record that the caller checked the digests exist. Later protects of
    /// the same digests skip the lease write and intent check while they
    /// stay continuously leased.
    pub(crate) fn mark_verified(&mut self) {
        self.verified = true;
        self.leases.mark_verified(&self.digests);
    }

    pub(crate) fn digests(&self) -> &BTreeSet<String> {
        &self.digests
    }
}

impl fmt::Debug for LayerLeaseGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayerLeaseGuard")
            .field("digests", &self.digests.len())
            .field("verified", &self.verified)
            .finish()
    }
}

impl Drop for LayerLeaseGuard {
    fn drop(&mut self) {
        self.leases.release(&self.digests);
    }
}

#[async_trait::async_trait]
impl SnapshotLayerRetention for LayerLeases {
    async fn protect_persisted_image_configs(&self, paths: Vec<PathBuf>) -> Result<()> {
        LayerLeases::protect_persisted_image_configs(self, paths).await
    }

    async fn set_runtime_image_configs(&self, paths: Vec<PathBuf>) {
        LayerLeases::set_runtime_image_configs(self, paths).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::layer_refs::test_digest;
    use super::super::layer_store::fake::{FakeLayerStore, OpKind};
    use super::super::layout::LAYER_GC_LEASES_PREFIX as LAYER_GC_LEASES_PREFIX_FOR_TESTS;
    use super::*;

    fn digests(indexes: &[usize]) -> BTreeSet<String> {
        indexes.iter().copied().map(test_digest).collect()
    }

    fn leases(store: &Arc<FakeLayerStore>) -> Arc<LayerLeases> {
        LayerLeases::new(Arc::clone(store) as Arc<dyn LayerStore>, "node/a")
    }

    fn lease_body(store: &FakeLayerStore, leases: &LayerLeases) -> BTreeSet<String> {
        let body = store.read(&leases.lease_key()).expect("lease written");
        let document: LeaseDocument = serde_json::from_slice(&body).expect("parse lease");
        document.digests.into_iter().collect()
    }

    fn intent_naming(store: &FakeLayerStore, name: &str, named: &BTreeSet<String>) {
        store.insert(
            &format!("{LAYER_GC_INTENTS_PREFIX}{name}.json"),
            serde_json::to_vec(&serde_json::json!({ "digests": named })).unwrap(),
        );
    }

    #[tokio::test(start_paused = true)]
    async fn protect_writes_lease_before_returning_and_coalesces_concurrent_callers() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases(&store);
        assert!(leases.lease_key().starts_with("layer-gc/leases/node_a-"));
        let pause = store.pause(OpKind::Put, LAYER_GC_LEASES_PREFIX_FOR_TESTS);

        let first = tokio::spawn({
            let leases = Arc::clone(&leases);
            async move { leases.protect(digests(&[1, 2]), ProtectPath::Restore).await }
        });
        let mut pause = pause;
        pause.reached().await;
        let second = tokio::spawn({
            let leases = Arc::clone(&leases);
            async move { leases.protect(digests(&[1]), ProtectPath::Restore).await }
        });
        tokio::task::yield_now().await;
        assert!(!first.is_finished());
        assert!(!second.is_finished());
        pause.release();

        let first = first.await.unwrap().expect("first protect");
        let second = second.await.unwrap().expect("second protect");
        let puts = store
            .ops()
            .into_iter()
            .filter(|op| op.kind == OpKind::Put)
            .count();
        assert_eq!(puts, 1, "concurrent protects coalesce into one lease write");
        assert_eq!(lease_body(&store, &leases), digests(&[1, 2]));
        assert!(first.needs_verification());
        assert!(second.needs_verification());
    }

    #[tokio::test(start_paused = true)]
    async fn protect_lists_intents_after_the_lease_put_and_waits_only_for_named_digests() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases(&store);
        intent_naming(&store, "other-pass", &digests(&[9]));

        let started = Instant::now();
        let _guard = leases
            .protect(digests(&[1]), ProtectPath::Publish)
            .await
            .expect("protect");
        assert!(
            started.elapsed() < INTENT_WAIT,
            "unnamed digests do not wait"
        );
        let ops = store.ops();
        let put = ops
            .iter()
            .position(|op| op.kind == OpKind::Put)
            .expect("lease put");
        let list = ops
            .iter()
            .position(|op| op.kind == OpKind::List && op.key == LAYER_GC_INTENTS_PREFIX)
            .expect("intent list");
        assert!(put < list, "lease is durable before intents are listed");

        intent_naming(&store, "named-pass", &digests(&[2]));
        let started = Instant::now();
        let _guard = leases
            .protect(digests(&[2]), ProtectPath::Publish)
            .await
            .expect("protect");
        assert!(started.elapsed() >= INTENT_WAIT, "named digests wait D_max");
    }

    #[tokio::test(start_paused = true)]
    async fn already_checked_digests_skip_put_and_list() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases(&store);
        let mut guard = leases
            .protect(digests(&[1]), ProtectPath::Restore)
            .await
            .expect("protect");
        guard.mark_verified();
        drop(guard);

        store.clear_ops();
        let warm = leases
            .protect(digests(&[1]), ProtectPath::Restore)
            .await
            .expect("warm protect");
        assert!(!warm.needs_verification());
        assert!(store.ops().is_empty(), "warm protect does no store work");
        drop(warm);

        // Once the tail expires and a rewrite drops the digest, the fast
        // path is gone.
        tokio::time::advance(LEASE_TAIL + Duration::from_secs(1)).await;
        leases.maintain().await;
        assert!(!lease_body(&store, &leases).contains(&test_digest(1)));
        store.clear_ops();
        let cold = leases
            .protect(digests(&[1]), ProtectPath::Restore)
            .await
            .expect("cold protect");
        assert!(cold.needs_verification());
        assert!(store.ops().iter().any(|op| op.kind == OpKind::Put));
    }

    #[tokio::test(start_paused = true)]
    async fn dropped_guard_and_removed_runtime_digests_stay_in_tail_for_two_hours() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases(&store);
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("image.json");
        std::fs::write(
            &config,
            serde_json::to_vec(&serde_json::json!({
                "lowers": [{ "digest": test_digest(5), "size": 1 }]
            }))
            .unwrap(),
        )
        .unwrap();
        leases.set_runtime_image_configs(vec![config.clone()]).await;
        let guard = leases
            .protect(digests(&[1]), ProtectPath::Restore)
            .await
            .expect("protect");
        assert_eq!(lease_body(&store, &leases), digests(&[1, 5]));

        drop(guard);
        leases.set_runtime_image_configs(Vec::new()).await;
        assert_eq!(leases.live_digests(), digests(&[1, 5]));

        tokio::time::advance(LEASE_TAIL - Duration::from_secs(60)).await;
        leases.maintain().await;
        assert_eq!(lease_body(&store, &leases), digests(&[1, 5]));

        tokio::time::advance(Duration::from_secs(120)).await;
        assert!(leases.live_digests().is_empty());
        leases.maintain().await;
        tokio::time::advance(LEASE_SHRINK_DEBOUNCE).await;
        leases.maintain().await;
        assert!(lease_body(&store, &leases).is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn resolver_recorded_image_config_digests_survive_file_eviction() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases(&store);
        let temp = tempfile::tempdir().unwrap();
        let evicted = temp.path().join("runtime/memory/image.json");
        leases.record_image_config(evicted.clone(), digests(&[7]));
        leases.set_runtime_image_configs(vec![evicted]).await;
        assert_eq!(lease_body(&store, &leases), digests(&[7]));
    }

    #[tokio::test(start_paused = true)]
    async fn lease_put_failure_fails_protect_closed() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases(&store);
        store.fail(OpKind::Put, LAYER_GC_LEASES_PREFIX_FOR_TESTS);
        let error = leases
            .protect(digests(&[1]), ProtectPath::Restore)
            .await
            .expect_err("lease write failure fails protect");
        assert!(format!("{error:#}").contains("injected Put failure"));
        assert!(
            !store.ops().iter().any(|op| op.kind == OpKind::List),
            "no intent check without a durable lease"
        );

        store.clear_failures();
        store.fail(OpKind::List, LAYER_GC_INTENTS_PREFIX);
        leases
            .protect(digests(&[1]), ProtectPath::Restore)
            .await
            .expect_err("intent list failure fails protect");
    }

    #[tokio::test(start_paused = true)]
    async fn lease_key_rotates_after_half_ttl_without_a_successful_write() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases(&store);
        leases.maintain().await;
        let first_key = leases.lease_key();
        assert!(store.contains(&first_key));

        store.fail(OpKind::Put, LAYER_GC_LEASES_PREFIX_FOR_TESTS);
        tokio::time::advance(LEASE_ROTATE_AFTER - Duration::from_secs(60)).await;
        leases.maintain().await;
        assert_eq!(
            leases.lease_key(),
            first_key,
            "no rotation before half the TTL"
        );

        tokio::time::advance(Duration::from_secs(120)).await;
        leases.maintain().await;
        let rotated = leases.lease_key();
        assert_ne!(rotated, first_key);
        assert!(rotated.ends_with("-g1.json"));
        assert!(rotated.contains(leases.instance()));

        store.clear_failures();
        leases.maintain().await;
        assert!(store.contains(&rotated));
    }
}
