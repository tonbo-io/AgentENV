//! Per-process managed-layer leases.
//!
//! A node process whose `[snapshot.layer_gc].mode` is `report` or `delete`
//! keeps one lease object, `layer-gc/leases/{node}-{instance}[-gN].json`,
//! listing every managed-layer digest the process may read: layers of running
//! and paused sandboxes (the runtime set), layers held by active guards
//! (restores in progress, publications before their record is written,
//! template builds through their base snapshot), and a two-hour tail of
//! digests that recently left either. In `off` mode this module is not
//! constructed at all.
//!
//! Serving paths call [`LayerLeases::hold`], which only updates memory and
//! nudges the background refresher; it never waits or fails. In `delete`
//! mode, and only there, a path that is about to trust a managed layer then
//! calls [`LayerLeases::gate`] (rule N1): the digests become durable in this
//! node's lease, the node lists GC deletion intents, waits out the deletion
//! window when a live intent names a digest it needs ([`INTENT_WAIT`]), and
//! checks that each digest exists. The GC publishes its intent before its
//! final lease read, so either the GC sees the lease or the node sees the
//! intent. See `docs/src/internals/snapshot-layer-gc.md` for the protocol and
//! proof.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Notify};
use tracing::{debug, error, info, warn};

use super::boot_clock::BootInstant;
use super::layer_refs::{
    image_config_digests, image_config_managed_layer_digests, raw_digest_tokens,
};
use super::layer_store::LayerStore;
use super::layout::{OssSnapshotArtifactLayout, LAYER_GC_INTENTS_PREFIX};
use crate::cfg::SnapshotLayerGcMode;
use crate::snapshot::repository::interfaces::{PersistedLayerCheck, SnapshotLayerRetention};

/// A lease protects its digests while its `LastModified` is at most this old
/// (compared with object-store time by the GC).
pub(crate) const LEASE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// A lease is rewritten at least this often even when nothing changed.
pub(crate) const LEASE_LIVENESS_REWRITE: Duration = Duration::from_secs(60 * 60);
/// A shrunk lease is rewritten after the live set stayed smaller this long.
pub(crate) const LEASE_SHRINK_DEBOUNCE: Duration = Duration::from_secs(5 * 60);
/// Refresher tick for liveness and shrink rewrites.
pub(crate) const LEASE_REFRESH_TICK: Duration = Duration::from_secs(60);
/// The lease content counts as durable for the gate's fast path only while
/// the last successful write is younger than this on the boot clock (half
/// the TTL). The refresher rewrites the lease at least hourly, so this only
/// bites when writes keep failing.
pub(crate) const LEASE_FRESH_FOR: Duration = Duration::from_secs(12 * 60 * 60);
/// Digests stay leased this long after their last holder released them.
pub(crate) const LEASE_TAIL: Duration = Duration::from_secs(2 * 60 * 60);
/// D_max: how long a node waits after seeing an intent that names a digest it
/// needs. Covers the GC's 45 s issue window, a 15 s DELETE timeout and 60 s
/// for a timed-out DELETE to land (see the const assertions in `layer_gc`).
pub(crate) const INTENT_WAIT: Duration = Duration::from_secs(120);
/// D_req: timeout of every lease, intent, existence-check and GC request.
pub(crate) const STORE_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

const LEASE_STALE_ERROR_AGE: Duration = Duration::from_secs(60 * 60);
/// How often one unreadable, unrecorded runtime image config is logged.
const UNREADABLE_CONFIG_WARN_INTERVAL: Duration = Duration::from_secs(60 * 60);
const INTENT_READ_CONCURRENCY: usize = 16;
const LAYER_CHECK_CONCURRENCY: usize = 16;
/// Version 2 declares the writer's mode; the GC treats a live lease that does
/// not declare `delete` as a reader that does not gate (rule G6).
const LEASE_DOCUMENT_VERSION: u32 = 2;

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
/// when shutdown was requested. A dropped sender can no longer request
/// shutdown, so the sleep then simply runs out.
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
                match changed {
                    Ok(()) if *shutdown.borrow() => return true,
                    Ok(()) => {}
                    Err(_) => {
                        (&mut sleep).await;
                        return false;
                    }
                }
            }
        }
    }
}

/// Which node operation gated managed layers (metric and log label).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GatePath {
    Restore,
    Publish,
    Startup,
    Resume,
}

impl GatePath {
    fn as_str(self) -> &'static str {
        match self {
            Self::Restore => "restore",
            Self::Publish => "publish",
            Self::Startup => "startup",
            Self::Resume => "resume",
        }
    }
}

/// How a successful gate went (metric and log label).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GateOutcome {
    /// Every digest was verified and freshly leased: no request.
    Fast,
    /// Lease made durable, intents listed, unverified digests checked.
    Cold,
    /// As `Cold`, plus the deletion-window wait for a named intent.
    IntentWait,
}

impl GateOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Cold => "cold",
            Self::IntentWait => "intent_wait",
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

/// Serialized lease object. The GC reads digests with the raw digest scan and
/// the `mode` field for rule G6, so later versions may add fields freely.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LeaseDocument {
    pub(crate) version: u32,
    pub(crate) node_id: String,
    pub(crate) instance: String,
    /// `report` or `delete`: whether this process gates before trusting a
    /// managed layer.
    pub(crate) mode: String,
    pub(crate) digests: Vec<String>,
}

struct RecordedConfig {
    digests: BTreeSet<String>,
    recorded_at: BootInstant,
}

