use crate::snapshot::SnapshotId;

/// Prefix of the content-addressed managed layers shared across snapshots.
pub(crate) const MANAGED_LAYERS_PREFIX: &str = "managed-layers/";
/// Prefix of the committed and pending snapshot records.
pub(crate) const CATALOG_RECORDS_PREFIX: &str = "catalog/records/";
/// Per-process managed-layer leases written by every OSS-backed node.
pub(crate) const LAYER_GC_LEASES_PREFIX: &str = "layer-gc/leases/";
/// Per-pass deletion intents published by managed-layer GC runners.
pub(crate) const LAYER_GC_INTENTS_PREFIX: &str = "layer-gc/intents/";
/// Last run report of each managed-layer GC runner.
pub(crate) const LAYER_GC_RUNS_PREFIX: &str = "layer-gc/runs/";

/// Committed object layout for the OSS snapshot backend.
pub(crate) struct OssSnapshotArtifactLayout<'a> {
    snapshot_id: &'a SnapshotId,
}

impl<'a> OssSnapshotArtifactLayout<'a> {
    pub(super) fn new(snapshot_id: &'a SnapshotId) -> Self {
        Self { snapshot_id }
    }

    pub(super) fn alias_key(alias: &str) -> String {
        format!("catalog/aliases/{alias}.json")
    }

    pub(super) fn record_key(id: &SnapshotId) -> String {
        format!("{CATALOG_RECORDS_PREFIX}{id}.json")
    }

    pub(crate) fn managed_layer_key(digest: &str) -> String {
        format!("{MANAGED_LAYERS_PREFIX}{digest}")
    }

    pub(super) fn artifact_prefix(&self) -> String {
        format!("artifacts/{}/", self.snapshot_id)
    }

    pub(super) fn artifact_key(&self, relative_path: &str) -> String {
        format!("{}{}", self.artifact_prefix(), relative_path)
    }

    /// Lease object of one node process (`owner` is `{node}-{instance}`).
    pub(super) fn layer_lease_key(owner: &str, generation: u32) -> String {
        if generation == 0 {
            format!("{LAYER_GC_LEASES_PREFIX}{owner}.json")
        } else {
            format!("{LAYER_GC_LEASES_PREFIX}{owner}-g{generation}.json")
        }
    }

    /// Deletion intent of one GC pass.
    pub(super) fn layer_gc_intent_key(runner: &str, pass_id: &str) -> String {
        format!("{LAYER_GC_INTENTS_PREFIX}{runner}-{pass_id}.json")
    }

    /// Run report of one GC runner, overwritten every pass.
    pub(super) fn layer_gc_run_key(runner: &str) -> String {
        format!("{LAYER_GC_RUNS_PREFIX}{runner}.json")
    }
}
