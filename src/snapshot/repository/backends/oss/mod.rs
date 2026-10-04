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
use tokio::task::JoinHandle;

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
    layers: OssLayerMaintenance,
}

/// Managed-layer leases and GC of one OSS-backed node process.
///
/// Leases are always maintained: protocol participation must not depend on
/// per-node configuration. The GC loop runs only when
/// `[snapshot.layer_gc].mode` is not `off`.
#[derive(Clone)]
pub(crate) struct OssLayerMaintenance {
    leases: Arc<LayerLeases>,
    gc: Arc<LayerGc>,
}

impl OssLayerMaintenance {
    pub(crate) fn retention(&self) -> Arc<dyn SnapshotLayerRetention> {
        Arc::clone(&self.leases) as Arc<dyn SnapshotLayerRetention>
    }

    /// Spawn the lease refresher and, unless GC is off, the GC loop. Both stop
    /// when `shutdown` turns true; await the handles to let in-flight deletes
    /// finish.
    pub(crate) fn start(&self, shutdown: watch::Receiver<bool>) -> Vec<JoinHandle<()>> {
        let mut tasks = vec![tokio::spawn(
            Arc::clone(&self.leases).run_refresher(shutdown.clone()),
        )];
        if self.gc.mode() != SnapshotLayerGcMode::Off {
            tasks.push(tokio::spawn(Arc::clone(&self.gc).run(shutdown)));
        }
        tasks
    }
}

impl OssBackend {
    /// Build the OSS backend from config.
    ///
    /// This convenience constructor remains available for tests and direct
    /// callers. The main backend factory constructs a shared cache once and
    /// uses [`OssBackend::from_parts`] instead. Publish compression stays
    /// disabled here; use [`OssBackend::new_with_publish_compression`] to
    /// exercise `[snapshot.publish_compression]`.
    pub fn new(config: &OssBackendConfig, cache_root: PathBuf) -> Result<Self> {
        Self::new_with_publish_compression(
            config,
            cache_root,
            &SnapshotPublishCompressionConfig::default(),
        )
    }

    /// Build the OSS backend from config with explicit publish-compression
    /// settings, for tests and direct callers that cannot go through the
    /// global config.
    pub fn new_with_publish_compression(
        config: &OssBackendConfig,
        cache_root: PathBuf,
        publish_compression: &SnapshotPublishCompressionConfig,
    ) -> Result<Self> {
        let cache = LocalArtifactCache::new(cache_root.clone(), config.cache_max_size_gb)?;
        Self::from_parts(
            config,
            SnapshotImageStoragePolicy::default(),
            publish_compression,
            &SnapshotLayerGcConfig::default(),
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

        let layer_store = Arc::clone(&client) as Arc<dyn LayerStore>;
        let leases = LayerLeases::new(Arc::clone(&layer_store), node_id);
        let gc = Arc::new(LayerGc::new(
            layer_store,
            Arc::clone(&leases),
            LayerGcSettings::from_config(layer_gc),
        ));

        let repository: Arc<dyn SnapshotRepository> = Arc::new(
            OssSnapshotRepository::new(
                Arc::clone(&client),
                config.snapshot_image_storage(),
                publish_compression,
            )
            .with_layer_leases(Arc::clone(&leases)),
        );

        std::fs::create_dir_all(&runtime_root)
            .with_context(|| format!("create oss runtime root '{}'", runtime_root.display()))?;
        let runtime_resolver: Arc<dyn SnapshotRuntimeResolver> = Arc::new(OssRuntimeResolver::new(
            client,
            cache,
            runtime_root,
            store,
            managed_layers_repo_blob_url,
            p2p_transport,
            Arc::clone(&leases),
        )?);

        Ok(Self {
            repository,
            runtime_resolver,
            layers: OssLayerMaintenance { leases, gc },
        })
    }

    /// Managed-layer lease and GC handle of this backend.
    pub(crate) fn layer_maintenance(&self) -> OssLayerMaintenance {
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
