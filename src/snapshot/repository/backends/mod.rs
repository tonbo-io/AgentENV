pub(crate) mod common;
pub(crate) mod oss;
pub(crate) mod posixfs;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::cfg::{ConfigManager, SnapshotImageStoragePolicy, SnapshotRepositoryBackendKind};
use crate::identity::NodeIdentity;
use crate::image::cache::local_image_services_from_app_config;
use crate::p2p::P2pTransport;
use crate::snapshot::artifact_cache::LocalArtifactCache;
use crate::snapshot::repository::interfaces::{SnapshotRepository, SnapshotRuntimeResolver};
pub use oss::OssBackend;
pub use posixfs::{PosixFsBackend, PosixFsBackendConfig};

/// A configured snapshot backend: repository, runtime resolver and, for the
/// OSS backend, its managed-layer lease and GC maintenance.
pub struct SnapshotBackendParts {
    pub repository: Arc<dyn SnapshotRepository>,
    pub runtime_resolver: Arc<dyn SnapshotRuntimeResolver>,
    pub(crate) layer_maintenance: Option<oss::OssLayerMaintenance>,
}

/// Builds the configured snapshot repository backend and its matching runtime resolver from the global configuration.
pub fn build_snapshot_backend(
    p2p_transport: Option<Arc<dyn P2pTransport>>,
) -> Result<SnapshotBackendParts> {
    let config = ConfigManager::global_config();
    let shared_cache_root = shared_runtime_cache_root();
    let overlaybd_layers = local_image_services_from_app_config(config).overlaybd_layers;
    match config.snapshot.repository_backend {
        SnapshotRepositoryBackendKind::PosixFs => {
            let root = config
                .backend
                .posix_fs
                .as_ref()
                .context("backend.posix_fs config is required when repository_backend = posix_fs")?
                .snapshot_store
                .join("repository");
            let cache = LocalArtifactCache::new(shared_cache_root.clone(), None)?;
            let (repository, runtime_resolver) = PosixFsBackend::from_parts(
                PosixFsBackendConfig {
                    root,
                    cache_root: Some(shared_cache_root.clone()),
                    runtime_cache_root: Some(shared_cache_root.join("runtime")),
                },
                overlaybd_layers,
                cache,
            )
            .into_parts();
            Ok(SnapshotBackendParts {
                repository,
                runtime_resolver,
                layer_maintenance: None,
            })
        }
        SnapshotRepositoryBackendKind::Oss => {
            let oss_config = config
                .backend
                .oss
                .as_ref()
                .context("backend.oss config is required when repository_backend = oss")?;
            let snapshot_image_storage = if config.snapshot.image_publish.enabled {
                SnapshotImageStoragePolicy::SourceRegistry
            } else {
                SnapshotImageStoragePolicy::ObjectStorage
            };
            let cache =
                LocalArtifactCache::new(shared_cache_root.clone(), oss_config.cache_max_size_gb)?;
            let node_id = NodeIdentity::from_config(&config.node_identity).id;
            let backend = OssBackend::from_parts(
                oss_config,
                snapshot_image_storage,
                &config.snapshot.publish_compression,
                &config.snapshot.layer_gc,
                &node_id,
                cache,
                shared_cache_root.join("runtime"),
                overlaybd_layers,
                p2p_transport,
            )?;
            let layer_maintenance = Some(backend.layer_maintenance());
            let (repository, runtime_resolver) = backend.into_parts();
            Ok(SnapshotBackendParts {
                repository,
                runtime_resolver,
                layer_maintenance,
            })
        }
    }
}

pub(crate) fn shared_runtime_cache_root() -> PathBuf {
    ConfigManager::global_config()
        .snapshot
        .local_cache_path
        .clone()
}
