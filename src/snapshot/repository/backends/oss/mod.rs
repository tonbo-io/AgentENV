mod boot_clock;
mod client;
mod config;
mod layer_gc;
mod layer_leases;
mod layer_refs;
mod layer_store;
mod layout;
mod repository;
mod resolver;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::watch;

use crate::cfg::{
    OssBackendConfig, SnapshotImageStoragePolicy, SnapshotLayerGcConfig, SnapshotLayerGcMode,
    SnapshotPublishCompressionConfig,
};
use crate::image::cache::{local_image_services_from_global_config, OverlaybdLayerStore};
use crate::p2p::P2pTransport;
use crate::snapshot::artifact_cache::LocalArtifactCache;
use crate::snapshot::repository::interfaces::{
    SnapshotLayerRetention, SnapshotRepository, SnapshotRuntimeResolver,
};

pub(crate) use self::client::OssClient;
pub(crate) use self::config::NormalizedOssConfig;
use self::layer_gc::{LayerGc, LayerGcSettings};
use self::layer_leases::LayerLeases;
use self::layer_store::LayerStore;
pub(crate) use self::layout::OssSnapshotArtifactLayout;
pub(crate) use self::repository::OssSnapshotRepository;
use self::resolver::OssRuntimeResolver;

/// OSS-backed snapshot backend.
///
/// Combines the durable committed-state repository (stored in OSS) with a
/// node-local runtime resolver that materializes runnable overlaybd configs.
pub struct OssBackend {
    repository: Arc<dyn SnapshotRepository>,
    runtime_resolver: Arc<dyn SnapshotRuntimeResolver>,
    /// `None` in `[snapshot.layer_gc].mode = "off"`: the process takes no part
    /// in the managed-layer GC protocol.
    layers: Option<OssLayerMaintenance>,
}

/// Managed-layer leases and GC of one OSS-backed node process in `report` or
/// `delete` mode.
#[derive(Clone)]
pub(crate) struct OssLayerMaintenance {
    leases: Arc<LayerLeases>,
    gc: Arc<LayerGc>,
}

impl OssLayerMaintenance {
    pub(crate) fn retention(&self) -> Arc<dyn SnapshotLayerRetention> {
        Arc::clone(&self.leases) as Arc<dyn SnapshotLayerRetention>
    }

    /// Spawn the lease refresher and the GC loop. Both stop when `shutdown`
    /// turns true; nothing waits for them.
    pub(crate) fn start(&self, shutdown: watch::Receiver<bool>) {
        tokio::spawn(Arc::clone(&self.leases).run_refresher(shutdown.clone()));
        tokio::spawn(Arc::clone(&self.gc).run(shutdown));
    }
}

/// Runs managed-layer GC passes on demand against a backend's object store,
/// sharing that backend's node lease. For integration tests against a real
/// S3-compatible store only: production passes run from the GC loop with
/// validated settings.
#[doc(hidden)]
pub struct OssLayerGcProbe {
    leases: Arc<LayerLeases>,
}

/// Outcome of one probe pass (a subset of the run report).
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct OssLayerGcPassSummary {
    /// `ok`, `degraded`, `skipped` or `aborted:<reason>`.
    pub outcome: String,
    /// Why a delete pass ran as a report, if it did.
    pub degraded: Option<String>,
    pub referenced_objects: u64,
    pub leased_only_objects: u64,
    pub young_objects: u64,
    pub garbage_objects: u64,
    pub non_gating_leases: u64,
    pub garbage_sample: Vec<String>,
    /// Digests deleted by the pass.
    pub deleted: Vec<String>,
}

impl OssLayerGcProbe {
    /// Write this backend's lease now, so a pass sees what it holds.
    pub async fn flush_lease(&self) -> Result<()> {
        self.leases.refresh_now().await.map(|_| ())
    }

    /// Run one pass in `mode` with `grace` as the minimum object age.
    pub async fn run_pass(
        &self,
        mode: SnapshotLayerGcMode,
        grace: std::time::Duration,
    ) -> OssLayerGcPassSummary {
        let gc = LayerGc::new(
            Arc::clone(self.leases.store()),
            Arc::clone(&self.leases),
            LayerGcSettings {
                mode,
                interval: std::time::Duration::from_secs(3600),
                grace,
                max_deletes_per_pass: 1000,
            },
        );
        let report = gc.run_pass(&watch::channel(false).1).await;
        OssLayerGcPassSummary {
            outcome: report.outcome,
            degraded: report.degraded,
            referenced_objects: report.referenced_objects,
            leased_only_objects: report.leased_only_objects,
            young_objects: report.young_objects,
            garbage_objects: report.garbage_objects,
            non_gating_leases: report.non_gating_leases,
            garbage_sample: report.garbage_sample,
            deleted: report
                .deleted
                .into_iter()
                .map(|layer| layer.digest)
                .collect(),
        }
    }
}

