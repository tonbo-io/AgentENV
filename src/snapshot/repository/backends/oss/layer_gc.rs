//! Reference-counted collection of OSS managed layers.
//!
//! A pass recomputes, from scratch, which objects under `managed-layers/`
//! something still needs: committed catalog records (structured extraction
//! united with a raw digest scan) and live node leases. It deletes an object
//! only when all of these hold (see `docs/src/internals/snapshot-layer-gc.md`):
//!
//! - G1: its `LastModified` is older than the grace period, measured against
//!   the object-store time of this runner's own lease written at pass start;
//! - G2: no catalog record read by the pass names it;
//! - G3: the pass durably published a deletion intent naming it before its
//!   final lease listing, and no live lease returned by that listing names it;
//! - G4: its DELETE is issued within [`INTENT_ISSUE_WINDOW`] of the intent
//!   write returning;
//! - G5: the span from the catalog listing to the end of the final lease read
//!   is at most [`PASS_MAX_SPAN`].
//!
//! Nodes make a digest durable in their lease and then list intents before
//! they trust that the layer exists, so either the pass sees the node's lease
//! or the node sees the intent and re-checks after the deletion window.
//!
//! `report` mode runs the same classification, including the lease read, but
//! writes no intent and deletes nothing.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use futures::{stream, StreamExt};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use super::layer_leases::{
    sleep_or_shutdown, with_timeout, LayerLeases, INTENT_WAIT, LEASE_TTL, STALE_INTENT_AGE,
    STORE_REQUEST_TIMEOUT,
};
use super::layer_refs::{managed_key_digest, raw_digest_tokens, record_digests};
use super::layer_store::{LayerStore, StoredObject};
use super::layout::{
    OssSnapshotArtifactLayout, CATALOG_RECORDS_PREFIX, LAYER_GC_INTENTS_PREFIX,
    LAYER_GC_LEASES_PREFIX, LAYER_GC_RUNS_PREFIX, MANAGED_LAYERS_PREFIX,
};
use crate::cfg::{SnapshotLayerGcConfig, SnapshotLayerGcMode};

/// D_issue: every DELETE is issued within this long after the intent write
/// returned. Nodes that saw the intent wait `INTENT_WAIT` (120 s), which
/// covers this window, one request timeout and a margin.
pub(crate) const INTENT_ISSUE_WINDOW: Duration = Duration::from_secs(45);
/// P_max: maximum span from the catalog listing to the end of the final lease
/// read. Must stay below the lease tail (2 h).
pub(crate) const PASS_MAX_SPAN: Duration = Duration::from_secs(30 * 60);
/// Leases older than this (object-store time) are removed by any runner.
/// Live keys are rewritten at least hourly, and a key abandoned after a
/// failed write is never written again.
pub(crate) const STALE_LEASE_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Latest first pass after process start, off the wake-critical path. The
/// actual delay is drawn from the upper half of this bound so nodes that wake
/// together do not all run their first pass at once, while a 15-minute wake
/// still gets a pass.
pub(crate) const FIRST_PASS_DELAY: Duration = Duration::from_secs(10 * 60);

const RECORD_READ_CONCURRENCY: usize = 16;
const LEASE_READ_CONCURRENCY: usize = 16;
const DELETE_CONCURRENCY: usize = 8;
const GARBAGE_SAMPLE: usize = 20;
const DOCUMENT_VERSION: u32 = 1;
const INTERVAL_JITTER: f64 = 0.1;

/// Effective GC settings.
#[derive(Clone, Debug)]
pub(crate) struct LayerGcSettings {
    pub(crate) mode: SnapshotLayerGcMode,
    pub(crate) interval: Duration,
    pub(crate) grace: Duration,
    pub(crate) max_deletes_per_pass: usize,
}

impl LayerGcSettings {
    pub(crate) fn from_config(config: &SnapshotLayerGcConfig) -> Self {
        Self {
            mode: config.mode,
            interval: Duration::from_secs(config.interval_secs),
            grace: Duration::from_secs(config.grace_secs),
            max_deletes_per_pass: config.max_deletes_per_pass,
        }
    }
}

/// One deleted managed layer, for the run report and the audit log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DeletedLayer {
    pub(crate) digest: String,
    pub(crate) size: u64,
    pub(crate) last_modified_unix_ms: u64,
}

/// Outcome and counts of one pass. Also the run report object's content.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct LayerGcReport {
    pub(crate) version: u32,
    pub(crate) runner: String,
    pub(crate) pass_id: String,
    pub(crate) mode: String,
    /// `ok`, `skipped` or `aborted:<reason>`.
    pub(crate) outcome: String,
    pub(crate) duration_ms: u64,
    pub(crate) records_scanned: u64,
    pub(crate) unparsed_records: u64,
    pub(crate) leases_live: u64,
    pub(crate) leases_expired: u64,
    pub(crate) listed_objects: u64,
    pub(crate) listed_bytes: u64,
    pub(crate) referenced_objects: u64,
    pub(crate) referenced_bytes: u64,
    pub(crate) leased_only_objects: u64,
    pub(crate) leased_only_bytes: u64,
    pub(crate) young_objects: u64,
    pub(crate) young_bytes: u64,
    pub(crate) unrecognized_keys: u64,
    pub(crate) garbage_objects: u64,
    pub(crate) garbage_bytes: u64,
    pub(crate) deleted_objects: u64,
    pub(crate) deleted_bytes: u64,
    pub(crate) delete_failures: u64,
    /// Delete candidates left for the next pass because the issue window
    /// (G4) closed or shutdown began before their DELETE was issued.
    pub(crate) not_issued: u64,
    pub(crate) stale_intents_removed: u64,
    pub(crate) stale_leases_removed: u64,
    pub(crate) garbage_sample: Vec<String>,
    pub(crate) deleted: Vec<DeletedLayer>,
}

impl LayerGcReport {
    pub(crate) fn is_ok(&self) -> bool {
        self.outcome == "ok"
    }
}

#[derive(Debug, Serialize)]
struct IntentDocument<'a> {
    version: u32,
    runner: &'a str,
    pass_id: &'a str,
    digests: Vec<&'a str>,
}

#[derive(Debug, Deserialize)]
struct RunSummary {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    outcome: String,
}

enum Completion {
    Done,
    Skipped,
}

#[derive(Default)]
struct LeaseScan {
    digests: BTreeSet<String>,
    live: u64,
    expired: u64,
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn earlier(time: SystemTime, by: Duration) -> SystemTime {
    time.checked_sub(by).unwrap_or(UNIX_EPOCH)
}

fn mode_rank(mode: &str) -> u8 {
    match mode {
        "delete" => 2,
        "report" => 1,
        _ => 0,
    }
}

/// Managed-layer GC runner of one node process.
pub(crate) struct LayerGc {
    store: Arc<dyn LayerStore>,
    leases: Arc<LayerLeases>,
    settings: LayerGcSettings,
}

impl LayerGc {
    pub(crate) fn new(
        store: Arc<dyn LayerStore>,
        leases: Arc<LayerLeases>,
        settings: LayerGcSettings,
    ) -> Self {
        Self {
            store,
            leases,
            settings,
        }
    }

    pub(crate) fn mode(&self) -> SnapshotLayerGcMode {
        self.settings.mode
    }

    fn deletes(&self) -> bool {
        self.settings.mode == SnapshotLayerGcMode::Delete
    }

