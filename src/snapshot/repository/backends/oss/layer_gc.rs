//! Reference-counted collection of OSS managed layers.
//!
//! A pass recomputes, from scratch, which objects under `managed-layers/`
//! something still needs: committed catalog records (structured extraction
//! united with a raw digest scan) and live node leases. It deletes an object
//! only when all of these hold (see `docs/src/internals/snapshot-layer-gc.md`):
//!
//! - G1: its `LastModified` is older than the grace period, measured against
//!   the object-store time `T0` of this runner's own lease written at pass
//!   start;
//! - G2: no catalog record read by the pass names it;
//! - G3: the pass durably published a deletion intent naming it before its
//!   final lease listing, and no live lease returned by that listing names it;
//! - G4: its DELETE is issued within [`INTENT_ISSUE_WINDOW`] of the intent's
//!   `LastModified`: the intent is rewritten after the final lease read and
//!   its new `LastModified` must be within [`PRE_DELETE_BUDGET`] of the first
//!   (object-store time), and DELETEs are issued only within
//!   [`DELETE_PHASE_BUDGET`] of that rewrite on the boot clock;
//! - G5: the rewritten intent's `LastModified` is within [`PASS_MAX_SPAN`] of
//!   `T0` (object-store time), so the catalog listing and the end of the
//!   final lease read are at most that far apart;
//! - G6: every live lease in both lease reads declares `delete` mode and
//!   every catalog record parses; otherwise the pass degrades to a report.
//!
//! Delete-mode nodes make a digest durable in their lease and then list
//! intents before they trust that the layer exists, so either the pass sees
//! the node's lease or the node sees the intent and re-checks after the
//! deletion window.
//!
//! `report` mode runs the same classification, including the lease read, but
//! writes no intent and deletes no managed layer.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures::{stream, StreamExt};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use super::boot_clock::BootInstant;
use super::layer_leases::{
    sleep_or_shutdown, with_timeout, LayerLeases, INTENT_WAIT, LEASE_TTL, STORE_REQUEST_TIMEOUT,
};
use super::layer_refs::{managed_key_digest, raw_digest_tokens, record_digests};
use super::layer_store::{LayerStore, StoredObject};
use super::layout::{
    OssSnapshotArtifactLayout, CATALOG_RECORDS_PREFIX, LAYER_GC_INTENTS_PREFIX,
    LAYER_GC_LEASES_PREFIX, LAYER_GC_RUNS_PREFIX, MANAGED_LAYERS_PREFIX,
};
use crate::cfg::{SnapshotLayerGcConfig, SnapshotLayerGcMode};

/// D_issue: every DELETE is issued within this long after the intent's
/// `LastModified`. Nodes that saw the intent wait [`INTENT_WAIT`], which
/// covers this window, one DELETE timeout and the landing bound.
pub(crate) const INTENT_ISSUE_WINDOW: Duration = Duration::from_secs(45);
/// G4a: the intent's rewrite after the final lease read must carry a
/// `LastModified` at most this long (minus the store's 1 s precision) after
/// the first intent write. Also the liveness timeout of the final lease read.
pub(crate) const PRE_DELETE_BUDGET: Duration = Duration::from_secs(20);
/// G4b: DELETEs are issued only within this long after the rewrite was sent,
/// on the boot clock.
pub(crate) const DELETE_PHASE_BUDGET: Duration = Duration::from_secs(20);
/// A3: a DELETE that timed out lands, if at all, within this long after its
/// timeout.
pub(crate) const DELETE_LANDING_BOUND: Duration = Duration::from_secs(60);
/// P_max: maximum span from the catalog listing to the end of the final lease
/// read (checked in object-store time). Must stay below the lease tail (2 h).
pub(crate) const PASS_MAX_SPAN: Duration = Duration::from_secs(30 * 60);
/// Object-store `LastModified` has whole-second precision.
const STORE_TIME_PRECISION: Duration = Duration::from_secs(1);
/// Leases older than this (object-store time) are removed by any runner.
/// Live keys are rewritten at least hourly, and a key abandoned after a
/// failed write is never written again.
pub(crate) const STALE_LEASE_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Intents older than this (object-store time) belong to finished or crashed
/// passes whose deletion window closed long ago; any runner removes them.
pub(crate) const STALE_INTENT_AGE: Duration = Duration::from_secs(10 * 60);
/// Latest first pass after process start, off the wake-critical path. The
/// actual delay is drawn from the upper half of this bound so nodes that wake
/// together do not all run their first pass at once, while a 15-minute wake
/// still gets a pass.
pub(crate) const FIRST_PASS_DELAY: Duration = Duration::from_secs(10 * 60);