impl OssBackend {
    /// GC probe sharing this backend's lease; see [`OssLayerGcProbe`].
    /// `None` in `off` mode, where the backend has no lease.
    #[doc(hidden)]
    pub fn layer_gc_probe(&self) -> Option<OssLayerGcProbe> {
        self.layers.as_ref().map(|layers| OssLayerGcProbe {
            leases: Arc::clone(&layers.leases),
        })
    }

    /// Build the OSS backend from config.
    ///
    /// This convenience constructor remains available for tests and direct
    /// callers. The main backend factory constructs a shared cache once and
    /// uses [`OssBackend::from_parts`] instead. Publish compression stays
    /// disabled and managed-layer GC `off` here; use
    /// [`OssBackend::new_with_publish_compression`] to exercise
    /// `[snapshot.publish_compression]` or `[snapshot.layer_gc]`.
    pub fn new(config: &OssBackendConfig, cache_root: PathBuf) -> Result<Self> {
        Self::new_with_publish_compression(
            config,
            cache_root,
            &SnapshotPublishCompressionConfig::default(),
            &SnapshotLayerGcConfig::default(),
        )
    }

    /// Build the OSS backend from config with explicit publish-compression
    /// and managed-layer GC settings, for tests and direct callers that
    /// cannot go through the global config.
    pub fn new_with_publish_compression(
        config: &OssBackendConfig,
        cache_root: PathBuf,
        publish_compression: &SnapshotPublishCompressionConfig,
        layer_gc: &SnapshotLayerGcConfig,
    ) -> Result<Self> {
        let cache = LocalArtifactCache::new(cache_root.clone(), config.cache_max_size_gb)?;
        Self::from_parts(
            config,
            SnapshotImageStoragePolicy::default(),
            publish_compression,
            layer_gc,
            "local",
            cache,
            cache_root.join("runtime"),
            local_image_services_from_global_config().overlaybd_layers,
            None,
        )
    }

    /// Build the OSS backend from config plus a shared node-local cache.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        config: &OssBackendConfig,
        snapshot_image_storage: SnapshotImageStoragePolicy,
        publish_compression: &SnapshotPublishCompressionConfig,
        layer_gc: &SnapshotLayerGcConfig,
        node_id: &str,
        cache: Arc<LocalArtifactCache>,
        runtime_root: PathBuf,
        store: Arc<dyn OverlaybdLayerStore>,
        p2p_transport: Option<Arc<dyn P2pTransport>>,
    ) -> Result<Self> {
        let config = NormalizedOssConfig::new(config, snapshot_image_storage)?;
        let managed_layers_repo_blob_url = config.managed_layers_repo_blob_url();
        let client = Arc::new(OssClient::new(
            config.bucket().to_string(),
            config.endpoint().to_string(),
            config.region().to_string(),
            config.prefix().to_string(),
            config.credential_source(),
            config.addressing_style(),
        )?);

        // Off takes no part in the protocol: no lease, no GC, no request
        // under `layer-gc/`, and serving paths run exactly as without GC.
        let layers = match layer_gc.mode {
            SnapshotLayerGcMode::Off => None,
            SnapshotLayerGcMode::Report | SnapshotLayerGcMode::Delete => {
                let layer_store = Arc::clone(&client) as Arc<dyn LayerStore>;
                let leases = LayerLeases::new(
                    Arc::clone(&layer_store),
                    node_id,
                    &managed_layers_repo_blob_url,
                    layer_gc.mode,
                )?;
                let gc = Arc::new(LayerGc::new(
                    layer_store,
                    Arc::clone(&leases),
                    LayerGcSettings::from_config(layer_gc),
                ));
                Some(OssLayerMaintenance { leases, gc })
            }
        };
        let leases = layers.as_ref().map(|layers| Arc::clone(&layers.leases));

        let mut repository = OssSnapshotRepository::new(
            Arc::clone(&client),
            config.snapshot_image_storage(),
            publish_compression,
        );
        if let Some(leases) = &leases {
            repository = repository.with_layer_leases(Arc::clone(leases));
        }
        let repository: Arc<dyn SnapshotRepository> = Arc::new(repository);

        std::fs::create_dir_all(&runtime_root)
            .with_context(|| format!("create oss runtime root '{}'", runtime_root.display()))?;
        let runtime_resolver: Arc<dyn SnapshotRuntimeResolver> = Arc::new(OssRuntimeResolver::new(
            client,
            cache,
            runtime_root,
            store,
            managed_layers_repo_blob_url,
            p2p_transport,
            leases,
        )?);

        Ok(Self {
            repository,
            runtime_resolver,
            layers,
        })
    }

    /// Managed-layer lease and GC handle of this backend; `None` in `off`
    /// mode.
    pub(crate) fn layer_maintenance(&self) -> Option<OssLayerMaintenance> {
        self.layers.clone()
    }

    /// Splits the backend into its repository and runtime-resolution components.
    pub fn into_parts(
        self,
    ) -> (
        Arc<dyn SnapshotRepository>,
        Arc<dyn SnapshotRuntimeResolver>,
    ) {
        (self.repository, self.runtime_resolver)
    }
}