    /// Run passes until shutdown: the first after a random delay between half
    /// of and the full [`FIRST_PASS_DELAY`], then every interval with ±10%
    /// jitter.
    pub(crate) async fn run(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        if self.settings.mode == SnapshotLayerGcMode::Off {
            return;
        }
        info!(
            mode = self.settings.mode.as_str(),
            runner = %self.leases.owner(),
            interval_secs = self.settings.interval.as_secs(),
            grace_secs = self.settings.grace.as_secs(),
            max_deletes_per_pass = self.settings.max_deletes_per_pass,
            "snapshot layer gc started"
        );
        let first_delay = FIRST_PASS_DELAY.mul_f64(rand::rng().random_range(0.5..=1.0));
        if sleep_or_shutdown(first_delay, &mut shutdown).await {
            return;
        }
        loop {
            self.run_pass(&shutdown).await;
            let jitter = rand::rng().random_range(-INTERVAL_JITTER..=INTERVAL_JITTER);
            let delay = self.settings.interval.mul_f64(1.0 + jitter);
            if sleep_or_shutdown(delay, &mut shutdown).await {
                break;
            }
        }
        debug!("snapshot layer gc stopped");
    }

    /// Run one pass and publish its report (log line, metrics, run report).
    pub(crate) async fn run_pass(&self, shutdown: &watch::Receiver<bool>) -> LayerGcReport {
        let started = Instant::now();
        let mut report = LayerGcReport {
            version: DOCUMENT_VERSION,
            runner: self.leases.owner().to_string(),
            pass_id: uuid::Uuid::now_v7().simple().to_string(),
            mode: self.settings.mode.as_str().to_string(),
            ..LayerGcReport::default()
        };
        let skipped = match self.execute(&mut report, shutdown).await {
            Ok(Completion::Done) => {
                report.outcome = "ok".to_string();
                false
            }
            Ok(Completion::Skipped) => {
                report.outcome = "skipped".to_string();
                true
            }
            Err(error) => {
                report.outcome = format!("aborted:{error:#}");
                false
            }
        };
        report.duration_ms = started.elapsed().as_millis() as u64;
        self.publish_report(&report, skipped).await;
        report
    }

    async fn publish_report(&self, report: &LayerGcReport, skipped: bool) {
        let outcome_label = if report.is_ok() {
            "ok"
        } else if skipped {
            "skipped"
        } else {
            "aborted"
        };
        metrics::counter!(
            "agentenv_snapshot_layer_gc_passes_total",
            "mode" => report.mode.clone(),
            "outcome" => outcome_label,
        )
        .increment(1);
        if report.is_ok() {
            for (class, objects, bytes) in [
                (
                    "referenced",
                    report.referenced_objects,
                    report.referenced_bytes,
                ),
                (
                    "leased",
                    report.leased_only_objects,
                    report.leased_only_bytes,
                ),
                ("young", report.young_objects, report.young_bytes),
                ("unrecognized", report.unrecognized_keys, 0),
                ("garbage", report.garbage_objects, report.garbage_bytes),
            ] {
                metrics::gauge!("agentenv_snapshot_layer_gc_objects", "class" => class)
                    .set(objects as f64);
                metrics::gauge!("agentenv_snapshot_layer_gc_bytes", "class" => class)
                    .set(bytes as f64);
            }
            metrics::gauge!("agentenv_snapshot_layer_gc_last_success_timestamp_seconds")
                .set(unix_ms(SystemTime::now()) as f64 / 1000.0);
        }
        metrics::counter!("agentenv_snapshot_layer_gc_deleted_objects_total")
            .increment(report.deleted_objects);
        metrics::counter!("agentenv_snapshot_layer_gc_deleted_bytes_total")
            .increment(report.deleted_bytes);
        metrics::counter!("agentenv_snapshot_layer_gc_delete_failures_total")
            .increment(report.delete_failures);
        metrics::counter!("agentenv_snapshot_layer_gc_not_issued_total")
            .increment(report.not_issued);

        macro_rules! pass_line {
            ($level:ident) => {
                $level!(
                    mode = %report.mode,
                    runner = %report.runner,
                    pass_id = %report.pass_id,
                    outcome = %report.outcome,
                    duration_ms = report.duration_ms,
                    records_scanned = report.records_scanned,
                    unparsed_records = report.unparsed_records,
                    leases_live = report.leases_live,
                    leases_expired = report.leases_expired,
                    listed_objects = report.listed_objects,
                    listed_bytes = report.listed_bytes,
                    referenced_objects = report.referenced_objects,
                    referenced_bytes = report.referenced_bytes,
                    leased_only_objects = report.leased_only_objects,
                    leased_only_bytes = report.leased_only_bytes,
                    young_objects = report.young_objects,
                    young_bytes = report.young_bytes,
                    unrecognized_keys = report.unrecognized_keys,
                    garbage_objects = report.garbage_objects,
                    garbage_bytes = report.garbage_bytes,
                    deleted_objects = report.deleted_objects,
                    deleted_bytes = report.deleted_bytes,
                    delete_failures = report.delete_failures,
                    not_issued = report.not_issued,
                    stale_intents_removed = report.stale_intents_removed,
                    stale_leases_removed = report.stale_leases_removed,
                    "snapshot layer gc pass complete"
                )
            };
        }
        if report.is_ok() || skipped {
            pass_line!(info);
        } else {
            pass_line!(warn);
        }

        if skipped {
            return;
        }
        let key = OssSnapshotArtifactLayout::layer_gc_run_key(self.leases.owner());
        let body = match serde_json::to_vec(report) {
            Ok(body) => body,
            Err(error) => {
                warn!(error = %error, "failed to serialize snapshot layer gc run report");
                return;
            }
        };
        if let Err(error) = with_timeout(
            "write snapshot layer gc run report",
            self.store.put_object(&key, body.into()),
        )
        .await
        {
            warn!(
                key = %key,
                error = %format!("{error:#}"),
                "failed to write snapshot layer gc run report"
            );
        }
    }

    async fn execute(
        &self,
        report: &mut LayerGcReport,
        shutdown: &watch::Receiver<bool>,
    ) -> Result<Completion> {
        // G1 reference time: the object-store time of our own fresh lease.
        let lease_key = self.leases.refresh_now().await.context("own lease write")?;
        let own_lease = with_timeout("stat own lease", self.store.stat_object(&lease_key))
            .await
            .context("own lease stat")?
            .ok_or_else(|| anyhow!("own lease '{lease_key}' missing after write"))?;
        let t0 = own_lease.last_modified;

        if self.another_runner_completed_recently(t0).await {
            return Ok(Completion::Skipped);
        }
        if *shutdown.borrow() {
            return Err(anyhow!("shutdown"));
        }

        let span_started = Instant::now();
        let classified = tokio::time::timeout(PASS_MAX_SPAN, self.classify(report, t0, shutdown))
            .await
            .map_err(|_| anyhow!("pass span exceeded {}s", PASS_MAX_SPAN.as_secs()))??;
        let Some(Classified {
            candidates,
            mut leased,
        }) = classified
        else {
            // Report mode deletes no managed layer, but it still removes
            // stale protocol objects so that intents and leases left behind
            // (for example after switching back from delete mode) do not
            // accumulate.
            self.remove_stale_protocol_objects(report, t0).await;
            return Ok(Completion::Done);
        };

        let deleted = self
            .delete_candidates(report, t0, span_started, candidates, &mut leased, shutdown)
            .await?;
        report.deleted_objects = deleted.len() as u64;
        report.deleted_bytes = deleted.iter().map(|layer| layer.size).sum();
        report.deleted = deleted;

        self.remove_stale_protocol_objects(report, t0).await;
        Ok(Completion::Done)
    }