struct LeaseState {
    /// Lease key generation. A failed write may still land later with older
    /// content, so every failure moves to a new key: a late landing can then
    /// only touch an abandoned key, which merely over-protects until it
    /// expires.
    generation: u32,
    /// Digests read by running and paused sandboxes on this node.
    runtime: BTreeSet<String>,
    /// Digests of runtime image configs materialized by the resolver, so the
    /// runtime set does not depend on evictable cache files still existing.
    recorded_configs: HashMap<PathBuf, RecordedConfig>,
    /// Digests held by active guards, with their holder counts.
    guards: HashMap<String, usize>,
    /// Digests that left the runtime set or lost their last guard, with the
    /// boot-clock time they did.
    tail: HashMap<String, BootInstant>,
    /// Digests some lease key of this process is guaranteed to hold until
    /// `last_success + LEASE_TTL`: the content of the last successful write,
    /// narrowed after each failed write to what any late landing of that
    /// write also carries.
    durable: Option<BTreeSet<String>>,
    /// Content of the lease write in flight, if any. Once it lands it
    /// replaces `durable`, so a digest only counts as durable when the
    /// in-flight write carries it too.
    inflight: Option<BTreeSet<String>>,
    /// Digests checked present after an intent check (rule N1) and durable
    /// since.
    verified: HashSet<String>,
    last_success: Option<BootInstant>,
    shrink_pending_since: Option<BootInstant>,
    /// When each unreadable, unrecorded runtime image config was last logged.
    unreadable_warned: HashMap<PathBuf, BootInstant>,
}

impl LeaseState {
    fn new() -> Self {
        Self {
            generation: 0,
            runtime: BTreeSet::new(),
            recorded_configs: HashMap::new(),
            guards: HashMap::new(),
            tail: HashMap::new(),
            durable: None,
            inflight: None,
            verified: HashSet::new(),
            last_success: None,
            shrink_pending_since: None,
            unreadable_warned: HashMap::new(),
        }
    }

    fn live_set(&mut self, now: BootInstant) -> BTreeSet<String> {
        self.tail
            .retain(|_, released| now.duration_since(*released) < LEASE_TAIL);
        let mut live = self.runtime.clone();
        live.extend(self.guards.keys().cloned());
        live.extend(self.tail.keys().cloned());
        live
    }

    fn fresh(&self, now: BootInstant) -> bool {
        self.last_success
            .is_some_and(|last| now.duration_since(last) < LEASE_FRESH_FOR)
    }

    fn durable_and_fresh(&self, digests: &BTreeSet<String>, now: BootInstant) -> bool {
        self.fresh(now)
            && self
                .durable
                .as_ref()
                .is_some_and(|durable| digests.is_subset(durable))
            && self
                .inflight
                .as_ref()
                .is_none_or(|inflight| digests.is_subset(inflight))
    }

    /// Whether `digest` may be trusted without a request: verified by an
    /// earlier gate and still durable in a fresh lease.
    fn trusted(&self, digest: &str, now: BootInstant) -> bool {
        self.fresh(now)
            && self.verified.contains(digest)
            && self
                .durable
                .as_ref()
                .is_some_and(|durable| durable.contains(digest))
            && self
                .inflight
                .as_ref()
                .is_none_or(|inflight| inflight.contains(digest))
    }

    fn start_tail(&mut self, digest: String, now: BootInstant) {
        self.tail.insert(digest, now);
    }
}

/// Lease registry of one node process in `report` or `delete` mode.
pub(crate) struct LayerLeases {
    store: Arc<dyn LayerStore>,
    node_id: String,
    instance: String,
    owner: String,
    /// `report` or `delete`; only `delete` gates.
    mode: SnapshotLayerGcMode,
    /// repoBlobUrl of this repository's `managed-layers/`, to tell remote
    /// managed lowers apart from local and foreign ones.
    managed_layers_repo_blob_url: String,
    state: Mutex<LeaseState>,
    /// Serializes lease writes; callers queued behind a write re-check
    /// whether it already covered them (coalescing).
    writer: tokio::sync::Mutex<()>,
    /// Wakes the refresher when the live set grew.
    refresh_notify: Notify,
}

impl fmt::Debug for LayerLeases {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayerLeases")
            .field("owner", &self.owner)
            .field("mode", &self.mode)
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
    /// Leases of a `report` or `delete` mode process. `off` takes no part in
    /// the protocol and must not construct leases.
    pub(crate) fn new(
        store: Arc<dyn LayerStore>,
        node_id: &str,
        managed_layers_repo_blob_url: &str,
        mode: SnapshotLayerGcMode,
    ) -> Result<Arc<Self>> {
        anyhow::ensure!(
            mode != SnapshotLayerGcMode::Off,
            "managed-layer leases are not used in off mode"
        );
        let instance = uuid::Uuid::now_v7().simple().to_string();
        let owner = format!("{}-{}", sanitize_key_component(node_id), instance);
        Ok(Arc::new(Self {
            store,
            node_id: node_id.to_string(),
            instance,
            owner,
            mode,
            managed_layers_repo_blob_url: managed_layers_repo_blob_url.to_string(),
            state: Mutex::new(LeaseState::new()),
            writer: tokio::sync::Mutex::new(()),
            refresh_notify: Notify::new(),
        }))
    }