const RECORD_READ_CONCURRENCY: usize = 16;
const LEASE_READ_CONCURRENCY: usize = 16;
const DELETE_CONCURRENCY: usize = 8;
const GARBAGE_SAMPLE: usize = 20;
const NON_GATING_SAMPLE: usize = 20;
const DOCUMENT_VERSION: u32 = 1;
const INTERVAL_JITTER: f64 = 0.1;

// Every DELETE is issued within the issue window: the rewrite lands within
// PRE_DELETE_BUDGET of the intent (with 1 s precision slack), and DELETEs go
// out within DELETE_PHASE_BUDGET of the rewrite being sent.
const _: () = assert!(
    PRE_DELETE_BUDGET.as_secs() + STORE_TIME_PRECISION.as_secs() + DELETE_PHASE_BUDGET.as_secs()
        <= INTENT_ISSUE_WINDOW.as_secs()
);
// Nodes that see an intent wait out the deletion window: the issue window, a
// DELETE request timeout, and the landing bound of a timed-out DELETE.
const _: () = assert!(
    INTENT_ISSUE_WINDOW.as_secs()
        + STORE_REQUEST_TIMEOUT.as_secs()
        + DELETE_LANDING_BOUND.as_secs()
        <= INTENT_WAIT.as_secs()
);

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
    /// `ok`, `degraded`, `skipped` or `aborted:<reason>`.
    pub(crate) outcome: String,
    /// Why a delete-mode pass ran as a report (rule G6), for example
    /// `non_gating_leases:3` or `unparsed_records:1`.
    #[serde(default)]
    pub(crate) degraded: Option<String>,
    pub(crate) duration_ms: u64,
    pub(crate) records_scanned: u64,
    pub(crate) unparsed_records: u64,
    pub(crate) leases_live: u64,
    pub(crate) leases_expired: u64,
    /// Live leases that do not declare `delete` mode (report-mode or older
    /// writers): while any exists, delete passes degrade to report passes.
    #[serde(default)]
    pub(crate) non_gating_leases: u64,
    /// Up to 20 keys of those leases, so operators can see which node still
    /// runs report mode.
    #[serde(default)]
    pub(crate) non_gating_sample: Vec<String>,
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
    /// Delete candidates left for the next pass because the delete phase
    /// (G4) closed or shutdown began before their DELETE was issued.
    pub(crate) not_issued: u64,
    pub(crate) stale_intents_removed: u64,
    pub(crate) stale_leases_removed: u64,
    pub(crate) garbage_sample: Vec<String>,
    pub(crate) deleted: Vec<DeletedLayer>,
}