    /// Best-effort duplicate-work avoidance: skip when another runner wrote a
    /// successful report of at least our mode within half an interval.
    async fn another_runner_completed_recently(&self, t0: SystemTime) -> bool {
        let own_key = OssSnapshotArtifactLayout::layer_gc_run_key(self.leases.owner());
        let since = earlier(t0, self.settings.interval / 2);
        let runs = match with_timeout(
            "list snapshot layer gc run reports",
            self.store.list_objects(LAYER_GC_RUNS_PREFIX),
        )
        .await
        {
            Ok(runs) => runs,
            Err(error) => {
                debug!(error = %format!("{error:#}"), "could not list snapshot layer gc run reports");
                return false;
            }
        };
        for run in runs
            .into_iter()
            .filter(|run| run.key != own_key && run.last_modified >= since)
        {
            let Ok(Some(body)) =
                with_timeout("read run report", self.store.get_object(&run.key)).await
            else {
                continue;
            };
            let Ok(summary) = serde_json::from_slice::<RunSummary>(&body) else {
                continue;
            };
            if summary.outcome == "ok"
                && mode_rank(&summary.mode) >= mode_rank(self.settings.mode.as_str())
            {
                debug!(other = %run.key, "another snapshot layer gc runner completed recently; skipping pass");
                return true;
            }
        }
        false
    }

    async fn classify(
        &self,
        report: &mut LayerGcReport,
        t0: SystemTime,
        shutdown: &watch::Receiver<bool>,
    ) -> Result<Option<Classified>> {
        let referenced = self.read_catalog(report).await?;
        let objects = self
            .store
            .list_objects(MANAGED_LAYERS_PREFIX)
            .await
            .context("managed-layer list")?;
        let leases = self.read_leases(t0).await?;
        report.leases_live = leases.live;
        report.leases_expired = leases.expired;
        if *shutdown.borrow() {
            return Err(anyhow!("shutdown"));
        }

        let young_since = earlier(t0, self.settings.grace);
        let mut garbage = Vec::new();
        for object in objects {
            report.listed_objects += 1;
            report.listed_bytes += object.size;
            let Some(digest) = managed_key_digest(&object.key) else {
                report.unrecognized_keys += 1;
                continue;
            };
            if referenced.contains(digest) {
                report.referenced_objects += 1;
                report.referenced_bytes += object.size;
            } else if leases.digests.contains(digest) {
                report.leased_only_objects += 1;
                report.leased_only_bytes += object.size;
            } else if object.last_modified >= young_since {
                report.young_objects += 1;
                report.young_bytes += object.size;
            } else {
                report.garbage_objects += 1;
                report.garbage_bytes += object.size;
                garbage.push(object);
            }
        }
        garbage.sort_by(|left, right| {
            left.last_modified
                .cmp(&right.last_modified)
                .then_with(|| left.key.cmp(&right.key))
        });
        report.garbage_sample = garbage
            .iter()
            .take(GARBAGE_SAMPLE)
            .filter_map(|object| managed_key_digest(&object.key).map(str::to_string))
            .collect();

        if !self.deletes() {
            return Ok(None);
        }
        garbage.truncate(self.settings.max_deletes_per_pass);
        Ok(Some(Classified {
            candidates: garbage,
            leased: leases.digests,
        }))
    }