    fn lock_state(&self) -> MutexGuard<'_, LeaseState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether serving paths must [`gate`](Self::gate) before trusting a
    /// managed layer (rule N1). Only `delete` mode gates.
    pub(crate) fn is_gating(&self) -> bool {
        self.mode == SnapshotLayerGcMode::Delete
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
    /// Synchronous and infallible: it registers the digests in memory and
    /// wakes the refresher when they are not yet durable. It never waits for
    /// a lease write. In `delete` mode a caller that is about to trust the
    /// layers must also [`gate`](Self::gate) them.
    pub(crate) fn hold(self: &Arc<Self>, digests: BTreeSet<String>) -> LayerLeaseGuard {
        let grew = {
            let mut guard = self.lock_state();
            let state = &mut *guard;
            for digest in &digests {
                *state.guards.entry(digest.clone()).or_insert(0) += 1;
            }
            !state.durable_and_fresh(&digests, BootInstant::now())
        };
        if grew && !digests.is_empty() {
            self.refresh_notify.notify_one();
        }
        LayerLeaseGuard {
            leases: Arc::clone(self),
            digests,
        }
    }

    /// Rule N1 (`delete` mode): before trusting `digests`, make them durable
    /// in this node's lease, list GC intents, wait out the deletion window
    /// when a live intent names an unverified digest, and check that every
    /// unverified digest exists. Digests verified earlier and durable in a
    /// fresh lease since need no request. The caller must hold the digests
    /// (see [`hold`](Self::hold)) for as long as it relies on them. Fails
    /// closed: a missing layer is [`LayerCheckError::Missing`]; any lease,
    /// intent or check failure is an error too.
    pub(crate) async fn gate(
        &self,
        digests: &BTreeSet<String>,
        path: GatePath,
    ) -> std::result::Result<GateOutcome, LayerCheckError> {
        let (outcome, missing) = self.gate_reporting_missing(digests, path).await?;
        match missing.into_iter().next() {
            Some(digest) => Err(LayerCheckError::Missing { digest }),
            None => Ok(outcome),
        }
    }

    /// [`gate`](Self::gate), but missing digests are returned instead of
    /// failing, so a caller can tell which of several subjects lost a layer.
    async fn gate_reporting_missing(
        &self,
        digests: &BTreeSet<String>,
        path: GatePath,
    ) -> std::result::Result<(GateOutcome, BTreeSet<String>), LayerCheckError> {
        let started = std::time::Instant::now();
        let result = self.gate_inner(digests, path).await;
        let (outcome, failure) = match &result {
            Ok((outcome, missing)) if missing.is_empty() => (outcome.as_str(), None),
            Ok(_) => ("failed", Some("missing")),
            Err(error) => ("failed", Some(error.reason())),
        };
        metrics::histogram!(
            "agentenv_snapshot_layer_gate_seconds",
            "path" => path.as_str(),
            "outcome" => outcome,
        )
        .record(started.elapsed().as_secs_f64());
        if let Some(reason) = failure {
            metrics::counter!(
                "agentenv_snapshot_layer_gate_failures_total",
                "path" => path.as_str(),
                "reason" => reason,
            )
            .increment(1);
        }
        result
    }

    async fn gate_inner(
        &self,
        digests: &BTreeSet<String>,
        path: GatePath,
    ) -> std::result::Result<(GateOutcome, BTreeSet<String>), LayerCheckError> {
        let unverified = {
            let state = self.lock_state();
            let now = BootInstant::now();
            digests
                .iter()
                .filter(|digest| !state.trusted(digest, now))
                .cloned()
                .collect::<BTreeSet<_>>()
        };
        if unverified.is_empty() {
            return Ok((GateOutcome::Fast, BTreeSet::new()));
        }
        self.ensure_durable(digests)
            .await
            .map_err(LayerCheckError::Protect)?;
        let waited = self
            .wait_for_intents(&unverified, path)
            .await
            .map_err(LayerCheckError::Protect)?;
        let missing = check_present(&self.store, &unverified).await?;
        let present = unverified
            .difference(&missing)
            .cloned()
            .collect::<BTreeSet<_>>();
        self.mark_verified(&present);
        let outcome = if waited {
            GateOutcome::IntentWait
        } else {
            GateOutcome::Cold
        };
        Ok((outcome, missing))
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
        let now = BootInstant::now();
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
                recorded_at: BootInstant::now(),
            },
        );
    }

    /// Whether an unreadable, unrecorded image config should be logged now
    /// (at most once per path per [`UNREADABLE_CONFIG_WARN_INTERVAL`]).
    fn should_warn_unreadable(&self, path: &Path) -> bool {
        let now = BootInstant::now();
        let mut state = self.lock_state();
        state.unreadable_warned.retain(|_, warned_at| {
            now.duration_since(*warned_at) < UNREADABLE_CONFIG_WARN_INTERVAL
        });
        if state.unreadable_warned.contains_key(path) {
            return false;
        }
        state.unreadable_warned.insert(path.to_path_buf(), now);
        true
    }

    /// Digests named by `paths`, read off the async runtime threads, united
    /// with what the resolver recorded for them.
    async fn image_config_paths_digests(&self, paths: &[PathBuf]) -> BTreeSet<String> {
        let owned = paths.to_vec();
        let read = tokio::task::spawn_blocking(move || {
            owned
                .into_iter()
                .map(|path| {
                    let found = image_config_digests(&path);
                    (path, found)
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_else(|error| {
            paths
                .iter()
                .map(|path| {
                    (
                        path.clone(),
                        Err(anyhow::anyhow!("image config read task failed: {error}")),
                    )
                })
                .collect()
        });
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
        for ((path, found), recorded) in read.into_iter().zip(recorded) {
            match found {
                Ok(found) => digests.extend(found),
                Err(error) => {
                    if recorded.is_none() {
                        metrics::counter!(
                            "agentenv_snapshot_layer_lease_unreadable_image_configs_total"
                        )
                        .increment(1);
                        if self.should_warn_unreadable(&path) {
                            warn!(
                                path = %path.display(),
                                error = %format!("{error:#}"),
                                "runtime image config unreadable and not recorded; its layers leave the lease after the tail"
                            );
                        }
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
    /// of every running and paused sandbox. Removed digests enter the tail.
    /// Growth wakes the refresher; nothing here waits for a lease write.
    pub(crate) async fn set_runtime_image_configs(&self, paths: Vec<PathBuf>) {
        let digests = self.image_config_paths_digests(&paths).await;
        let now = BootInstant::now();
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
            self.refresh_notify.notify_one();
        }
    }

    /// Add the digests named by `paths` to the runtime set without removing
    /// anything: used when the caller could not observe every sandbox, so a
    /// partial view can only over-protect.
    pub(crate) async fn add_runtime_image_configs(&self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        let digests = self.image_config_paths_digests(&paths).await;
        let grew = {
            let mut state = self.lock_state();
            state.runtime.extend(digests.iter().cloned());
            !state.durable_and_fresh(&digests, BootInstant::now())
        };
        if grew {
            self.refresh_notify.notify_one();
        }
    }

    /// Paused sandboxes restored from local persistence: keep every layer
    /// their image configs name in the runtime set and, in `delete` mode,
    /// gate the remote managed lowers (rule N1). Returns the configs that
    /// read a missing managed layer; their sandboxes must not resume. Local
    /// (`file=`) lowers are never gated, so a config without remote managed
    /// lowers needs no request. Configs that cannot be read are not checked
    /// (their resume fails opening them anyway). `report` mode only updates
    /// memory and returns no configs.
    pub(crate) async fn verify_persisted_image_configs(
        &self,
        paths: Vec<PathBuf>,
        check: PersistedLayerCheck,
    ) -> Result<Vec<PathBuf>> {
        self.add_runtime_image_configs(paths.clone()).await;
        if !self.is_gating() || paths.is_empty() {
            return Ok(Vec::new());
        }
        let managed_layers_repo_blob_url = self.managed_layers_repo_blob_url.clone();
        let managed = tokio::task::spawn_blocking(move || {
            paths
                .into_iter()
                .filter_map(|path| {
                    match image_config_managed_layer_digests(&path, &managed_layers_repo_blob_url) {
                        Ok(managed) => Some((path, managed)),
                        Err(error) => {
                            debug!(
                                path = %path.display(),
                                error = %format!("{error:#}"),
                                "persisted image config unreadable; not checking its managed layers"
                            );
                            None
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .await
        .context("join persisted image config read")?;
        let needed = managed
            .iter()
            .flat_map(|(_, digests)| digests.iter().cloned())
            .collect::<BTreeSet<_>>();
        if needed.is_empty() {
            return Ok(Vec::new());
        }
        let path = match check {
            PersistedLayerCheck::Startup => GatePath::Startup,
            PersistedLayerCheck::Resume => GatePath::Resume,
        };
        let (_, missing) = self
            .gate_reporting_missing(&needed, path)
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        if !missing.is_empty() {
            warn!(
                missing = ?missing,
                "managed layers of persisted paused sandboxes are missing; those sandboxes cannot resume"
            );
        }
        Ok(managed
            .into_iter()
            .filter(|(_, digests)| !digests.is_disjoint(&missing))
            .map(|(path, _)| path)
            .collect())
    }

    async fn ensure_durable(&self, required: &BTreeSet<String>) -> Result<()> {
        if self
            .lock_state()
            .durable_and_fresh(required, BootInstant::now())
        {
            return Ok(());
        }
        let _writer = self.writer.lock().await;
        if self
            .lock_state()
            .durable_and_fresh(required, BootInstant::now())
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

    /// One refresher step: write when the lease was never written, misses a
    /// live digest, stayed larger than the live set past the debounce, or is
    /// older than the liveness interval. Returns `false` when a write failed.
    pub(crate) async fn maintain(&self) -> bool {
        let now = BootInstant::now();
        let reason = {
            let mut guard = self.lock_state();
            let state = &mut *guard;
            let live = state.live_set(now);
            let age = state.last_success.map(|last| now.duration_since(last));
            if let Some(age) = age {
                metrics::gauge!("agentenv_snapshot_layer_lease_age_seconds").set(age.as_secs_f64());
            }
            // Only a strict shrink that persists starts or keeps the debounce.
            if !state
                .durable
                .as_ref()
                .is_some_and(|durable| live.is_subset(durable) && live != *durable)
            {
                state.shrink_pending_since = None;
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
        let Some(reason) = reason else {
            return true;
        };
        let _writer = self.writer.lock().await;
        match self.write_locked(reason).await {
            Ok(()) => true,
            Err(error) => {
                debug!(error = %format!("{error:#}"), "managed-layer lease refresh failed");
                false
            }
        }
    }

    /// Keep the lease fresh until shutdown: every tick, and promptly when a
    /// hold or the runtime set grew. After a failed write the refresher waits
    /// a full tick before writing again, so a store outage does not turn
    /// every hold into a new abandoned key.
    pub(crate) async fn run_refresher(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        info!(
            lease = %self.lease_key(),
            mode = self.mode.as_str(),
            "managed-layer lease refresher started"
        );
        loop {
            let ok = self.maintain().await;
            if ok {
                tokio::select! {
                    stop = sleep_or_shutdown(LEASE_REFRESH_TICK, &mut shutdown) => {
                        if stop {
                            break;
                        }
                    }
                    () = self.refresh_notify.notified() => {}
                }
            } else if sleep_or_shutdown(LEASE_REFRESH_TICK, &mut shutdown).await {
                break;
            }
        }
        debug!("managed-layer lease refresher stopped");
    }

    /// Write the current live set. The caller holds `self.writer`.
    async fn write_locked(&self, reason: WriteReason) -> Result<()> {
        let now = BootInstant::now();
        let (key, digests, last_success) = {
            let mut guard = self.lock_state();
            let state = &mut *guard;
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
            mode: self.mode.as_str().to_string(),
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
                // and replace the key's content with the attempted set.
                // Abandon the key: the next write goes to a new key. Whatever
                // lands, the abandoned key still holds every digest that both
                // its last content and the attempted set carry, until that
                // last success is LEASE_TTL old; `last_success` (and so the
                // freshness bound) stays with it.
                state.durable = state
                    .durable
                    .take()
                    .map(|durable| durable.intersection(&digests).cloned().collect());
                state.verified.retain(|digest| digests.contains(digest));
                state.generation += 1;
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

    /// List GC intents and wait out the deletion window when one of them
    /// names any of `digests`. Returns whether it waited. An intent counts
    /// however old it is: a runner removes its intent after its deletes (or
    /// after holding it for [`INTENT_WAIT`] when one failed), and any runner
    /// removes intents left by crashed passes once they are 10 minutes old.
    async fn wait_for_intents(&self, digests: &BTreeSet<String>, path: GatePath) -> Result<bool> {
        let intents = with_timeout(
            "list managed-layer GC intents",
            self.store.list_objects(LAYER_GC_INTENTS_PREFIX),
        )
        .await
        .context("list managed-layer GC intents")?;
        if intents.is_empty() {
            return Ok(false);
        }
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
        if !named.into_iter().any(|named| named) {
            return Ok(false);
        }
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
        Ok(true)
    }

    #[cfg(test)]
    pub(crate) fn live_digests(&self) -> BTreeSet<String> {
        self.lock_state().live_set(BootInstant::now())
    }

    #[cfg(test)]
    pub(crate) fn verified_digests(&self) -> BTreeSet<String> {
        self.lock_state().verified.iter().cloned().collect()
    }

    #[cfg(test)]
    pub(crate) fn instance(&self) -> &str {
        &self.instance
    }
}

/// Why a gate refused to trust a set of managed layers.
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

impl LayerCheckError {
    fn reason(&self) -> &'static str {
        match self {
            Self::Protect(_) => "lease",
            Self::Check { .. } => "check",
            Self::Missing { .. } => "missing",
        }
    }
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

/// Stat every digest under `managed-layers/` and return the missing ones.
/// Stops at the first check that errors.
async fn check_present(
    store: &Arc<dyn LayerStore>,
    digests: &BTreeSet<String>,
) -> std::result::Result<BTreeSet<String>, LayerCheckError> {
    stream::iter(digests.iter().cloned())
        .map(|digest| async move {
            let key = OssSnapshotArtifactLayout::managed_layer_key(&digest);
            match with_timeout("check managed layer", store.stat_object(&key)).await {
                Ok(Some(_)) => Ok(None),
                Ok(None) => Ok(Some(digest)),
                Err(error) => Err(LayerCheckError::Check { digest, error }),
            }
        })
        .buffer_unordered(LAYER_CHECK_CONCURRENCY)
        .try_filter_map(|missing| async move { Ok(missing) })
        .try_collect::<BTreeSet<String>>()
        .await
}

/// Holds digests in the node lease until dropped; they then stay in the
/// lease for [`LEASE_TAIL`].
pub(crate) struct LayerLeaseGuard {
    leases: Arc<LayerLeases>,
    digests: BTreeSet<String>,
}

impl LayerLeaseGuard {
    pub(crate) fn digests(&self) -> &BTreeSet<String> {
        &self.digests
    }
}

impl fmt::Debug for LayerLeaseGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayerLeaseGuard")
            .field("digests", &self.digests.len())
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
    fn is_gating(&self) -> bool {
        LayerLeases::is_gating(self)
    }

    async fn verify_persisted_image_configs(
        &self,
        paths: Vec<PathBuf>,
        check: PersistedLayerCheck,
    ) -> Result<Vec<PathBuf>> {
        LayerLeases::verify_persisted_image_configs(self, paths, check).await
    }

    async fn set_runtime_image_configs(&self, paths: Vec<PathBuf>) {
        LayerLeases::set_runtime_image_configs(self, paths).await
    }

    async fn add_runtime_image_configs(&self, paths: Vec<PathBuf>) {
        LayerLeases::add_runtime_image_configs(self, paths).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::Instant;

    use super::super::boot_clock::advance_boot_clock_for_test;
    use super::super::layer_refs::test_digest;
    use super::super::layer_store::fake::{FakeLayerStore, OpKind};
    use super::super::layout::LAYER_GC_LEASES_PREFIX as LAYER_GC_LEASES_PREFIX_FOR_TESTS;
    use super::*;

    const MANAGED_URL: &str = "s3://bucket/prefix/managed-layers";

    fn digests(indexes: &[usize]) -> BTreeSet<String> {
        indexes.iter().copied().map(test_digest).collect()
    }

    fn leases_in(store: &Arc<FakeLayerStore>, mode: SnapshotLayerGcMode) -> Arc<LayerLeases> {
        LayerLeases::new(
            Arc::clone(store) as Arc<dyn LayerStore>,
            "node/a",
            MANAGED_URL,
            mode,
        )
        .expect("leases")
    }

    fn gating(store: &Arc<FakeLayerStore>) -> Arc<LayerLeases> {
        leases_in(store, SnapshotLayerGcMode::Delete)
    }

    fn lease_document(store: &FakeLayerStore, key: &str) -> LeaseDocument {
        let body = store.read(key).expect("lease written");
        serde_json::from_slice(&body).expect("parse lease")
    }

    fn lease_body(store: &FakeLayerStore, leases: &LayerLeases) -> BTreeSet<String> {
        lease_document(store, &leases.lease_key())
            .digests
            .into_iter()
            .collect()
    }

    fn put_layers(store: &FakeLayerStore, indexes: &[usize]) {
        for index in indexes {
            store.insert(
                &OssSnapshotArtifactLayout::managed_layer_key(&test_digest(*index)),
                vec![0u8],
            );
        }
    }

    fn intent_naming(store: &FakeLayerStore, name: &str, named: &BTreeSet<String>) {
        store.insert(
            &format!("{LAYER_GC_INTENTS_PREFIX}{name}.json"),
            serde_json::to_vec(&serde_json::json!({ "digests": named })).unwrap(),
        );
    }

    fn count(store: &FakeLayerStore, kind: OpKind) -> usize {
        store.ops().iter().filter(|op| op.kind == kind).count()
    }

    #[test]
    fn off_mode_constructs_no_leases() {
        let store = Arc::new(FakeLayerStore::new());
        assert!(LayerLeases::new(
            store as Arc<dyn LayerStore>,
            "node",
            MANAGED_URL,
            SnapshotLayerGcMode::Off
        )
        .is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn report_mode_hold_never_touches_the_store_and_the_refresher_writes_one_report_lease() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases_in(&store, SnapshotLayerGcMode::Report);
        assert!(!leases.is_gating());
        store.fail(OpKind::Put, LAYER_GC_LEASES_PREFIX_FOR_TESTS);
        let guards = (1..=5)
            .map(|index| leases.hold(digests(&[index])))
            .collect::<Vec<_>>();
        assert!(store.ops().is_empty(), "hold does no store work");

        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("paused.json");
        std::fs::write(
            &config,
            serde_json::to_vec(&serde_json::json!({
                "repoBlobUrl": MANAGED_URL,
                "lowers": [{ "digest": test_digest(9), "size": 1, "dir": "/cache/9" }],
            }))
            .unwrap(),
        )
        .unwrap();
        let unverified = leases
            .verify_persisted_image_configs(vec![config], PersistedLayerCheck::Startup)
            .await
            .expect("report mode never fails");
        assert!(unverified.is_empty());
        assert!(store.ops().is_empty(), "report mode verifies nothing");

        store.clear_failures();
        let (stop, shutdown) = watch::channel(false);
        let refresher = tokio::spawn(Arc::clone(&leases).run_refresher(shutdown));
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(count(&store, OpKind::Put), 1, "one coalesced write");
        let document = lease_document(&store, &leases.lease_key());
        assert_eq!(document.version, 2);
        assert_eq!(document.mode, "report");
        assert_eq!(
            document.digests.into_iter().collect::<BTreeSet<_>>(),
            digests(&[1, 2, 3, 4, 5, 9])
        );
        assert!(
            !store.ops().iter().any(|op| op.kind == OpKind::List),
            "report mode never lists intents"
        );

        // Growth wakes the refresher promptly.
        let _more = leases.hold(digests(&[6]));
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(lease_body(&store, &leases).contains(&test_digest(6)));
        stop.send(true).unwrap();
        refresher.await.unwrap();
        drop(guards);
    }

    #[tokio::test(start_paused = true)]
    async fn gate_cold_path_orders_put_list_head_coalesces_and_then_takes_the_fast_path() {
        let store = Arc::new(FakeLayerStore::new());
        put_layers(&store, &[1, 2]);
        let leases = gating(&store);
        assert!(leases.lease_key().starts_with("layer-gc/leases/node_a-"));
        let mut pause = store.pause(OpKind::Put, LAYER_GC_LEASES_PREFIX_FOR_TESTS);

        let gate = |indexes: &'static [usize]| {
            let leases = Arc::clone(&leases);
            tokio::spawn(async move {
                let held = digests(indexes);
                let guard = leases.hold(held.clone());
                let outcome = leases.gate(&held, GatePath::Restore).await;
                (guard, outcome)
            })
        };
        let first = gate(&[1, 2]);
        pause.reached().await;
        let second = gate(&[1]);
        tokio::task::yield_now().await;
        assert!(!first.is_finished());
        assert!(!second.is_finished());
        pause.release();

        let (_first_guard, first) = first.await.unwrap();
        let (_second_guard, second) = second.await.unwrap();
        assert_eq!(first.expect("first gate"), GateOutcome::Cold);
        assert!(second.is_ok());
        assert_eq!(count(&store, OpKind::Put), 1, "concurrent gates coalesce");
        let document = lease_document(&store, &leases.lease_key());
        assert_eq!(document.mode, "delete");
        assert_eq!(lease_body(&store, &leases), digests(&[1, 2]));
        let ops = store.ops();
        let put = ops.iter().position(|op| op.kind == OpKind::Put).unwrap();
        let list = ops
            .iter()
            .position(|op| op.kind == OpKind::List && op.key == LAYER_GC_INTENTS_PREFIX)
            .expect("intent list");
        let first_stat = ops.iter().position(|op| op.kind == OpKind::Stat).unwrap();
        assert!(put < list && list < first_stat);

        store.clear_ops();
        let held = digests(&[1, 2]);
        let _guard = leases.hold(held.clone());
        assert_eq!(
            leases.gate(&held, GatePath::Restore).await.unwrap(),
            GateOutcome::Fast
        );
        assert!(store.ops().is_empty(), "verified digests need no request");
    }

    #[tokio::test(start_paused = true)]
    async fn gate_fails_closed_and_stops_at_the_first_check_error() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = gating(&store);
        let held = digests(&[1]);
        let _guard = leases.hold(held.clone());

        store.fail(OpKind::Put, LAYER_GC_LEASES_PREFIX_FOR_TESTS);
        let error = leases
            .gate(&held, GatePath::Restore)
            .await
            .expect_err("lease write failure fails the gate");
        assert!(matches!(error, LayerCheckError::Protect(_)), "{error}");
        assert!(
            !store.ops().iter().any(|op| op.kind == OpKind::List),
            "no intent check without a durable lease"
        );

        store.clear_failures();
        store.fail(OpKind::List, LAYER_GC_INTENTS_PREFIX);
        assert!(matches!(
            leases.gate(&held, GatePath::Restore).await,
            Err(LayerCheckError::Protect(_))
        ));

        store.clear_failures();
        assert!(matches!(
            leases.gate(&held, GatePath::Restore).await,
            Err(LayerCheckError::Missing { ref digest }) if *digest == test_digest(1)
        ));

        // One failing HEAD ends the check without waiting for slow ones.
        put_layers(&store, &[2, 3, 4]);
        for index in [2, 3] {
            store.set_latency(
                OpKind::Stat,
                &OssSnapshotArtifactLayout::managed_layer_key(&test_digest(index)),
                Duration::from_secs(10),
            );
        }
        store.fail(
            OpKind::Stat,
            &OssSnapshotArtifactLayout::managed_layer_key(&test_digest(4)),
        );
        let held = digests(&[2, 3, 4]);
        let _guard = leases.hold(held.clone());
        let started = Instant::now();
        let error = leases
            .gate(&held, GatePath::Publish)
            .await
            .expect_err("check error");
        assert!(matches!(error, LayerCheckError::Check { .. }), "{error}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test(start_paused = true)]
    async fn failed_write_keeps_verified_digests_fast_while_fresh_and_new_digests_fail_closed() {
        let store = Arc::new(FakeLayerStore::new());
        put_layers(&store, &[1, 2]);
        let leases = gating(&store);
        let held = digests(&[1]);
        let guard = leases.hold(held.clone());
        leases.gate(&held, GatePath::Restore).await.expect("cold");
        let abandoned = leases.lease_key();

        store.fail(OpKind::Put, LAYER_GC_LEASES_PREFIX_FOR_TESTS);
        tokio::time::advance(LEASE_LIVENESS_REWRITE).await;
        let attempted = leases.live_digests();
        assert!(!leases.maintain().await, "liveness write fails");
        assert_ne!(leases.lease_key(), abandoned, "the key is abandoned");
        // Whatever lands under the abandoned key (its last content or the
        // failed attempt) still carries every verified digest.
        let verified = leases.verified_digests();
        assert_eq!(verified, digests(&[1]));
        assert!(verified.is_subset(&lease_body_at(&store, &abandoned)));
        assert!(verified.is_subset(&attempted));

        store.clear_ops();
        assert_eq!(
            leases.gate(&held, GatePath::Restore).await.unwrap(),
            GateOutcome::Fast
        );
        assert!(store.ops().is_empty());

        let new = digests(&[2]);
        let _new_guard = leases.hold(new.clone());
        assert!(matches!(
            leases.gate(&new, GatePath::Restore).await,
            Err(LayerCheckError::Protect(_))
        ));

        // A host suspended past the freshness bound loses the fast path.
        advance_boot_clock_for_test(LEASE_FRESH_FOR);
        assert!(leases.gate(&held, GatePath::Restore).await.is_err());
        drop(guard);
    }

    fn lease_body_at(store: &FakeLayerStore, key: &str) -> BTreeSet<String> {
        lease_document(store, key).digests.into_iter().collect()
    }

    #[tokio::test(start_paused = true)]
    async fn a_named_intent_delays_the_gate_however_old_it_is_and_unnamed_ones_do_not() {
        let store = Arc::new(FakeLayerStore::new());
        put_layers(&store, &[1, 2]);
        let leases = gating(&store);
        intent_naming(&store, "other-pass", &digests(&[9]));
        let held = digests(&[1]);
        let _guard = leases.hold(held.clone());
        let started = Instant::now();
        assert_eq!(
            leases.gate(&held, GatePath::Publish).await.unwrap(),
            GateOutcome::Cold
        );
        assert!(
            started.elapsed() < INTENT_WAIT,
            "unnamed digests do not wait"
        );

        intent_naming(&store, "crashed-pass", &digests(&[2]));
        store.advance_s3_clock(Duration::from_secs(60 * 60));
        tokio::time::advance(Duration::from_secs(60 * 60)).await;
        let held = digests(&[2]);
        let _guard = leases.hold(held.clone());
        let started = Instant::now();
        assert_eq!(
            leases.gate(&held, GatePath::Restore).await.unwrap(),
            GateOutcome::IntentWait
        );
        assert!(started.elapsed() >= INTENT_WAIT, "named digests wait D_max");
    }

    #[tokio::test(start_paused = true)]
    async fn dropped_guard_and_removed_runtime_digests_stay_in_tail_for_two_hours() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = gating(&store);
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
        let guard = leases.hold(digests(&[1]));
        leases.maintain().await;
        assert_eq!(lease_body(&store, &leases), digests(&[1, 5]));

        drop(guard);
        leases.set_runtime_image_configs(Vec::new()).await;
        assert_eq!(leases.live_digests(), digests(&[1, 5]));

        tokio::time::advance(LEASE_TAIL - Duration::from_secs(60)).await;
        leases.maintain().await;
        assert_eq!(lease_body(&store, &leases), digests(&[1, 5]));

        // The tail counts host suspend.
        advance_boot_clock_for_test(Duration::from_secs(120));
        assert!(leases.live_digests().is_empty());
        leases.maintain().await;
        tokio::time::advance(LEASE_SHRINK_DEBOUNCE).await;
        leases.maintain().await;
        assert!(lease_body(&store, &leases).is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_growth_wakes_the_refresher_and_add_only_grows() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = leases_in(&store, SnapshotLayerGcMode::Report);
        let temp = tempfile::tempdir().unwrap();
        let evicted = temp.path().join("runtime/memory/image.json");
        leases.record_image_config(evicted.clone(), digests(&[7]));
        leases.set_runtime_image_configs(vec![evicted]).await;
        assert!(store.ops().is_empty(), "the runtime refresh never writes");
        leases.maintain().await;
        assert_eq!(
            lease_body(&store, &leases),
            digests(&[7]),
            "resolver-recorded digests survive file eviction"
        );

        let other = temp.path().join("other/image.json");
        leases.record_image_config(other.clone(), digests(&[8]));
        leases.add_runtime_image_configs(vec![other]).await;
        assert_eq!(leases.live_digests(), digests(&[7, 8]));
    }

    #[tokio::test(start_paused = true)]
    async fn failed_lease_write_moves_to_a_new_key_and_never_rewrites_the_old_one() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = gating(&store);
        leases.maintain().await;
        let first_key = leases.lease_key();
        assert!(store.contains(&first_key));

        // A failed write may land later with older content, so the key is
        // abandoned at once.
        store.fail(OpKind::Put, LAYER_GC_LEASES_PREFIX_FOR_TESTS);
        tokio::time::advance(LEASE_LIVENESS_REWRITE).await;
        leases.maintain().await;
        let rotated = leases.lease_key();
        assert_ne!(rotated, first_key);
        assert!(rotated.ends_with("-g1.json"));
        assert!(rotated.contains(leases.instance()));

        store.clear_failures();
        leases.maintain().await;
        assert!(store.contains(&rotated));

        store.clear_ops();
        tokio::time::advance(LEASE_LIVENESS_REWRITE).await;
        leases.maintain().await;
        let puts = store
            .ops()
            .into_iter()
            .filter(|op| op.kind == OpKind::Put)
            .map(|op| op.key)
            .collect::<Vec<_>>();
        assert_eq!(puts, vec![rotated], "only the new key is written");
    }

    fn write_image_config(path: &std::path::Path, lowers: serde_json::Value) {
        std::fs::write(
            path,
            serde_json::to_vec(&serde_json::json!({
                "repoBlobUrl": MANAGED_URL,
                "lowers": lowers,
                "upper": {},
                "resultFile": ""
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn persisted_configs_gate_only_remote_managed_lowers_and_report_missing_ones() {
        let store = Arc::new(FakeLayerStore::new());
        let leases = gating(&store);
        let temp = tempfile::tempdir().unwrap();
        let local_only = temp.path().join("local.json");
        write_image_config(
            &local_only,
            serde_json::json!([
                { "file": "/local/upper.commit", "digest": test_digest(3), "size": 3 }
            ]),
        );
        let verified = leases
            .verify_persisted_image_configs(vec![local_only.clone()], PersistedLayerCheck::Startup)
            .await
            .expect("local lowers need no request");
        assert!(verified.is_empty());
        assert!(store.ops().is_empty(), "local lowers are never gated");
        assert!(leases.live_digests().contains(&test_digest(3)));

        let present = temp.path().join("present.json");
        let missing = temp.path().join("missing.json");
        write_image_config(
            &present,
            serde_json::json!([
                { "digest": test_digest(1), "size": 1, "dir": "/cache/1" },
                { "file": "/local/upper.commit", "digest": test_digest(3), "size": 3 }
            ]),
        );
        write_image_config(
            &missing,
            serde_json::json!([{ "digest": test_digest(2), "size": 2, "dir": "/cache/2" }]),
        );
        put_layers(&store, &[1]);

        let reported = leases
            .verify_persisted_image_configs(
                vec![present.clone(), missing.clone(), local_only],
                PersistedLayerCheck::Resume,
            )
            .await
            .expect("verify persisted");
        assert_eq!(reported, vec![missing]);
        assert_eq!(lease_body(&store, &leases), digests(&[1, 2, 3]));
        let local_key = OssSnapshotArtifactLayout::managed_layer_key(&test_digest(3));
        assert!(
            !store
                .ops()
                .iter()
                .any(|op| op.kind == OpKind::Stat && op.key == local_key),
            "local lowers are not checked remotely"
        );

        store.clear_ops();
        let held = digests(&[1]);
        let _guard = leases.hold(held.clone());
        assert_eq!(
            leases.gate(&held, GatePath::Restore).await.unwrap(),
            GateOutcome::Fast,
            "present layers are verified"
        );
        assert!(store.ops().is_empty());
    }
}