impl LayerGcReport {
    /// Whether the pass completed its classification (`ok` or `degraded`).
    pub(crate) fn classified(&self) -> bool {
        self.outcome == "ok" || self.outcome == "degraded"
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

/// The `mode` a lease declares. Anything but exactly `delete` (a missing
/// field, version 1, another value, an unparseable body) does not gate.
#[derive(Debug, Deserialize)]
struct LeaseModeField {
    #[serde(default)]
    mode: Option<String>,
}

fn lease_declares_gating(body: &[u8]) -> bool {
    serde_json::from_slice::<LeaseModeField>(body)
        .ok()
        .and_then(|lease| lease.mode)
        .is_some_and(|mode| mode == SnapshotLayerGcMode::Delete.as_str())
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
    non_gating: u64,
    non_gating_sample: Vec<String>,
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn earlier(time: SystemTime, by: Duration) -> SystemTime {
    time.checked_sub(by).unwrap_or(UNIX_EPOCH)
}

/// `later - earlier` in object-store time; zero when `later` is earlier.
fn store_span(later: SystemTime, earlier: SystemTime) -> Duration {
    later.duration_since(earlier).unwrap_or(Duration::ZERO)
}

fn mode_rank(mode: &str) -> u8 {
    match mode {
        "delete" => 2,
        "report" => 1,
        _ => 0,
    }
}

/// Managed-layer GC runner of one node process (`report` or `delete` mode).
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

    fn deletes(&self) -> bool {
        self.settings.mode == SnapshotLayerGcMode::Delete
    }

    /// Run passes until shutdown: the first after a random delay between half
    /// of and the full [`FIRST_PASS_DELAY`], then every interval with ±10%
    /// jitter.
    pub(crate) async fn run(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
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
                report.outcome = if report.degraded.is_some() {
                    "degraded".to_string()
                } else {
                    "ok".to_string()
                };
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
        let outcome_label = if report.classified() {
            report.outcome.as_str()
        } else if skipped {
            "skipped"
        } else {
            "aborted"
        };
        metrics::counter!(
            "agentenv_snapshot_layer_gc_passes_total",
            "mode" => report.mode.clone(),
            "outcome" => outcome_label.to_string(),
        )
        .increment(1);
        if report.classified() {
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
            metrics::gauge!("agentenv_snapshot_layer_gc_non_gating_leases")
                .set(report.non_gating_leases as f64);
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
                    degraded = report.degraded.as_deref().unwrap_or(""),
                    duration_ms = report.duration_ms,
                    records_scanned = report.records_scanned,
                    unparsed_records = report.unparsed_records,
                    leases_live = report.leases_live,
                    leases_expired = report.leases_expired,
                    non_gating_leases = report.non_gating_leases,
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
        if report.classified() || skipped {
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
        // G1 and G5 reference time: the object-store time of our own fresh
        // lease, written before the catalog listing starts.
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

        // Liveness only: G5 is enforced in object-store time before deletes.
        let classified = tokio::time::timeout(PASS_MAX_SPAN, self.classify(report, t0, shutdown))
            .await
            .map_err(|_| anyhow!("pass span exceeded {}s", PASS_MAX_SPAN.as_secs()))??;
        let Some(Classified {
            candidates,
            mut leased,
        }) = classified
        else {
            // Report mode (and a degraded delete pass) deletes no managed
            // layer, but still removes stale protocol objects so that intents
            // and leases left behind do not accumulate.
            self.remove_stale_protocol_objects(report, t0).await;
            return Ok(Completion::Done);
        };

        let deleted = self
            .delete_candidates(report, t0, candidates, &mut leased, shutdown)
            .await?;
        report.deleted_objects = deleted.len() as u64;
        report.deleted_bytes = deleted.iter().map(|layer| layer.size).sum();
        report.deleted = deleted;

        self.remove_stale_protocol_objects(report, t0).await;
        Ok(Completion::Done)
    }

    /// Best-effort duplicate-work avoidance: skip when another runner wrote a
    /// successful report of at least our mode within half an interval. A
    /// degraded delete pass counts as a report pass.
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
            let effective_mode = match summary.outcome.as_str() {
                "ok" => summary.mode.as_str(),
                "degraded" => SnapshotLayerGcMode::Report.as_str(),
                _ => continue,
            };
            if mode_rank(effective_mode) >= mode_rank(self.settings.mode.as_str()) {
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
        report.non_gating_leases = leases.non_gating;
        report.non_gating_sample = leases.non_gating_sample.clone();
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
        // G6 (first lease read and the catalog): a reader that does not gate
        // could trust a layer this pass deletes, and a record this release
        // cannot parse may name layers in a form the raw scan misses.
        if leases.non_gating > 0 {
            report.degraded = Some(format!("non_gating_leases:{}", leases.non_gating));
            return Ok(None);
        }
        if report.unparsed_records > 0 {
            report.degraded = Some(format!("unparsed_records:{}", report.unparsed_records));
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
    /// [`LEASE_TTL`] of `t0`), and which live leases do not declare `delete`
    /// mode. Aborts on any read error other than a lease deleted between
    /// listing and reading.
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
                let body = with_timeout("read managed-layer lease", store.get_object(&lease.key))
                    .await
                    .with_context(|| format!("lease '{}' read", lease.key))?;
                Ok::<_, anyhow::Error>((lease.key, body))
            })
            .buffer_unordered(LEASE_READ_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        for result in bodies {
            let (key, body) = result?;
            let Some(body) = body else {
                continue;
            };
            scan.live += 1;
            scan.digests.extend(raw_digest_tokens(&body));
            if !lease_declares_gating(&body) {
                scan.non_gating += 1;
                if scan.non_gating_sample.len() < NON_GATING_SAMPLE {
                    scan.non_gating_sample.push(key);
                }
            }
        }
        scan.non_gating_sample.sort();
        Ok(scan)
    }

    async fn delete_candidates(
        &self,
        report: &mut LayerGcReport,
        t0: SystemTime,
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
        let body = Bytes::from(serde_json::to_vec(&intent).context("serialize deletion intent")?);
        if let Err(error) = with_timeout(
            "write deletion intent",
            self.store.put_object(&intent_key, body.clone()),
        )
        .await
        {
            // The write may still land; any runner removes it once stale.
            return Err(error.context("deletion intent write"));
        }
        let intent_acked = BootInstant::now();
        let intent_written = match self.stat_intent(&intent_key).await {
            Ok(last_modified) => last_modified,
            Err(error) => {
                self.remove_intent(&intent_key).await;
                return Err(error);
            }
        };

        // G3: the final lease read starts after the intent is durable. The
        // timeout is for liveness; G4a below bounds the span in store time.
        let final_leases = match tokio::time::timeout(PRE_DELETE_BUDGET, self.read_leases(t0)).await
        {
            Ok(Ok(scan)) => scan,
            Ok(Err(error)) => {
                self.remove_intent(&intent_key).await;
                return Err(error.context("final lease read"));
            }
            Err(_) => {
                self.remove_intent(&intent_key).await;
                return Err(anyhow!(
                    "final lease read exceeded {}s",
                    PRE_DELETE_BUDGET.as_secs()
                ));
            }
        };
        // G6 (final lease read).
        if final_leases.non_gating > 0 {
            self.remove_intent(&intent_key).await;
            report.non_gating_leases = final_leases.non_gating;
            report.non_gating_sample = final_leases.non_gating_sample;
            report.degraded = Some(format!("non_gating_leases:{}", final_leases.non_gating));
            return Ok(Vec::new());
        }
        leased.extend(final_leases.digests);

        // G4a and G5 in object-store time: rewrite the intent (same bytes)
        // and compare its LastModified with the intent's and the pass start.
        let delete_phase_started = BootInstant::now();
        let rewritten = match with_timeout(
            "rewrite deletion intent",
            self.store.put_object(&intent_key, body),
        )
        .await
        {
            Ok(()) => self.stat_intent(&intent_key).await,
            Err(error) => Err(error.context("deletion intent rewrite")),
        };
        let rewritten = match rewritten {
            Ok(last_modified) => last_modified,
            Err(error) => {
                self.remove_intent(&intent_key).await;
                return Err(error);
            }
        };
        if store_span(rewritten, t0) > PASS_MAX_SPAN - STORE_TIME_PRECISION {
            self.remove_intent(&intent_key).await;
            return Err(anyhow!(
                "pass span exceeded {}s in object-store time",
                PASS_MAX_SPAN.as_secs()
            ));
        }
        if store_span(rewritten, intent_written) > PRE_DELETE_BUDGET - STORE_TIME_PRECISION {
            self.remove_intent(&intent_key).await;
            return Err(anyhow!(
                "pre-delete window exceeded {}s in object-store time",
                PRE_DELETE_BUDGET.as_secs()
            ));
        }

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

        // G4b: issue each DELETE within the delete phase on the boot clock,
        // which keeps counting while the host is suspended.
        let store = &self.store;
        let outcomes = stream::iter(to_delete)
            .map(|(digest, object)| async move {
                if delete_phase_started.elapsed() > DELETE_PHASE_BUDGET || *shutdown.borrow() {
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
                phase_secs = DELETE_PHASE_BUDGET.as_secs(),
                "snapshot layer gc delete phase closed before every delete was issued; the rest wait for the next pass"
            );
        }
        deleted.sort_by(|left, right| {
            left.last_modified_unix_ms
                .cmp(&right.last_modified_unix_ms)
                .then_with(|| left.digest.cmp(&right.digest))
        });
        if report.delete_failures == 0 {
            // Every issued DELETE has landed: nodes no longer need to wait.
            self.remove_intent(&intent_key).await;
        } else {
            // A failed or timed-out DELETE may still land later: keep the
            // intent until every landing bound has passed.
            self.hold_intent(&intent_key, intent_acked, shutdown).await;
        }
        Ok(deleted)
    }

    async fn stat_intent(&self, intent_key: &str) -> Result<SystemTime> {
        with_timeout("stat deletion intent", self.store.stat_object(intent_key))
            .await
            .context("deletion intent stat")?
            .map(|intent| intent.last_modified)
            .ok_or_else(|| anyhow!("deletion intent '{intent_key}' missing after write"))
    }

    /// Keep the intent until [`INTENT_WAIT`] after its write was
    /// acknowledged, on the boot clock, then remove it. On shutdown it is
    /// left behind for any runner's stale-intent cleanup.
    async fn hold_intent(
        &self,
        intent_key: &str,
        acked: BootInstant,
        shutdown: &watch::Receiver<bool>,
    ) {
        let mut shutdown = shutdown.clone();
        loop {
            let held = acked.elapsed();
            if held >= INTENT_WAIT {
                self.remove_intent(intent_key).await;
                return;
            }
            if sleep_or_shutdown(INTENT_WAIT - held, &mut shutdown).await {
                info!(
                    key = %intent_key,
                    "shutdown while holding a deletion intent; any runner removes it once it is stale"
                );
                return;
            }
        }
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
                "failed to delete snapshot layer gc intent; any runner removes it once it is stale"
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use bytes::Bytes;
    use serde_json::json;

    use super::super::boot_clock::advance_boot_clock_for_test;
    use super::super::layer_leases::{GatePath, LayerCheckError, LayerLeaseGuard};
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
        let leases = LayerLeases::new(Arc::clone(&dyn_store), node, MANAGED_URL, settings.mode)
            .expect("leases");
        let gc = LayerGc::new(dyn_store, Arc::clone(&leases), settings);
        (leases, gc)
    }

    fn gc(store: &Arc<FakeLayerStore>, mode: SnapshotLayerGcMode) -> LayerGc {
        gc_with(store, "gc-node", settings(mode)).1
    }

    /// A delete-mode (gating) node.
    fn node(store: &Arc<FakeLayerStore>, name: &str) -> Arc<LayerLeases> {
        LayerLeases::new(
            Arc::clone(store) as Arc<dyn LayerStore>,
            name,
            MANAGED_URL,
            SnapshotLayerGcMode::Delete,
        )
        .expect("leases")
    }

    /// Rule N1 as a gated reader runs it: hold, then gate.
    async fn gate_digests(
        leases: &Arc<LayerLeases>,
        digests: BTreeSet<String>,
        path: GatePath,
    ) -> std::result::Result<LayerLeaseGuard, LayerCheckError> {
        let guard = leases.hold(digests.clone());
        leases.gate(&digests, path).await.map(|_| guard)
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

    /// A lease written by a delete-mode (gating) node.
    fn lease_body(digests: &[usize]) -> Vec<u8> {
        let digests = digests.iter().copied().map(test_digest).collect::<Vec<_>>();
        serde_json::to_vec(&json!({ "version": 2, "mode": "delete", "digests": digests })).unwrap()
    }

    /// A lease written by a report-mode node.
    fn report_lease_body(digests: &[usize]) -> Vec<u8> {
        let digests = digests.iter().copied().map(test_digest).collect::<Vec<_>>();
        serde_json::to_vec(&json!({ "version": 2, "mode": "report", "digests": digests })).unwrap()
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
    async fn external_refs_with_managed_url_keep_layers_and_unparseable_records_degrade_delete() {
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
        let future = format!("{CATALOG_RECORDS_PREFIX}future.json");
        store.insert(
            &future,
            format!("{{\"future_format\": [\"{}\"", test_digest(3)),
        );
        for index in 1..=4 {
            put_layer(&store, index, OLD);
        }

        let report = gc(&store, SnapshotLayerGcMode::Report)
            .run_pass(&running())
            .await;
        assert_eq!(report.outcome, "ok");
        assert_eq!(report.unparsed_records, 1);
        assert_eq!(report.referenced_objects, 3);
        assert_eq!(report.garbage_sample, vec![test_digest(4)]);
        // The report runner's own lease is non-gating; drop it so the delete
        // pass below degrades for the record alone.
        for key in store.keys(LAYER_GC_LEASES_PREFIX) {
            store.remove(&key);
        }

        // G6: a record this release cannot parse degrades a delete pass.
        let delete = gc(&store, SnapshotLayerGcMode::Delete);
        let report = delete.run_pass(&running()).await;
        assert_eq!(report.outcome, "degraded", "{}", report.outcome);
        assert_eq!(report.degraded.as_deref(), Some("unparsed_records:1"));
        assert!(report.deleted.is_empty());
        assert!(position(&store.ops(), OpKind::Put, LAYER_GC_INTENTS_PREFIX).is_none());
        for index in 1..=4 {
            assert!(has_layer(&store, index));
        }

        store.remove(&future);
        let report = delete.run_pass(&running()).await;
        assert_eq!(report.outcome, "ok");
        assert_eq!(
            deleted_digests(&report),
            vec![test_digest(3), test_digest(4)]
        );
        assert!(has_layer(&store, 1) && has_layer(&store, 2));
    }

    #[tokio::test(start_paused = true)]
    async fn a_live_lease_that_does_not_declare_delete_degrades_delete_passes() {
        for (name, body) in [
            ("report-node", report_lease_body(&[])),
            (
                "version-one-node",
                serde_json::to_vec(&json!({ "version": 1, "digests": [] })).unwrap(),
            ),
            ("garbled-node", b"{ not json".to_vec()),
        ] {
            let store = Arc::new(FakeLayerStore::new());
            put_layer(&store, 1, OLD);
            let key = format!("{LAYER_GC_LEASES_PREFIX}{name}.json");
            store.insert(&key, body);
            let (_, first) = gc_with(&store, "gc-a", settings(SnapshotLayerGcMode::Delete));

            let report = first.run_pass(&running()).await;
            assert_eq!(report.outcome, "degraded", "{name}");
            assert_eq!(
                report.degraded.as_deref(),
                Some("non_gating_leases:1"),
                "{name}"
            );
            assert_eq!(report.non_gating_leases, 1, "{name}");
            assert_eq!(report.non_gating_sample, vec![key.clone()], "{name}");
            assert_eq!(report.garbage_objects, 1, "{name}: still classified");
            let ops = store.ops();
            assert!(position(&ops, OpKind::Put, LAYER_GC_INTENTS_PREFIX).is_none());
            assert!(position(&ops, OpKind::Delete, MANAGED_LAYERS_PREFIX).is_none());
            assert!(has_layer(&store, 1), "{name}");

            // A degraded run does not stand in for a delete pass: once the
            // non-gating lease is gone, the next delete runner deletes.
            store.remove(&key);
            let (_, second) = gc_with(&store, "gc-b", settings(SnapshotLayerGcMode::Delete));
            let report = second.run_pass(&running()).await;
            assert_eq!(report.outcome, "ok", "{name}");
            assert_eq!(deleted_digests(&report), vec![test_digest(1)], "{name}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_report_lease_appearing_before_the_final_lease_read_degrades_after_the_intent() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        store.set_before_op(|op: &Op, objects: &mut FakeObjects| {
            if op.kind == OpKind::Put && op.key.starts_with(LAYER_GC_INTENTS_PREFIX) {
                objects.put(
                    &format!("{LAYER_GC_LEASES_PREFIX}report-node.json"),
                    report_lease_body(&[]),
                );
            }
        });
        let report = gc(&store, SnapshotLayerGcMode::Delete)
            .run_pass(&running())
            .await;
        assert_eq!(report.outcome, "degraded", "{}", report.outcome);
        assert_eq!(report.non_gating_leases, 1);
        assert!(has_layer(&store, 1));
        assert!(position(&store.ops(), OpKind::Put, LAYER_GC_INTENTS_PREFIX).is_some());
        assert!(position(&store.ops(), OpKind::Delete, MANAGED_LAYERS_PREFIX).is_none());
        assert!(
            store.keys(LAYER_GC_INTENTS_PREFIX).is_empty(),
            "the degraded pass removes its intent"
        );
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
    async fn deletes_stop_at_the_delete_phase_bound_and_pass_aborts_beyond_p_max() {
        let store = Arc::new(FakeLayerStore::new());
        for index in 0..40 {
            put_layer(&store, 100 + index, OLD);
        }
        // Batches of 8 complete every 14 s: issued at 0 and 14 s; the third
        // batch would start at 28 s, past the 20 s delete phase.
        store.set_latency(
            OpKind::Delete,
            MANAGED_LAYERS_PREFIX,
            Duration::from_secs(14),
        );
        let gc = gc(&store, SnapshotLayerGcMode::Delete);

        let report = gc.run_pass(&running()).await;
        assert_eq!(report.outcome, "ok");
        assert_eq!(report.deleted_objects, 16);
        assert_eq!(report.delete_failures, 0);
        assert_eq!(
            report.not_issued, 24,
            "candidates past the delete phase are counted"
        );
        assert_eq!(store.keys(MANAGED_LAYERS_PREFIX).len(), 24);
        assert!(store.keys(LAYER_GC_INTENTS_PREFIX).is_empty());

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
        // Classification ends 5 s before the pass span runs out.
        store.set_latency(
            OpKind::List,
            CATALOG_RECORDS_PREFIX,
            PASS_MAX_SPAN - Duration::from_secs(5),
        );
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
        // final lease listing (within its own request timeout) outlasts the
        // remaining pass span.
        pause.reached().await;
        store.set_latency(
            OpKind::List,
            LAYER_GC_LEASES_PREFIX,
            Duration::from_secs(10),
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
                        let result = gate_digests(
                            &publication,
                            BTreeSet::from([test_digest(1)]),
                            GatePath::Publish,
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
        let guard = gate_digests(
            &publication,
            BTreeSet::from([test_digest(1)]),
            GatePath::Publish,
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
        let running_guest = gate_digests(
            &restorer,
            BTreeSet::from([test_digest(1)]),
            GatePath::Restore,
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
        let error = gate_digests(
            &restorer,
            BTreeSet::from([test_digest(2)]),
            GatePath::Restore,
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
            let guard = gate_digests(
                &publication,
                BTreeSet::from([test_digest(1)]),
                GatePath::Publish,
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
    async fn crash_after_intent_write_forces_slow_path_until_a_runner_removes_the_stale_intent() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 5, OLD);
        put_intent(&store, "crashed-runner-pass", &[5], Duration::ZERO);

        let started = Instant::now();
        let _first = gate_digests(
            &node(&store, "node-a"),
            BTreeSet::from([test_digest(5)]),
            GatePath::Restore,
        )
        .await
        .expect("gate");
        assert!(
            started.elapsed() >= INTENT_WAIT,
            "a live intent forces the slow path"
        );

        // Nodes do not age intents: an old one still forces the wait until a
        // runner's hygiene removes it.
        store.advance_s3_clock(STALE_INTENT_AGE + Duration::from_secs(60));
        let started = Instant::now();
        let _second = gate_digests(
            &node(&store, "node-b"),
            BTreeSet::from([test_digest(5)]),
            GatePath::Restore,
        )
        .await
        .expect("gate");
        assert!(started.elapsed() >= INTENT_WAIT);

        let report = gc(&store, SnapshotLayerGcMode::Report)
            .run_pass(&running())
            .await;
        assert_eq!(report.stale_intents_removed, 1);
        assert!(store.keys(LAYER_GC_INTENTS_PREFIX).is_empty());
        let started = Instant::now();
        let _third = gate_digests(
            &node(&store, "node-c"),
            BTreeSet::from([test_digest(5)]),
            GatePath::Restore,
        )
        .await
        .expect("gate");
        assert!(started.elapsed() < INTENT_WAIT, "fast again once removed");
    }

    #[tokio::test(start_paused = true)]
    async fn pass_span_is_bounded_in_object_store_time_when_the_runner_is_paused() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        let gc = Arc::new(gc(&store, SnapshotLayerGcMode::Delete));
        let mut pause = store.pause(OpKind::List, MANAGED_LAYERS_PREFIX);
        let pass = tokio::spawn({
            let gc = Arc::clone(&gc);
            async move { gc.run_pass(&running()).await }
        });
        // The runner's VM is paused for 31 minutes during classification:
        // object-store time moves on, its own clocks do not.
        pause.reached().await;
        store.advance_s3_clock(Duration::from_secs(31 * 60));
        pause.release();
        let report = pass.await.unwrap();

        assert!(
            report.outcome.contains("pass span exceeded"),
            "{}",
            report.outcome
        );
        assert!(has_layer(&store, 1));
        assert!(position(&store.ops(), OpKind::Delete, MANAGED_LAYERS_PREFIX).is_none());
        assert!(position(&store.ops(), OpKind::Put, LAYER_GC_INTENTS_PREFIX).is_some());
        assert!(store.keys(LAYER_GC_INTENTS_PREFIX).is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn a_pause_between_the_final_lease_read_and_the_rewrite_aborts_before_deleting() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        let gc = Arc::new(gc(&store, SnapshotLayerGcMode::Delete));
        let mut first = store.pause(OpKind::Put, LAYER_GC_INTENTS_PREFIX);
        let pass = tokio::spawn({
            let gc = Arc::clone(&gc);
            async move { gc.run_pass(&running()).await }
        });
        first.reached().await;
        let mut rewrite = store.pause(OpKind::Put, LAYER_GC_INTENTS_PREFIX);
        first.release();
        rewrite.reached().await;
        store.advance_s3_clock(Duration::from_secs(30));
        rewrite.release();
        let report = pass.await.unwrap();

        assert!(
            report.outcome.contains("pre-delete window exceeded"),
            "{}",
            report.outcome
        );
        assert!(has_layer(&store, 1));
        assert!(position(&store.ops(), OpKind::Delete, MANAGED_LAYERS_PREFIX).is_none());
        assert!(store.keys(LAYER_GC_INTENTS_PREFIX).is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn a_host_suspend_in_the_delete_phase_leaves_every_candidate_not_issued() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        put_layer(&store, 2, OLD);
        let gc = Arc::new(gc(&store, SnapshotLayerGcMode::Delete));
        let mut first = store.pause(OpKind::Stat, LAYER_GC_INTENTS_PREFIX);
        let pass = tokio::spawn({
            let gc = Arc::clone(&gc);
            async move { gc.run_pass(&running()).await }
        });
        first.reached().await;
        let mut second = store.pause(OpKind::Stat, LAYER_GC_INTENTS_PREFIX);
        first.release();
        second.reached().await;
        // The host suspends right after the rewrite: the boot clock and
        // object-store time move on, the monotonic clock does not.
        let suspend = Duration::from_secs(3 * 60);
        advance_boot_clock_for_test(suspend);
        store.advance_s3_clock(suspend);
        second.release();
        let report = pass.await.unwrap();

        assert_eq!(report.outcome, "ok", "{}", report.outcome);
        assert_eq!(report.not_issued, 2);
        assert!(report.deleted.is_empty());
        assert!(has_layer(&store, 1) && has_layer(&store, 2));
        assert!(store.keys(LAYER_GC_INTENTS_PREFIX).is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_delete_holds_the_intent_for_the_deletion_window_unless_shutting_down() {
        let store = Arc::new(FakeLayerStore::new());
        put_layer(&store, 1, OLD);
        put_layer(&store, 2, OLD);
        store.fail(OpKind::Delete, &layer_key(1));
        let gc = Arc::new(gc(&store, SnapshotLayerGcMode::Delete));

        let started = Instant::now();
        let report = gc.run_pass(&running()).await;
        assert_eq!(report.outcome, "ok");
        assert_eq!(report.delete_failures, 1);
        assert_eq!(deleted_digests(&report), vec![test_digest(2)]);
        assert!(started.elapsed() >= INTENT_WAIT, "the intent was held");
        assert!(store.keys(LAYER_GC_INTENTS_PREFIX).is_empty());

        // Shutdown during the hold leaves the intent for stale cleanup.
        let (stop, shutdown) = watch::channel(false);
        let pass = tokio::spawn({
            let gc = Arc::clone(&gc);
            async move { gc.run_pass(&shutdown).await }
        });
        tokio::time::sleep(Duration::from_secs(30)).await;
        stop.send(true).unwrap();
        let report = pass.await.unwrap();
        assert_eq!(report.delete_failures, 1);
        assert_eq!(store.keys(LAYER_GC_INTENTS_PREFIX).len(), 1);
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

    fn track(tracker: &Mutex<Tracker>, digests: &BTreeSet<String>, held: bool) {
        let mut tracker = tracker.lock().unwrap();
        for digest in digests {
            if held {
                *tracker.held.entry(digest.clone()).or_default() += 1;
            } else {
                let count = tracker.held.get_mut(digest).expect("held");
                *count -= 1;
                if *count == 0 {
                    tracker.held.remove(digest);
                }
            }
        }
    }

    /// One randomized run: two delete-mode runners, gated restores and
    /// deduplicating publications on two nodes, record deletes, a VM pause of
    /// the runners (object-store time jumps, process clocks do not) and a
    /// host suspend (object-store time and the boot clock jump, the
    /// monotonic clock does not). With `report_reader`, a report-mode node
    /// with a live lease also restores and trusts layers without gating.
    async fn run_seed(seed: u64, report_reader: bool) -> Tracker {
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
        let reader = report_reader.then(|| {
            LayerLeases::new(
                Arc::clone(&store) as Arc<dyn LayerStore>,
                "report-reader",
                MANAGED_URL,
                SnapshotLayerGcMode::Report,
            )
            .expect("leases")
        });
        if let Some(reader) = &reader {
            assert!(reader.maintain().await, "report lease written");
        }

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
            let Ok(guard) = gate_digests(leases, digests.clone(), GatePath::Restore).await else {
                return;
            };
            track(shared_tracker, &digests, true);
            random_sleep(rng, 400).await;
            track(shared_tracker, &digests, false);
            drop(guard);
        };
        let reader = &reader;
        let report_restore = move || async move {
            let Some(reader) = reader.as_ref() else {
                return;
            };
            random_sleep(rng, 300).await;
            let record = format!("{CATALOG_RECORDS_PREFIX}r{}.json", pick(rng, 2));
            let Ok(Some(body)) = shared_store.get_object(&record).await else {
                return;
            };
            let digests = raw_digest_tokens(&body);
            // Report mode holds without gating; it trusts what it sees.
            let guard = reader.hold(digests.clone());
            reader.maintain().await;
            for digest in &digests {
                let key = OssSnapshotArtifactLayout::managed_layer_key(digest);
                if !matches!(shared_store.stat_object(&key).await, Ok(Some(_))) {
                    return;
                }
            }
            track(shared_tracker, &digests, true);
            random_sleep(rng, 400).await;
            track(shared_tracker, &digests, false);
            drop(guard);
        };
        let pause_runner_vm = move || async move {
            random_sleep(rng, 400).await;
            shared_store.advance_s3_clock(Duration::from_secs(pick(rng, 45) * 60));
        };
        let suspend_host = move || async move {
            random_sleep(rng, 400).await;
            let by = Duration::from_secs(pick(rng, 600));
            advance_boot_clock_for_test(by);
            shared_store.advance_s3_clock(by);
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
                match gate_digests(leases, digests.clone(), GatePath::Publish).await {
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
            report_restore(),
            pause_runner_vm(),
            suspend_host(),
        );

        // One more operation runs the invariant check on the final state.
        let _ = store.list_objects(MANAGED_LAYERS_PREFIX).await;
        let mut final_state = tracker.lock().unwrap();
        std::mem::take(&mut *final_state)
    }

    fn run_seed_paused(seed: u64, report_reader: bool) -> Tracker {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()
            .unwrap();
        let tracker = runtime.block_on(run_seed(seed, report_reader));
        assert!(
            tracker.violations.is_empty(),
            "seed {seed}: {:?}",
            tracker.violations
        );
        tracker
    }

    #[test]
    fn randomized_interleavings_never_delete_a_needed_layer() {
        let deleted = (0..200u64)
            .map(|seed| run_seed_paused(seed, false).deleted)
            .sum::<usize>();
        assert!(deleted > 0, "the runs exercised deletions");
    }

    #[test]
    fn randomized_interleavings_with_a_report_mode_reader_degrade_every_delete_pass() {
        for seed in 0..50u64 {
            assert_eq!(
                run_seed_paused(seed, true).deleted,
                0,
                "seed {seed}: a live report-mode lease blocks deletes"
            );
        }
    }
}