    /// G2: digests named by any catalog record. Aborts on any read error other
    /// than a record deleted between listing and reading.
    async fn read_catalog(&self, report: &mut LayerGcReport) -> Result<BTreeSet<String>> {
        let records = self
            .store
            .list_objects(CATALOG_RECORDS_PREFIX)
            .await
            .context("catalog list")?;
        let store = &self.store;
        let results = stream::iter(records)
            .map(|record| async move {
                let body = with_timeout("read catalog record", store.get_object(&record.key))
                    .await
                    .with_context(|| format!("catalog record '{}' read", record.key))?;
                Ok::<_, anyhow::Error>((record.key, body))
            })
            .buffer_unordered(RECORD_READ_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        let mut referenced = BTreeSet::new();
        for result in results {
            let (key, body) = result?;
            let Some(body) = body else {
                // Deleted between listing and reading.
                continue;
            };
            report.records_scanned += 1;
            let extracted = record_digests(&body);
            if !extracted.parsed {
                report.unparsed_records += 1;
            }
            if extracted.committed && extracted.digests.is_empty() {
                return Err(anyhow!(
                    "committed catalog record '{key}' names no layer digests"
                ));
            }
            referenced.extend(extracted.digests);
        }
        Ok(referenced)
    }

    /// Digests named by live leases (object-store `LastModified` within
    /// [`LEASE_TTL`] of `t0`). Aborts on any read error other than a lease
    /// deleted between listing and reading.
    async fn read_leases(&self, t0: SystemTime) -> Result<LeaseScan> {
        let listed = with_timeout(
            "list managed-layer leases",
            self.store.list_objects(LAYER_GC_LEASES_PREFIX),
        )
        .await
        .context("lease list")?;
        let live_since = earlier(t0, LEASE_TTL);
        let mut scan = LeaseScan::default();
        let mut live = Vec::new();
        for lease in listed {
            if lease.last_modified >= live_since {
                live.push(lease);
            } else {
                scan.expired += 1;
            }
        }
        let store = &self.store;
        let bodies = stream::iter(live)
            .map(|lease| async move {
                with_timeout("read managed-layer lease", store.get_object(&lease.key))
                    .await
                    .with_context(|| format!("lease '{}' read", lease.key))
            })
            .buffer_unordered(LEASE_READ_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        for body in bodies {
            if let Some(body) = body? {
                scan.live += 1;
                scan.digests.extend(raw_digest_tokens(&body));
            }
        }
        Ok(scan)
    }

    async fn delete_candidates(
        &self,
        report: &mut LayerGcReport,
        t0: SystemTime,
        span_started: Instant,
        candidates: Vec<StoredObject>,
        leased: &mut BTreeSet<String>,
        shutdown: &watch::Receiver<bool>,
    ) -> Result<Vec<DeletedLayer>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let intent_key =
            OssSnapshotArtifactLayout::layer_gc_intent_key(self.leases.owner(), &report.pass_id);
        let intent = IntentDocument {
            version: DOCUMENT_VERSION,
            runner: self.leases.owner(),
            pass_id: &report.pass_id,
            digests: candidates
                .iter()
                .filter_map(|object| managed_key_digest(&object.key))
                .collect(),
        };
        let body = serde_json::to_vec(&intent).context("serialize deletion intent")?;
        if let Err(error) = with_timeout(
            "write deletion intent",
            self.store.put_object(&intent_key, body.into()),
        )
        .await
        {
            // The write may still land; it then ages out as a stale intent.
            return Err(error.context("deletion intent write"));
        }
        let intent_acked = Instant::now();

        // G3: the final lease read starts after the intent is durable.
        let final_leases = match tokio::time::timeout(
            PASS_MAX_SPAN.saturating_sub(span_started.elapsed()),
            self.read_leases(t0),
        )
        .await
        {
            Ok(Ok(scan)) => scan,
            Ok(Err(error)) => {
                self.remove_intent(&intent_key).await;
                return Err(error.context("final lease read"));
            }
            Err(_) => {
                self.remove_intent(&intent_key).await;
                return Err(anyhow!("pass span exceeded {}s", PASS_MAX_SPAN.as_secs()));
            }
        };
        // G5.
        if span_started.elapsed() > PASS_MAX_SPAN {
            self.remove_intent(&intent_key).await;
            return Err(anyhow!("pass span exceeded {}s", PASS_MAX_SPAN.as_secs()));
        }
        leased.extend(final_leases.digests);

        let mut to_delete = Vec::new();
        for object in candidates {
            let Some(digest) = managed_key_digest(&object.key) else {
                continue;
            };
            if leased.contains(digest) {
                // Leased after classification: reclassify.
                report.garbage_objects -= 1;
                report.garbage_bytes -= object.size;
                report.leased_only_objects += 1;
                report.leased_only_bytes += object.size;
                continue;
            }
            to_delete.push((digest.to_string(), object));
        }

        // G4: issue each DELETE within the window after the intent write.
        let deadline = intent_acked + INTENT_ISSUE_WINDOW;
        let store = &self.store;
        let outcomes = stream::iter(to_delete)
            .map(|(digest, object)| async move {
                if Instant::now() > deadline || *shutdown.borrow() {
                    return (digest, object, None);
                }
                let result =
                    with_timeout("delete managed layer", store.delete_object(&object.key)).await;
                (digest, object, Some(result))
            })
            .buffer_unordered(DELETE_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

        let mut deleted = Vec::new();
        for (digest, object, outcome) in outcomes {
            match outcome {
                Some(Ok(())) => {
                    info!(
                        digest = %digest,
                        size = object.size,
                        last_modified_unix_ms = unix_ms(object.last_modified),
                        pass_id = %report.pass_id,
                        "snapshot layer gc deleted managed layer"
                    );
                    deleted.push(DeletedLayer {
                        digest,
                        size: object.size,
                        last_modified_unix_ms: unix_ms(object.last_modified),
                    });
                }
                Some(Err(error)) => {
                    report.delete_failures += 1;
                    warn!(
                        digest = %digest,
                        error = %format!("{error:#}"),
                        "snapshot layer gc failed to delete managed layer"
                    );
                }
                None => report.not_issued += 1,
            }
        }
        if report.not_issued > 0 {
            warn!(
                not_issued = report.not_issued,
                window_secs = INTENT_ISSUE_WINDOW.as_secs(),
                "snapshot layer gc issue window closed before every delete was issued; the rest wait for the next pass"
            );
        }
        deleted.sort_by(|left, right| {
            left.last_modified_unix_ms
                .cmp(&right.last_modified_unix_ms)
                .then_with(|| left.digest.cmp(&right.digest))
        });
        if report.delete_failures == 0 {
            self.remove_intent(&intent_key).await;
        }
        // A failed or timed-out DELETE may still land later, so its intent
        // stays until it ages out as stale and nodes keep re-checking.
        Ok(deleted)
    }

    async fn remove_intent(&self, intent_key: &str) {
        if let Err(error) = with_timeout(
            "delete deletion intent",
            self.store.delete_object(intent_key),
        )
        .await
        {
            debug!(
                key = %intent_key,
                error = %format!("{error:#}"),
                "failed to delete snapshot layer gc intent; it ages out as stale"
            );
        }
    }

    /// Remove intents of finished or crashed passes and week-old leases.
    async fn remove_stale_protocol_objects(&self, report: &mut LayerGcReport, t0: SystemTime) {
        let stale_intents_before = earlier(t0, STALE_INTENT_AGE);
        match with_timeout(
            "list deletion intents",
            self.store.list_objects(LAYER_GC_INTENTS_PREFIX),
        )
        .await
        {
            Ok(intents) => {
                for intent in intents
                    .into_iter()
                    .filter(|intent| intent.last_modified < stale_intents_before)
                {
                    match with_timeout("delete stale intent", self.store.delete_object(&intent.key))
                        .await
                    {
                        Ok(()) => report.stale_intents_removed += 1,
                        Err(error) => debug!(
                            key = %intent.key,
                            error = %format!("{error:#}"),
                            "failed to delete stale snapshot layer gc intent"
                        ),
                    }
                }
            }
            Err(error) => {
                debug!(error = %format!("{error:#}"), "failed to list snapshot layer gc intents")
            }
        }

        let leases = match self.read_stale_leases(t0).await {
            Ok(leases) => leases,
            Err(error) => {
                debug!(error = %format!("{error:#}"), "failed to list managed-layer leases for hygiene");
                return;
            }
        };
        for key in leases {
            match with_timeout("delete stale lease", self.store.delete_object(&key)).await {
                Ok(()) => report.stale_leases_removed += 1,
                Err(error) => debug!(
                    key = %key,
                    error = %format!("{error:#}"),
                    "failed to delete stale managed-layer lease"
                ),
            }
        }
    }

    async fn read_stale_leases(&self, t0: SystemTime) -> Result<Vec<String>> {
        let stale_before = earlier(t0, STALE_LEASE_AGE);
        let own = self.leases.lease_key();
        Ok(with_timeout(
            "list managed-layer leases",
            self.store.list_objects(LAYER_GC_LEASES_PREFIX),
        )
        .await?
        .into_iter()
        .filter(|lease| lease.last_modified < stale_before && lease.key != own)
        .map(|lease| lease.key)
        .collect())
    }
}

struct Classified {
    candidates: Vec<StoredObject>,
    leased: BTreeSet<String>,
}

// Nodes that see an intent wait out the deletion window: the issue window,
// a DELETE request timeout, the intent write timeout, and a margin.
const _: () = assert!(
    INTENT_ISSUE_WINDOW.as_secs() + 2 * STORE_REQUEST_TIMEOUT.as_secs() + 45
        <= INTENT_WAIT.as_secs()
);

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use bytes::Bytes;
    use serde_json::json;

    use super::super::layer_leases::{
        protect_and_check, LayerCheckError, ProtectPath, INTENT_WAIT,
    };
    use super::super::layer_refs::test_digest;
    use super::super::layer_store::fake::{split_mix, FakeLayerStore, FakeObjects, Op, OpKind};
    use super::*;
    use crate::snapshot::{
        CommittedSnapshot, ExternalLayer, ManagedLayer, OverlaybdLayerRef, SnapshotRecord,
    };

    const HOUR: Duration = Duration::from_secs(60 * 60);
    const OLD: Duration = Duration::from_secs(48 * 60 * 60);
    const MANAGED_URL: &str = "s3://bucket/prefix/managed-layers";

    fn settings(mode: SnapshotLayerGcMode) -> LayerGcSettings {
        LayerGcSettings {
            mode,
            interval: HOUR,
            grace: HOUR,
            max_deletes_per_pass: 1000,
        }
    }

    fn gc_with(
        store: &Arc<FakeLayerStore>,
        node: &str,
        settings: LayerGcSettings,
    ) -> (Arc<LayerLeases>, LayerGc) {
        let dyn_store = Arc::clone(store) as Arc<dyn LayerStore>;
        let leases = LayerLeases::new(Arc::clone(&dyn_store), node, MANAGED_URL);
        let gc = LayerGc::new(dyn_store, Arc::clone(&leases), settings);
        (leases, gc)
    }

    fn gc(store: &Arc<FakeLayerStore>, mode: SnapshotLayerGcMode) -> LayerGc {
        gc_with(store, "gc-node", settings(mode)).1
    }

    fn node(store: &Arc<FakeLayerStore>, name: &str) -> Arc<LayerLeases> {
        LayerLeases::new(Arc::clone(store) as Arc<dyn LayerStore>, name, MANAGED_URL)
    }

    fn running() -> watch::Receiver<bool> {
        watch::channel(false).1
    }

    fn layer_key(index: usize) -> String {
        OssSnapshotArtifactLayout::managed_layer_key(&test_digest(index))
    }

    fn put_layer(store: &FakeLayerStore, index: usize, age: Duration) {
        store.insert_at(&layer_key(index), vec![0u8; 10], store.s3_now() - age);
    }

    fn has_layer(store: &FakeLayerStore, index: usize) -> bool {
        store.contains(&layer_key(index))
    }

    fn record_bytes(digests: &[usize]) -> Vec<u8> {
        let mut committed = CommittedSnapshot::mock();
        committed.memory_layers = digests
            .iter()
            .map(|index| ManagedLayer {
                digest: test_digest(*index),
                size: 10,
                uuid: None,
            })
            .collect();
        serde_json::to_vec(&SnapshotRecord::mock_ready(committed)).unwrap()
    }

    fn put_record(store: &FakeLayerStore, name: &str, digests: &[usize]) {
        store.insert(
            &format!("{CATALOG_RECORDS_PREFIX}{name}.json"),
            record_bytes(digests),
        );
    }

    fn lease_body(digests: &[usize]) -> Vec<u8> {
        let digests = digests.iter().copied().map(test_digest).collect::<Vec<_>>();
        serde_json::to_vec(&json!({ "version": 1, "digests": digests })).unwrap()
    }

    fn put_lease(store: &FakeLayerStore, name: &str, digests: &[usize], age: Duration) {
        store.insert_at(
            &format!("{LAYER_GC_LEASES_PREFIX}{name}.json"),
            lease_body(digests),
            store.s3_now() - age,
        );
    }

    fn put_intent(store: &FakeLayerStore, name: &str, digests: &[usize], age: Duration) {
        store.insert_at(
            &format!("{LAYER_GC_INTENTS_PREFIX}{name}.json"),
            lease_body(digests),
            store.s3_now() - age,
        );
    }

    fn deleted_digests(report: &LayerGcReport) -> Vec<String> {
        report
            .deleted
            .iter()
            .map(|layer| layer.digest.clone())
            .collect()
    }

    fn position(ops: &[Op], kind: OpKind, prefix: &str) -> Option<usize> {
        ops.iter()
            .position(|op| op.kind == kind && op.key.starts_with(prefix))
    }

    fn rposition(ops: &[Op], kind: OpKind, prefix: &str) -> Option<usize> {
        ops.iter()
            .rposition(|op| op.kind == kind && op.key.starts_with(prefix))
    }

    #[tokio::test(start_paused = true)]
    async fn report_mode_classifies_without_writing_intent_or_deleting() {
        let store = Arc::new(FakeLayerStore::new());
        put_record(&store, "r1", &[1]);
        put_lease(&store, "other-node", &[2], Duration::ZERO);
        put_layer(&store, 1, OLD);
        put_layer(&store, 2, OLD);
        put_layer(&store, 3, Duration::from_secs(10 * 60));
        put_layer(&store, 4, OLD);
        store.insert_at(
            &format!("{MANAGED_LAYERS_PREFIX}sha256:abc"),
            vec![0u8; 10],
            store.s3_now() - OLD,
        );
        let gc = gc(&store, SnapshotLayerGcMode::Report);

        let report = gc.run_pass(&running()).await;

        assert_eq!(report.outcome, "ok");
        assert_eq!(report.mode, "report");
        assert_eq!(report.listed_objects, 5);
        assert_eq!(report.listed_bytes, 50);
        assert_eq!(report.records_scanned, 1);
        assert_eq!(report.referenced_objects, 1);
        assert_eq!(report.leased_only_objects, 1);
        assert_eq!(report.young_objects, 1);
        assert_eq!(report.unrecognized_keys, 1);
        assert_eq!(report.garbage_objects, 1);
        assert_eq!(report.garbage_bytes, 10);
        assert_eq!(report.garbage_sample, vec![test_digest(4)]);
        assert_eq!(report.deleted_objects, 0);
        let ops = store.ops();
        assert!(position(&ops, OpKind::Put, LAYER_GC_INTENTS_PREFIX).is_none());
        assert!(position(&ops, OpKind::Delete, "").is_none());
        for index in 1..=4 {
            assert!(has_layer(&store, index));
        }
        let run = store
            .read(&OssSnapshotArtifactLayout::layer_gc_run_key(&report.runner))
            .expect("run report written");
        let run: LayerGcReport = serde_json::from_slice(&run).unwrap();
        assert_eq!(run.garbage_objects, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn delete_mode_deletes_only_old_unreferenced_unleased_layers_oldest_first_and_capped() {
        let store = Arc::new(FakeLayerStore::new());
        put_record(&store, "r1", &[1]);
        put_lease(&store, "other-node", &[2], Duration::ZERO);
        put_layer(&store, 1, OLD);
        put_layer(&store, 2, OLD);
        put_layer(&store, 3, Duration::from_secs(10 * 60));
        put_layer(&store, 10, 5 * HOUR);
        put_layer(&store, 11, 3 * HOUR);
        put_layer(&store, 12, 4 * HOUR);
        let (_, gc) = gc_with(
            &store,
            "gc-node",
            LayerGcSettings {
                max_deletes_per_pass: 2,
                ..settings(SnapshotLayerGcMode::Delete)
            },
        );

        let report = gc.run_pass(&running()).await;

        assert_eq!(report.outcome, "ok");
        assert_eq!(report.garbage_objects, 3);
        assert_eq!(
            deleted_digests(&report),
            vec![test_digest(10), test_digest(12)]
        );
        assert_eq!(report.deleted_objects, 2);
        assert_eq!(report.deleted_bytes, 20);
        assert!(!has_layer(&store, 10));
        assert!(!has_layer(&store, 12));
        for index in [1, 2, 3, 11] {
            assert!(has_layer(&store, index), "layer {index} kept");
        }
        assert!(store.keys(LAYER_GC_INTENTS_PREFIX).is_empty());

        let next = gc.run_pass(&running()).await;
        assert_eq!(deleted_digests(&next), vec![test_digest(11)]);
    }

    #[tokio::test(start_paused = true)]
    async fn external_refs_with_managed_url_and_unparseable_records_keep_layers() {
        let store = Arc::new(FakeLayerStore::new());
        let mut committed = CommittedSnapshot::mock();
        committed.rootfs_layers = vec![
            OverlaybdLayerRef::External(ExternalLayer {
                digest: test_digest(1),
                repo_blob_url: MANAGED_URL.to_string(),
                size: 10,
            }),
            OverlaybdLayerRef::External(ExternalLayer {
                digest: test_digest(2),
                repo_blob_url: "https://registry.example/v2/ns/image/blobs".to_string(),
                size: 10,
            }),
        ];
        store.insert(
            &format!("{CATALOG_RECORDS_PREFIX}restored.json"),
            serde_json::to_vec(&SnapshotRecord::mock_ready(committed)).unwrap(),
        );
        store.insert(
            &format!("{CATALOG_RECORDS_PREFIX}future.json"),
            format!("{{\"future_format\": [\"{}\"", test_digest(3)),
        );
        for index in 1..=4 {
            put_layer(&store, index, OLD);
        }
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;

        assert_eq!(report.outcome, "ok");
        assert_eq!(report.unparsed_records, 1);
        assert_eq!(report.referenced_objects, 3);
        assert_eq!(deleted_digests(&report), vec![test_digest(4)]);
        for index in 1..=3 {
            assert!(has_layer(&store, index));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn aborts_without_deletes_on_record_get_error_lease_get_error_list_error_or_missing_own_lease(
    ) {
        type Setup = fn(&FakeLayerStore);
        let cases: Vec<(&str, Setup)> = vec![
            ("record get", |store| {
                store.fail(OpKind::Get, CATALOG_RECORDS_PREFIX)
            }),
            ("record list", |store| {
                store.fail(OpKind::List, CATALOG_RECORDS_PREFIX)
            }),
            ("lease get", |store| {
                store.fail(OpKind::Get, &format!("{LAYER_GC_LEASES_PREFIX}other"))
            }),
            ("lease list", |store| {
                store.fail(OpKind::List, LAYER_GC_LEASES_PREFIX)
            }),
            ("layer list", |store| {
                store.fail(OpKind::List, MANAGED_LAYERS_PREFIX)
            }),
            ("own lease missing", |store| {
                store.set_before_op(|op, objects| {
                    if op.kind == OpKind::Stat && op.key.starts_with(LAYER_GC_LEASES_PREFIX) {
                        objects.remove(&op.key);
                    }
                })
            }),
        ];
        for (name, setup) in cases {
            let store = Arc::new(FakeLayerStore::new());
            put_record(&store, "r1", &[1]);
            put_lease(&store, "other-node", &[2], Duration::ZERO);
            for index in 1..=3 {
                put_layer(&store, index, OLD);
            }
            setup(&store);
            let gc = gc(&store, SnapshotLayerGcMode::Delete);

            let report = gc.run_pass(&running()).await;

            assert!(
                report.outcome.starts_with("aborted:"),
                "{name}: {}",
                report.outcome
            );
            assert!(has_layer(&store, 3), "{name}: garbage kept");
            assert!(
                position(&store.ops(), OpKind::Delete, MANAGED_LAYERS_PREFIX).is_none(),
                "{name}: no layer deletes"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn committed_record_without_tokens_aborts_the_pass() {
        let store = Arc::new(FakeLayerStore::new());
        store.insert(
            &format!("{CATALOG_RECORDS_PREFIX}empty.json"),
            serde_json::to_vec(&SnapshotRecord::mock_ready(CommittedSnapshot::mock())).unwrap(),
        );
        put_layer(&store, 1, OLD);
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;
        assert!(report.outcome.starts_with("aborted:"), "{}", report.outcome);
        assert!(has_layer(&store, 1));

        // A record that is not committed yet (template build) names nothing
        // and is fine.
        let mut pending = SnapshotRecord::mock_ready(CommittedSnapshot::mock());
        pending.committed = None;
        store.insert(
            &format!("{CATALOG_RECORDS_PREFIX}empty.json"),
            serde_json::to_vec(&pending).unwrap(),
        );
        let report = gc.run_pass(&running()).await;
        assert_eq!(report.outcome, "ok");
        assert!(!has_layer(&store, 1));
    }

    #[tokio::test(start_paused = true)]
    async fn expired_leases_are_ignored_and_live_leases_protect() {
        let store = Arc::new(FakeLayerStore::new());
        put_lease(&store, "live", &[1], LEASE_TTL - Duration::from_secs(1));
        put_lease(&store, "expired", &[2], LEASE_TTL + Duration::from_secs(1));
        put_layer(&store, 1, OLD);
        put_layer(&store, 2, OLD);
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;

        assert_eq!(report.outcome, "ok");
        assert_eq!(report.leases_live, 2, "own lease and the live one");
        assert_eq!(report.leases_expired, 1);
        assert!(has_layer(&store, 1));
        assert!(!has_layer(&store, 2));
    }

    #[tokio::test(start_paused = true)]
    async fn intent_put_precedes_final_lease_list_and_is_deleted_after_deletes() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;
        assert_eq!(deleted_digests(&report), vec![test_digest(1)]);

        let ops = store.ops();
        let intent_put = position(&ops, OpKind::Put, LAYER_GC_INTENTS_PREFIX).expect("intent put");
        let layer_delete =
            position(&ops, OpKind::Delete, MANAGED_LAYERS_PREFIX).expect("layer delete");
        let intent_delete = ops
            .iter()
            .position(|op| op.kind == OpKind::Delete && op.key == ops[intent_put].key)
            .expect("intent delete");
        // The final lease read is the last lease listing before the first
        // layer delete (hygiene lists leases again afterwards).
        let final_lease_list =
            rposition(&ops[..layer_delete], OpKind::List, LAYER_GC_LEASES_PREFIX)
                .expect("final lease list before deletes");
        assert!(intent_put < final_lease_list);
        assert!(final_lease_list < layer_delete);
        assert!(layer_delete < intent_delete);
    }

    #[tokio::test(start_paused = true)]
    async fn deletes_stop_at_the_issue_deadline_and_pass_aborts_beyond_p_max() {
        let store = Arc::new(FakeLayerStore::new());
        for index in 0..40 {
            put_layer(&store, 100 + index, OLD);
        }
        // Batches of 8 complete every 14 s: issued at 0, 14, 28 and 42 s;
        // the fifth batch would start at 56 s, past the 45 s window.
        store.set_latency(
            OpKind::Delete,
            MANAGED_LAYERS_PREFIX,
            Duration::from_secs(14),
        );
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;
        assert_eq!(report.outcome, "ok");
        assert_eq!(report.deleted_objects, 32);
        assert_eq!(report.delete_failures, 0);
        assert_eq!(
            report.not_issued, 8,
            "candidates past the window are counted"
        );
        assert_eq!(store.keys(MANAGED_LAYERS_PREFIX).len(), 8);

        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        store.set_latency(
            OpKind::List,
            CATALOG_RECORDS_PREFIX,
            PASS_MAX_SPAN + Duration::from_secs(60),
        );
        let gc = self::gc(&store, SnapshotLayerGcMode::Delete);
        let report = gc.run_pass(&running()).await;
        assert!(
            report.outcome.contains("pass span exceeded"),
            "{}",
            report.outcome
        );
        assert!(has_layer(&store, 1));
        assert!(position(&store.ops(), OpKind::Put, LAYER_GC_INTENTS_PREFIX).is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn final_lease_read_beyond_p_max_aborts_after_the_intent_and_removes_it() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        let gc = Arc::new(gc(&store, SnapshotLayerGcMode::Delete));
        let mut pause = store.pause(OpKind::Put, LAYER_GC_INTENTS_PREFIX);
        let pass = tokio::spawn({
            let gc = Arc::clone(&gc);
            async move {
                let shutdown = running();
                gc.run_pass(&shutdown).await
            }
        });
        // The intent write is reached after classification; from now on the
        // final lease listing takes longer than the whole pass may.
        pause.reached().await;
        store.set_latency(
            OpKind::List,
            LAYER_GC_LEASES_PREFIX,
            PASS_MAX_SPAN + Duration::from_secs(60),
        );
        pause.release();
        let report = pass.await.unwrap();

        assert!(
            report.outcome.contains("pass span exceeded"),
            "{}",
            report.outcome
        );
        assert!(has_layer(&store, 1), "no delete after a G5 abort");
        assert!(position(&store.ops(), OpKind::Delete, MANAGED_LAYERS_PREFIX).is_none());
        assert!(
            position(&store.ops(), OpKind::Put, LAYER_GC_INTENTS_PREFIX).is_some(),
            "the intent was written"
        );
        assert!(
            store.keys(LAYER_GC_INTENTS_PREFIX).is_empty(),
            "the aborted pass removes its intent"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn stale_intents_and_week_old_leases_are_cleaned_in_every_mode_but_fresh_ones_are_not() {
        for mode in [SnapshotLayerGcMode::Report, SnapshotLayerGcMode::Delete] {
            let store = Arc::new(FakeLayerStore::new());
            put_intent(
                &store,
                "crashed",
                &[1],
                STALE_INTENT_AGE + Duration::from_secs(60),
            );
            put_intent(&store, "running", &[2], Duration::from_secs(5 * 60));
            put_lease(&store, "gone", &[3], STALE_LEASE_AGE + HOUR);
            put_lease(&store, "expired", &[4], 2 * LEASE_TTL);
            put_layer(&store, 5, OLD);

            let report = gc(&store, mode).run_pass(&running()).await;
            assert_eq!(report.outcome, "ok", "{mode:?}");
            assert_eq!(report.stale_intents_removed, 1, "{mode:?}");
            assert_eq!(report.stale_leases_removed, 1, "{mode:?}");
            assert_eq!(
                store.keys(LAYER_GC_INTENTS_PREFIX),
                vec![format!("{LAYER_GC_INTENTS_PREFIX}running.json")],
                "{mode:?}"
            );
            assert!(!store.contains(&format!("{LAYER_GC_LEASES_PREFIX}gone.json")));
            assert!(store.contains(&format!("{LAYER_GC_LEASES_PREFIX}expired.json")));
            assert_eq!(
                has_layer(&store, 5),
                mode == SnapshotLayerGcMode::Report,
                "{mode:?}: only delete mode removes managed layers"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn runner_skips_when_another_runner_completed_within_half_interval() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        let other_run = format!("{LAYER_GC_RUNS_PREFIX}other.json");
        let run = |mode: &str, outcome: &str| {
            serde_json::to_vec(&json!({ "mode": mode, "outcome": outcome })).unwrap()
        };

        store.insert_at(
            &other_run,
            run("delete", "ok"),
            store.s3_now() - Duration::from_secs(10 * 60),
        );
        let report = gc(&store, SnapshotLayerGcMode::Delete)
            .run_pass(&running())
            .await;
        assert_eq!(report.outcome, "skipped");
        assert!(position(&store.ops(), OpKind::List, CATALOG_RECORDS_PREFIX).is_none());
        assert!(has_layer(&store, 1));

        // A report-mode run does not stand in for a delete-mode pass.
        store.insert_at(
            &other_run,
            run("report", "ok"),
            store.s3_now() - Duration::from_secs(10 * 60),
        );
        let report = gc(&store, SnapshotLayerGcMode::Delete)
            .run_pass(&running())
            .await;
        assert_eq!(report.outcome, "ok");
        assert!(!has_layer(&store, 1));

        put_layer(&store, 2, OLD);
        for key in store.keys(LAYER_GC_RUNS_PREFIX) {
            store.remove(&key);
        }
        store.insert_at(
            &other_run,
            run("delete", "ok"),
            store.s3_now() - Duration::from_secs(40 * 60),
        );
        let report = gc_with(&store, "third", settings(SnapshotLayerGcMode::Delete))
            .1
            .run_pass(&running())
            .await;
        assert_eq!(report.outcome, "ok");
        assert!(!has_layer(&store, 2));
    }

    #[tokio::test(start_paused = true)]
    async fn publication_dedup_after_final_lease_list_sees_intent_waits_and_fails_retryably_when_layer_deleted(
    ) {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        let publication = node(&store, "publisher");
        let started = Instant::now();
        let spawned = Arc::new(Mutex::new(None));
        // The publication deduplicated against layer 1 earlier and reaches its
        // pre-commit check just as the GC issues the delete, i.e. after the
        // GC's final lease listing.
        store.set_before_op({
            let publication = Arc::clone(&publication);
            let spawned = Arc::clone(&spawned);
            move |op: &Op, _: &mut FakeObjects| {
                if op.kind == OpKind::Delete && op.key == layer_key(1) {
                    let publication = Arc::clone(&publication);
                    let task = tokio::spawn(async move {
                        let result = protect_and_check(
                            &publication,
                            BTreeSet::from([test_digest(1)]),
                            ProtectPath::Publish,
                            true,
                        )
                        .await;
                        (result.map(|_| ()), Instant::now())
                    });
                    *spawned.lock().unwrap() = Some(task);
                }
            }
        });
        // Keep the intent visible while the publication checks intents.
        store.set_latency(
            OpKind::Delete,
            LAYER_GC_INTENTS_PREFIX,
            Duration::from_secs(10),
        );
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;
        assert_eq!(deleted_digests(&report), vec![test_digest(1)]);

        let task = spawned.lock().unwrap().take().expect("publication started");
        let (result, finished) = task.await.unwrap();
        assert!(
            matches!(result, Err(LayerCheckError::Missing { ref digest }) if *digest == test_digest(1)),
            "{result:?}"
        );
        assert!(
            finished.duration_since(started) >= INTENT_WAIT,
            "the publication saw the intent and waited out the deletion window"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn lease_made_durable_before_final_lease_list_blocks_deletion() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        put_layer(&store, 2, OLD);
        // A restore leases layer 1 between classification and the final
        // lease listing: the intent names it, the final listing sees it.
        store.set_before_op(|op: &Op, objects: &mut FakeObjects| {
            if op.kind == OpKind::Put && op.key.starts_with(LAYER_GC_INTENTS_PREFIX) {
                objects.put(
                    &format!("{LAYER_GC_LEASES_PREFIX}restorer.json"),
                    lease_body(&[1]),
                );
            }
        });
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;

        assert_eq!(deleted_digests(&report), vec![test_digest(2)]);
        assert_eq!(report.garbage_objects, 1);
        assert_eq!(report.leased_only_objects, 1);
        assert!(has_layer(&store, 1));
    }

    #[tokio::test(start_paused = true)]
    async fn record_written_after_catalog_scan_is_protected_by_lease_tail() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        let publication = node(&store, "publisher");
        let guard = protect_and_check(
            &publication,
            BTreeSet::from([test_digest(1)]),
            ProtectPath::Publish,
            true,
        )
        .await
        .expect("pre-commit check");
        // The record lands after the GC listed the catalog; the publication's
        // guard is already gone and only its tail holds the layer.
        store.set_before_op(|op: &Op, objects: &mut FakeObjects| {
            if op.kind == OpKind::List && op.key == MANAGED_LAYERS_PREFIX {
                objects.put(
                    &format!("{CATALOG_RECORDS_PREFIX}published.json"),
                    record_bytes(&[1]),
                );
            }
        });
        drop(guard);
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;
        assert_eq!(report.outcome, "ok");
        assert_eq!(report.leased_only_objects, 1);
        assert!(has_layer(&store, 1));

        let report = gc.run_pass(&running()).await;
        assert_eq!(report.referenced_objects, 1);
        assert!(has_layer(&store, 1));
    }

    #[tokio::test(start_paused = true)]
    async fn restore_of_snapshot_deleted_before_catalog_scan_is_protected_or_fails_cleanly() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        put_layer(&store, 2, OLD);
        put_record(&store, "running", &[1]);
        put_record(&store, "deleted", &[2]);
        let restorer = node(&store, "restorer");

        // Restored before the snapshot was deleted: protected while it runs.
        let running_guest = protect_and_check(
            &restorer,
            BTreeSet::from([test_digest(1)]),
            ProtectPath::Restore,
            false,
        )
        .await
        .expect("restore");
        store.remove(&format!("{CATALOG_RECORDS_PREFIX}running.json"));
        store.remove(&format!("{CATALOG_RECORDS_PREFIX}deleted.json"));
        let gc = gc(&store, SnapshotLayerGcMode::Delete);
        let report = gc.run_pass(&running()).await;
        assert_eq!(deleted_digests(&report), vec![test_digest(2)]);
        assert!(has_layer(&store, 1));

        // Restore of a snapshot whose layer was collected fails cleanly.
        let error = protect_and_check(
            &restorer,
            BTreeSet::from([test_digest(2)]),
            ProtectPath::Restore,
            false,
        )
        .await
        .expect_err("collected layer");
        assert!(matches!(error, LayerCheckError::Missing { .. }));
        drop(running_guest);
    }

    #[tokio::test(start_paused = true)]
    async fn two_concurrent_runners_and_a_publication_never_delete_a_needed_layer() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        put_layer(&store, 2, OLD);
        put_layer(&store, 3, OLD);
        put_record(&store, "r2", &[2]);
        store.set_random_latency(7, Duration::from_millis(500));
        let (_, first) = gc_with(&store, "gc-a", settings(SnapshotLayerGcMode::Delete));
        let (_, second) = gc_with(&store, "gc-b", settings(SnapshotLayerGcMode::Delete));
        let publication = node(&store, "publisher");
        let publish = async {
            let guard = protect_and_check(
                &publication,
                BTreeSet::from([test_digest(1)]),
                ProtectPath::Publish,
                true,
            )
            .await;
            if guard.is_ok() {
                store.insert(
                    &format!("{CATALOG_RECORDS_PREFIX}r1.json"),
                    record_bytes(&[1]),
                );
            }
            guard.map(|_| ())
        };

        let shutdown = running();
        let (a, b, published) = tokio::join!(
            first.run_pass(&shutdown),
            second.run_pass(&shutdown),
            publish
        );

        assert!(a.outcome == "ok" || a.outcome == "skipped", "{}", a.outcome);
        assert!(b.outcome == "ok" || b.outcome == "skipped", "{}", b.outcome);
        assert!(has_layer(&store, 2), "referenced layer kept");
        match published {
            Ok(()) => assert!(has_layer(&store, 1), "published layer kept"),
            Err(error) => assert!(matches!(error, LayerCheckError::Missing { .. })),
        }
        assert!(!has_layer(&store, 3), "garbage collected");
    }

    #[tokio::test(start_paused = true)]
    async fn crash_after_intent_write_forces_slow_path_until_the_intent_is_stale_then_cleans_it() {
        let store = Arc::new(FakeLayerStore::new());
        put_intent(&store, "crashed-runner-pass", &[5], Duration::ZERO);

        let started = Instant::now();
        let _guard = node(&store, "node-a")
            .protect(BTreeSet::from([test_digest(5)]), ProtectPath::Restore)
            .await
            .expect("protect");
        assert!(
            started.elapsed() >= INTENT_WAIT,
            "stale intent forces the slow path"
        );

        // Once the intent is older than the stale age, nodes ignore it even
        // while no runner removes it (for example with GC switched off).
        store.advance_s3_clock(STALE_INTENT_AGE + Duration::from_secs(60));
        let started = Instant::now();
        let _guard = node(&store, "node-b")
            .protect(BTreeSet::from([test_digest(5)]), ProtectPath::Restore)
            .await
            .expect("protect");
        assert!(started.elapsed() < INTENT_WAIT, "fast again once stale");

        let report = gc(&store, SnapshotLayerGcMode::Report)
            .run_pass(&running())
            .await;
        assert_eq!(report.stale_intents_removed, 1);
        assert!(store.keys(LAYER_GC_INTENTS_PREFIX).is_empty());
    }

    /// Shared state of one randomized run.
    #[derive(Default)]
    struct Tracker {
        held: BTreeMap<String, usize>,
        violations: Vec<String>,
        deleted: usize,
    }

    fn check_invariant(objects: &FakeObjects, tracker: &mut Tracker, at: &str) {
        let mut needed = BTreeSet::new();
        for key in objects.keys_with_prefix(CATALOG_RECORDS_PREFIX) {
            let body = objects.get(&key).expect("listed key");
            needed.extend(raw_digest_tokens(&body));
        }
        needed.extend(tracker.held.keys().cloned());
        for digest in needed {
            if !objects.contains(&OssSnapshotArtifactLayout::managed_layer_key(&digest)) {
                tracker.violations.push(format!("{digest} missing at {at}"));
            }
        }
    }

    async fn random_sleep(rng: &Mutex<u64>, max_secs: u64) {
        let secs = split_mix(&mut rng.lock().unwrap()) % (max_secs + 1);
        tokio::time::sleep(Duration::from_secs(secs)).await;
    }

    fn pick(rng: &Mutex<u64>, n: u64) -> u64 {
        split_mix(&mut rng.lock().unwrap()) % n
    }

    async fn run_seed(seed: u64) -> Tracker {
        let store = Arc::new(FakeLayerStore::new());
        for index in 0..6 {
            put_layer(&store, index, OLD);
        }
        put_record(&store, "r0", &[0, 1]);
        put_record(&store, "r1", &[2, 3]);
        store.set_random_latency(seed, Duration::from_secs(3));
        let tracker = Arc::new(Mutex::new(Tracker::default()));
        store.set_before_op({
            let tracker = Arc::clone(&tracker);
            move |op: &Op, objects: &mut FakeObjects| {
                let mut tracker = tracker.lock().unwrap();
                if op.kind == OpKind::Delete
                    && op.key.starts_with(MANAGED_LAYERS_PREFIX)
                    && objects.contains(&op.key)
                {
                    tracker.deleted += 1;
                }
                check_invariant(objects, &mut tracker, &format!("{:?} {}", op.kind, op.key));
            }
        });
        let rng_state = Mutex::new(seed ^ 0x5eed);
        let node_leases = [node(&store, "node-a"), node(&store, "node-b")];
        // Grace 0: every unreferenced object is a candidate immediately, so
        // safety rests on the protocol alone.
        // A short interval keeps the duplicate-work skip window (half the
        // interval) below the spacing of most passes, so the two runners
        // genuinely overlap instead of skipping each other.
        let gc_settings = LayerGcSettings {
            grace: Duration::ZERO,
            interval: Duration::from_secs(120),
            ..settings(SnapshotLayerGcMode::Delete)
        };
        let (_, gc_a) = gc_with(&store, "gc-a", gc_settings.clone());
        let (_, gc_b) = gc_with(&store, "gc-b", gc_settings);

        let rng = &rng_state;
        let nodes = &node_leases;
        let shared_store = &store;
        let shared_tracker = &tracker;
        let gc_actor = move |gc: LayerGc| async move {
            let shutdown = running();
            for _ in 0..3 {
                random_sleep(rng, 200).await;
                gc.run_pass(&shutdown).await;
            }
        };
        let restore = move |actor: u64| async move {
            random_sleep(rng, 300).await;
            let record = format!("{CATALOG_RECORDS_PREFIX}r{}.json", pick(rng, 2));
            let Ok(Some(body)) = shared_store.get_object(&record).await else {
                return;
            };
            let digests = raw_digest_tokens(&body);
            let leases = &nodes[(actor % 2) as usize];
            let Ok(guard) =
                protect_and_check(leases, digests.clone(), ProtectPath::Restore, false).await
            else {
                return;
            };
            {
                let mut tracker = shared_tracker.lock().unwrap();
                for digest in &digests {
                    *tracker.held.entry(digest.clone()).or_default() += 1;
                }
            }
            random_sleep(rng, 400).await;
            {
                let mut tracker = shared_tracker.lock().unwrap();
                for digest in &digests {
                    let count = tracker.held.get_mut(digest).expect("held");
                    *count -= 1;
                    if *count == 0 {
                        tracker.held.remove(digest);
                    }
                }
            }
            drop(guard);
        };
        let publish = move |actor: u64| async move {
            random_sleep(rng, 300).await;
            let base = pick(rng, 6) as usize;
            let fresh = 100 + actor as usize;
            let digests = BTreeSet::from([test_digest(base), test_digest(fresh)]);
            let leases = &nodes[(actor % 2) as usize];
            for _attempt in 0..3 {
                // Upload, deduplicating against existing objects.
                for digest in &digests {
                    let key = OssSnapshotArtifactLayout::managed_layer_key(digest);
                    if !matches!(shared_store.stat_object(&key).await, Ok(Some(_))) {
                        let _ = shared_store
                            .put_object(&key, Bytes::from_static(b"layer"))
                            .await;
                    }
                }
                match protect_and_check(leases, digests.clone(), ProtectPath::Publish, true).await {
                    Ok(_guard) => {
                        let _ = shared_store
                            .put_object(
                                &format!("{CATALOG_RECORDS_PREFIX}p{actor}.json"),
                                record_bytes(&[base, fresh]).into(),
                            )
                            .await;
                        return;
                    }
                    Err(LayerCheckError::Missing { .. }) => continue,
                    Err(_) => return,
                }
            }
        };
        let delete_record = move || async move {
            random_sleep(rng, 300).await;
            let record = format!("{CATALOG_RECORDS_PREFIX}r{}.json", pick(rng, 2));
            let _ = shared_store.delete_object(&record).await;
        };

        tokio::join!(
            gc_actor(gc_a),
            gc_actor(gc_b),
            restore(0),
            restore(1),
            restore(2),
            publish(0),
            publish(1),
            publish(2),
            delete_record(),
            delete_record(),
        );

        // One more operation runs the invariant check on the final state.
        let _ = store.list_objects(MANAGED_LAYERS_PREFIX).await;
        let mut final_state = tracker.lock().unwrap();
        std::mem::take(&mut *final_state)
    }

    #[test]
    fn randomized_interleavings_never_delete_a_needed_layer() {
        let mut deleted = 0;
        for seed in 0..200u64 {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .start_paused(true)
                .build()
                .unwrap();
            let tracker = runtime.block_on(run_seed(seed));
            assert!(
                tracker.violations.is_empty(),
                "seed {seed}: {:?}",
                tracker.violations
            );
            deleted += tracker.deleted;
        }
        assert!(deleted > 0, "the runs exercised deletions");
    }
}
