use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use futures::{stream, StreamExt, TryStreamExt};
use overlaybd::config::{DownloadConfig, LayerConfig};
use tracing::{debug, info};

use super::client::OssClient;
use super::layer_leases::{LayerLeaseGuard, LayerLeases, ProtectPath};
use super::layer_refs::{committed_managed_layer_digests, managed_layer_ref_digest};
use super::layout::OssSnapshotArtifactLayout;
use crate::image::cache::OverlaybdLayerStore;
use crate::p2p::P2pTransport;
use crate::snapshot::artifact_cache::{CacheArtifactLease, CacheHandle, LocalArtifactCache};
use crate::snapshot::p2p;
use crate::snapshot::repository::interfaces::SnapshotRuntimeResolver;
use crate::snapshot::runtime_support::{
    hydrate_runtime_manifest, materialize_image_config_error, parse_firecracker_manifest,
    runtime_image_cache_key, validate_managed_artifact, RuntimeImageMaterializer,
};
use crate::snapshot::types::RuntimeArtifactLease;
use crate::snapshot::{
    CommittedAttachedDrive, CommittedSnapshot, OverlaybdLayerRef, RepositoryError,
    RepositoryResult, ResolvedAttachedDrive, RunnableSnapshot, SnapshotId, SnapshotRecord,
    SNAPSHOT_ARTIFACT_LAYOUT,
};

const MANAGED_LAYER_EXISTS_CONCURRENCY: usize = 16;

struct MaterializeSpec<'a> {
    label: &'a str,
    cache_key: &'a str,
    allow_empty_layers: bool,
    download: Option<DownloadConfig>,
    /// The resolve already checked every managed layer under its lease.
    layers_checked: bool,
}

/// Keeps a resolved snapshot's node-local cache entries and its managed-layer
/// lease alive for as long as the [`RunnableSnapshot`] lives.
struct OssRuntimeLease {
    _cache: CacheArtifactLease,
    _layers: LayerLeaseGuard,
}

impl RuntimeArtifactLease for OssRuntimeLease {}

/// Indexed managed-layer digests of every subject a committed snapshot reads
/// from this repository's `managed-layers/`, labelled like materialization
/// errors.
fn managed_layer_checks(
    committed: &CommittedSnapshot,
    managed_layers_repo_blob_url: &str,
) -> Vec<(String, Vec<(usize, String)>)> {
    let refs = |layers: &[OverlaybdLayerRef]| {
        layers
            .iter()
            .enumerate()
            .filter_map(|(index, layer)| {
                managed_layer_ref_digest(layer, managed_layers_repo_blob_url)
                    .map(|digest| (index, digest.to_string()))
            })
            .collect::<Vec<_>>()
    };
    let mut checks = vec![
        ("rootfs".to_string(), refs(&committed.rootfs_layers)),
        (
            "memory".to_string(),
            committed
                .memory_layers
                .iter()
                .enumerate()
                .map(|(index, layer)| (index, layer.digest.clone()))
                .collect(),
        ),
    ];
    for drive in &committed.attached_drives {
        match drive {
            CommittedAttachedDrive::Overlaybd {
                drive_id, layers, ..
            } => checks.push((format!("drive '{drive_id}'"), refs(layers))),
        }
    }
    if let Some(tools_drive) = &committed.tools_drive {
        checks.push((
            "tools drive".to_string(),
            vec![(0, tools_drive.digest.clone())],
        ));
    }
    checks.retain(|(_, layers)| !layers.is_empty());
    checks
}

/// Lease every managed layer a committed snapshot reads before trusting that
/// any of them exists (GC rule N1), then check them unless they were already
/// checked while continuously leased. Returns the guard and whether this call
/// checked every layer.
async fn protect_snapshot_layers(
    leases: &Arc<LayerLeases>,
    id: &SnapshotId,
    committed: &CommittedSnapshot,
    managed_layers_repo_blob_url: &str,
) -> RepositoryResult<(LayerLeaseGuard, bool)> {
    let digests = committed_managed_layer_digests(committed, managed_layers_repo_blob_url);
    let mut guard = leases
        .protect(digests, ProtectPath::Restore)
        .await
        .map_err(|error| {
            RepositoryError::backend(format!("lease managed layers of snapshot '{id}'"), error)
        })?;
    if !guard.needs_verification() {
        return Ok((guard, false));
    }
    let checks = managed_layer_checks(committed, managed_layers_repo_blob_url);
    futures::future::try_join_all(checks.iter().map(|(label, layers)| {
        let store = Arc::clone(leases.store());
        validate_managed_layers(layers, label, move |key| {
            let store = Arc::clone(&store);
            async move { Ok(store.stat_object(&key).await?.is_some()) }
        })
    }))
    .await?;
    guard.mark_verified();
    Ok((guard, true))
}

async fn validate_managed_layers<F, Fut>(
    layers: &[(usize, String)],
    label: &str,
    exists: F,
) -> RepositoryResult<()>
where
    F: Fn(String) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<bool>> + Send,
{
    stream::iter(layers.iter().cloned())
        .map(|(index, digest)| {
            let exists = exists.clone();
            async move {
                let key = OssSnapshotArtifactLayout::managed_layer_key(&digest);
                let present = exists(key.clone()).await.map_err(|error| {
                    RepositoryError::backend(format!("check managed layer '{key}'"), error)
                })?;
                if !present {
                    return Err(RepositoryError::ArtifactNotFound {
                        artifact: format!("{label}: managed layer {index} '{digest}' is missing"),
                    });
                }
                Ok(())
            }
        })
        .buffer_unordered(MANAGED_LAYER_EXISTS_CONCURRENCY)
        .try_for_each(|()| async { Ok(()) })
        .await
}

/// Resolves committed OSS-backed snapshots into node-local runnable paths.
pub(crate) struct OssRuntimeResolver {
    client: Arc<OssClient>,
    cache: Arc<LocalArtifactCache>,
    image_materializer: RuntimeImageMaterializer,
    managed_layers_repo_blob_url: String,
    p2p_transport: Option<Arc<dyn P2pTransport>>,
    leases: Arc<LayerLeases>,
}

impl OssRuntimeResolver {
    fn layout<'a>(&self, id: &'a SnapshotId) -> OssSnapshotArtifactLayout<'a> {
        OssSnapshotArtifactLayout::new(id)
    }

    pub(crate) fn new(
        client: Arc<OssClient>,
        cache: Arc<LocalArtifactCache>,
        runtime_root: PathBuf,
        store: Arc<dyn OverlaybdLayerStore>,
        managed_layers_repo_blob_url: String,
        p2p_transport: Option<Arc<dyn P2pTransport>>,
        leases: Arc<LayerLeases>,
    ) -> RepositoryResult<Self> {
        Ok(Self {
            client,
            cache,
            image_materializer: RuntimeImageMaterializer::new(runtime_root, store),
            managed_layers_repo_blob_url,
            p2p_transport,
            leases,
        })
    }

    /// Record which managed layers each materialized runtime image config
    /// names, so the node lease keeps covering them after the evictable cache
    /// file is gone.
    fn record_image_configs(&self, digests: &std::collections::BTreeSet<String>, paths: &[&Path]) {
        for path in paths {
            self.leases
                .record_image_config(path.to_path_buf(), digests.clone());
        }
    }
}

#[async_trait]
impl SnapshotRuntimeResolver for OssRuntimeResolver {
    async fn resolve(&self, snapshot: Arc<SnapshotRecord>) -> RepositoryResult<RunnableSnapshot> {
        let id = snapshot.id.clone();
        let committed =
            snapshot
                .committed
                .as_ref()
                .ok_or_else(|| RepositoryError::InvalidRequest {
                    reason: format!("snapshot '{}' is not ready", snapshot.id),
                })?;
        let layout = self.layout(&id);
        let mut handles: Vec<CacheHandle> = Vec::new();
        let resolve_started = Instant::now();
        let (layer_guard, layers_checked) = protect_snapshot_layers(
            &self.leases,
            &id,
            committed,
            &self.managed_layers_repo_blob_url,
        )
        .await?;
        let memory_layers: Vec<OverlaybdLayerRef> = committed
            .memory_layers
            .iter()
            .map(|m| OverlaybdLayerRef::Managed(m.clone()))
            .collect();

        // Every branch reads immutable committed metadata and writes a distinct
        // cache key or runtime image-config path. Drive them together so
        // object-store round trips and local materialization do not serialize.
        // `join!` deliberately lets every branch finish on error, preserving
        // each cache operation's cleanup protocol.
        let vm_state = self.resolve_vm_state(&id);
        let firecracker_manifest = self.load_committed_firecracker_manifest(&layout, &id);
        let memory = async {
            let mut branch_handles = Vec::new();
            let cache_key = runtime_image_cache_key(&id, "memory/image.json");
            let path = self
                .materialize_layers_and_pin(
                    &memory_layers,
                    &self.image_materializer.memory_image_config_path(&id),
                    MaterializeSpec {
                        label: "memory",
                        cache_key: &cache_key,
                        allow_empty_layers: true,
                        download: None,
                        layers_checked,
                    },
                    &mut branch_handles,
                )
                .await?;
            Ok::<_, RepositoryError>((path, branch_handles))
        };
        let rootfs = async {
            let mut branch_handles = Vec::new();
            let cache_key = runtime_image_cache_key(&id, "rootfs/image.json");
            let path = self
                .materialize_layers_and_pin(
                    &committed.rootfs_layers,
                    &self.image_materializer.rootfs_image_config_path(&id),
                    MaterializeSpec {
                        label: "rootfs",
                        cache_key: &cache_key,
                        allow_empty_layers: false,
                        download: None,
                        layers_checked,
                    },
                    &mut branch_handles,
                )
                .await?;
            Ok::<_, RepositoryError>((path, branch_handles))
        };
        let attached = async {
            let mut branch_handles = Vec::new();
            let drives = self
                .resolve_attached_drives(
                    &id,
                    &committed.attached_drives,
                    layers_checked,
                    &mut branch_handles,
                )
                .await?;
            Ok::<_, RepositoryError>((drives, branch_handles))
        };
        let tools = async {
            let mut branch_handles = Vec::new();
            let path = self
                .resolve_tools_drive(committed, &mut branch_handles)
                .await?;
            Ok::<_, RepositoryError>((path, branch_handles))
        };

        let (vm_state, committed_manifest, memory, rootfs, attached, tools) = tokio::join!(
            vm_state,
            firecracker_manifest,
            memory,
            rootfs,
            attached,
            tools
        );
        let (vm_state_path, vm_state_handle) = vm_state?;
        let committed_manifest = committed_manifest?;
        let (mem_image_config_path, mut memory_handles) = memory?;
        let (rootfs_image_config_path, mut rootfs_handles) = rootfs?;
        let (attached_drives, mut attached_handles) = attached?;
        let (tools_drive_path, mut tools_handles) = tools?;
        handles.push(vm_state_handle);
        handles.append(&mut memory_handles);
        handles.append(&mut rootfs_handles);
        handles.append(&mut attached_handles);
        handles.append(&mut tools_handles);
        info!(
            snapshot_id = %id,
            phase = "runtime_artifacts_resolved",
            elapsed_ms = resolve_started.elapsed().as_secs_f64() * 1000.0,
            "snapshot runtime resolution phase completed"
        );

        let mut config_paths = vec![
            mem_image_config_path.as_path(),
            rootfs_image_config_path.as_path(),
        ];
        config_paths.extend(attached_drives.iter().map(|drive| match drive {
            ResolvedAttachedDrive::Overlaybd {
                image_config_path, ..
            } => image_config_path.as_path(),
        }));
        self.record_image_configs(layer_guard.digests(), &config_paths);

        // Runtime artifacts are protected by the sandbox start-window lease (over
        // local-only commits) + the orchestrator running set; the resolved-handle
        // needs no separate local image ref pin. The managed-layer lease guard
        // keeps the remote layers leased until the launch hands over to the
        // runtime set.
        let cache_lease: Arc<dyn RuntimeArtifactLease> = Arc::new(OssRuntimeLease {
            _cache: CacheArtifactLease { _handles: handles },
            _layers: layer_guard,
        });

        let runtime_manifest = hydrate_runtime_manifest(
            committed_manifest,
            vm_state_path,
            mem_image_config_path,
            rootfs_image_config_path,
            tools_drive_path,
            &attached_drives,
        )?;

        let runnable = RunnableSnapshot::new((*snapshot).clone(), runtime_manifest, cache_lease);
        debug!(snapshot_id = %id, "resolved oss snapshot to local runnable paths");
        Ok(runnable)
    }
}

// ── private helpers ───────────────────────────────────────────────────

impl OssRuntimeResolver {
    async fn resolve_vm_state(&self, id: &SnapshotId) -> RepositoryResult<(PathBuf, CacheHandle)> {
        let layout = self.layout(id);
        let key = layout.artifact_key(SNAPSHOT_ARTIFACT_LAYOUT.vm_state);
        let client = Arc::clone(&self.client);
        let p2p_transport = self.p2p_transport.clone();
        let p2p_key = p2p::fixed_artifact_key(id, SNAPSHOT_ARTIFACT_LAYOUT.vm_state);
        let handle = self
            .cache
            .ensure_cached(&key, |destination| {
                let client = Arc::clone(&client);
                let key = key.clone();
                let p2p_transport = p2p_transport.clone();
                let p2p_key = p2p_key.clone();
                async move {
                    if let Some(transport) = p2p_transport.as_ref() {
                        match p2p::fetch_artifact(transport, &p2p_key, &destination).await {
                            Ok(size) => return Ok(size),
                            Err(error) => {
                                debug!(
                                    key = %p2p_key,
                                    error = %error,
                                    "P2P vm_state fetch failed; using backend fallback"
                                );
                            }
                        }
                    }
                    client.get_to_file(&key, &destination).await
                }
            })
            .await
            .map_err(|error| RepositoryError::ArtifactNotFound {
                artifact: format!("vm state artifact for snapshot '{id}': {error}"),
            })?;
        Ok((handle.path().to_path_buf(), handle))
    }

    async fn resolve_tools_drive(
        &self,
        snapshot: &crate::snapshot::CommittedSnapshot,
        handles: &mut Vec<CacheHandle>,
    ) -> RepositoryResult<Option<PathBuf>> {
        let Some(tools_drive) = &snapshot.tools_drive else {
            // Compatibility for snapshots created before tools-drive artifacts
            // were repository-backed. Remove after those snapshots age out of retention.
            return Ok(None);
        };
        let key = OssSnapshotArtifactLayout::managed_layer_key(&tools_drive.digest);
        let cache_key = key.clone();
        let client = Arc::clone(&self.client);
        let expected = tools_drive.clone();
        let handle = self
            .cache
            .ensure_cached(&cache_key, move |destination| {
                let client = Arc::clone(&client);
                let key = key.clone();
                let expected = expected.clone();
                async move {
                    client.get_to_file(&key, &destination).await?;
                    validate_managed_artifact(&destination, &expected, "tools drive")
                        .await
                        .map_err(anyhow::Error::new)?;
                    Ok(expected.size)
                }
            })
            .await
            .map_err(|error| {
                RepositoryError::backend(
                    format!("materialize tools drive '{}'", tools_drive.digest),
                    error,
                )
            })?;
        validate_managed_artifact(handle.path(), tools_drive, "tools drive").await?;
        let path = handle.path().to_path_buf();
        handles.push(handle);
        Ok(Some(path))
    }

    /// Materialize an image config from layers and pin it in the cache.
    async fn materialize_layers_and_pin(
        &self,
        layers: &[OverlaybdLayerRef],
        destination: &Path,
        spec: MaterializeSpec<'_>,
        handles: &mut Vec<CacheHandle>,
    ) -> RepositoryResult<PathBuf> {
        if !spec.allow_empty_layers && layers.is_empty() {
            return Err(RepositoryError::InvalidRequest {
                reason: format!("{} has no layers", spec.label),
            });
        }

        let download = spec.download.clone();
        let handle = self
            .cache
            .ensure_cached_at(spec.cache_key, destination.to_path_buf(), |dest| {
                let download = download.clone();
                async move {
                    let path = self
                        .materialize_image_config(
                            layers,
                            &dest,
                            spec.label,
                            download,
                            spec.layers_checked,
                        )
                        .await
                        .map_err(anyhow::Error::new)?;
                    tokio::fs::metadata(&path)
                        .await
                        .map(|metadata| metadata.len())
                        .map_err(|error| {
                            anyhow::Error::new(RepositoryError::backend(
                                format!("stat {} image config '{}'", spec.label, path.display()),
                                error,
                            ))
                        })
                }
            })
            .await
            .map_err(|error| materialize_image_config_error(spec.label, error))?;
        let path = handle.path().to_path_buf();
        handles.push(handle);
        Ok(path)
    }

    /// Verify remote managed layers exist (unless this resolve already checked
    /// them under its lease), build an `ImageConfig`, and write it.
    async fn materialize_image_config(
        &self,
        layers: &[OverlaybdLayerRef],
        destination: &Path,
        label: &str,
        download: Option<DownloadConfig>,
        layers_checked: bool,
    ) -> RepositoryResult<PathBuf> {
        if !layers_checked {
            let managed_layers = layers
                .iter()
                .enumerate()
                .filter_map(|(index, layer)| {
                    managed_layer_ref_digest(layer, &self.managed_layers_repo_blob_url)
                        .map(|digest| (index, digest.to_string()))
                })
                .collect::<Vec<_>>();
            let client = Arc::clone(&self.client);
            validate_managed_layers(&managed_layers, label, move |key| {
                let client = Arc::clone(&client);
                async move { client.exists(&key).await }
            })
            .await?;
        }

        self.image_materializer
            .materialize_image_config(
                layers,
                destination,
                label,
                Some(&self.managed_layers_repo_blob_url),
                download,
                |_, managed| async move {
                    Ok(LayerConfig {
                        digest: managed.digest,
                        size: managed.size,
                        uuid: managed.uuid.unwrap_or_default(),
                        ..Default::default()
                    })
                },
            )
            .await
    }

    async fn resolve_attached_drives(
        &self,
        id: &SnapshotId,
        committed_drives: &[CommittedAttachedDrive],
        layers_checked: bool,
        handles: &mut Vec<CacheHandle>,
    ) -> RepositoryResult<Vec<ResolvedAttachedDrive>> {
        let mut drives = Vec::new();

        for drive in committed_drives {
            match drive {
                CommittedAttachedDrive::Overlaybd {
                    drive_id,
                    layers,
                    read_only,
                    virtual_size,
                    mount_path,
                    sub_path,
                } => {
                    if *virtual_size == 0 {
                        return Err(RepositoryError::InvalidRequest {
                            reason: format!(
                                "attached drive '{}' virtual_size must be non-zero",
                                drive_id
                            ),
                        });
                    }
                    let image_config_path = self
                        .materialize_layers_and_pin(
                            layers,
                            &self
                                .image_materializer
                                .drive_image_config_path(id, drive_id),
                            MaterializeSpec {
                                label: &format!("drive '{drive_id}'"),
                                cache_key: &runtime_image_cache_key(
                                    id,
                                    &format!("drives/{drive_id}/image.json"),
                                ),
                                allow_empty_layers: false,
                                download: None,
                                layers_checked,
                            },
                            handles,
                        )
                        .await?;

                    drives.push(ResolvedAttachedDrive::Overlaybd {
                        drive_id: drive_id.clone(),
                        image_config_path,
                        read_only: *read_only,
                        virtual_size: *virtual_size,
                        mount_path: crate::sandbox::normalize_mount_path_for_drive(
                            drive_id,
                            mount_path.clone(),
                        )
                        .unwrap_or_else(|_| {
                            crate::sandbox::ExtraDrive::default_mount_path(drive_id)
                        }),
                        sub_path: sub_path.clone(),
                    });
                }
            }
        }

        Ok(drives)
    }

    async fn load_committed_firecracker_manifest(
        &self,
        layout: &OssSnapshotArtifactLayout<'_>,
        snapshot_id: &SnapshotId,
    ) -> RepositoryResult<crate::sandbox::FirecrackerSnapshotManifest> {
        let p2p_key =
            p2p::fixed_artifact_key(snapshot_id, SNAPSHOT_ARTIFACT_LAYOUT.firecracker_manifest);
        if let Some(transport) = self.p2p_transport.as_ref() {
            match p2p::fetch_artifact_bytes(transport, &p2p_key).await {
                Ok(bytes) => match parse_firecracker_manifest(&bytes, &p2p_key) {
                    Ok(manifest) => return Ok(manifest),
                    Err(error) => {
                        debug!(
                            key = %p2p_key,
                            error = %error,
                            "P2P firecracker manifest parse failed; using backend fallback"
                        );
                    }
                },
                Err(error) => {
                    debug!(
                        key = %p2p_key,
                        error = %error,
                        "P2P firecracker manifest fetch failed; using backend fallback"
                    );
                }
            }
        }

        let key = layout.artifact_key(SNAPSHOT_ARTIFACT_LAYOUT.firecracker_manifest);
        let bytes = self.client.get_bytes(&key).await.map_err(|error| {
            if OssClient::is_not_found_error(&error) {
                return RepositoryError::ArtifactNotFound {
                    artifact: format!(
                        "firecracker manifest artifact for snapshot '{}' at key '{}'",
                        snapshot_id, key
                    ),
                };
            }
            RepositoryError::backend(
                format!("read firecracker manifest for snapshot '{snapshot_id}'"),
                error,
            )
        })?;
        parse_firecracker_manifest(&bytes, &key)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use tokio::sync::Barrier;

    fn digest(index: usize) -> String {
        format!("sha256:{index:064x}")
    }

    #[tokio::test(start_paused = true)]
    async fn resolve_protects_before_validating_and_validates_external_managed_refs() {
        use super::super::layer_store::fake::{FakeLayerStore, OpKind};
        use super::super::layer_store::LayerStore;
        use super::super::layout::{LAYER_GC_INTENTS_PREFIX, LAYER_GC_LEASES_PREFIX};
        use crate::snapshot::{ExternalLayer, ManagedLayer};

        const MANAGED_URL: &str = "s3://bucket/prefix/managed-layers";
        let mut committed = CommittedSnapshot::mock();
        committed.rootfs_layers = vec![
            OverlaybdLayerRef::Managed(ManagedLayer {
                digest: digest(1),
                size: 1,
                uuid: None,
            }),
            OverlaybdLayerRef::External(ExternalLayer {
                digest: digest(2),
                repo_blob_url: format!("{MANAGED_URL}/"),
                size: 1,
            }),
            OverlaybdLayerRef::External(ExternalLayer {
                digest: digest(3),
                repo_blob_url: "https://registry.example/v2/ns/image/blobs".to_string(),
                size: 1,
            }),
        ];
        committed.memory_layers = vec![ManagedLayer {
            digest: digest(4),
            size: 1,
            uuid: None,
        }];
        let id = SnapshotId::generate();
        let store = Arc::new(FakeLayerStore::new());
        for index in [1, 2, 4] {
            store.insert(
                &OssSnapshotArtifactLayout::managed_layer_key(&digest(index)),
                vec![0u8],
            );
        }
        let leases = LayerLeases::new(Arc::clone(&store) as Arc<dyn LayerStore>, "node");

        let (guard, checked) = protect_snapshot_layers(&leases, &id, &committed, MANAGED_URL)
            .await
            .expect("resolve protection");
        assert!(checked);
        let ops = store.ops();
        let put = ops
            .iter()
            .position(|op| op.kind == OpKind::Put && op.key.starts_with(LAYER_GC_LEASES_PREFIX))
            .expect("lease put");
        let list = ops
            .iter()
            .position(|op| op.kind == OpKind::List && op.key == LAYER_GC_INTENTS_PREFIX)
            .expect("intent list");
        let stats = ops
            .iter()
            .enumerate()
            .filter(|(_, op)| op.kind == OpKind::Stat)
            .collect::<Vec<_>>();
        assert!(put < list);
        assert!(stats.iter().all(|(index, _)| *index > list));
        let stat_keys = stats
            .iter()
            .map(|(_, op)| op.key.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let expected = [1, 2, 4]
            .into_iter()
            .map(|index| OssSnapshotArtifactLayout::managed_layer_key(&digest(index)))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(stat_keys, expected, "registry layers are not checked");
        assert_eq!(
            guard.digests(),
            &[1, 2, 4].into_iter().map(digest).collect()
        );

        // Warm: already checked while continuously leased.
        store.clear_ops();
        let (_warm, checked) = protect_snapshot_layers(&leases, &id, &committed, MANAGED_URL)
            .await
            .expect("warm resolve protection");
        assert!(!checked);
        assert!(store.ops().is_empty());

        // A missing external managed-URL layer fails the resolve cleanly.
        let other = LayerLeases::new(Arc::clone(&store) as Arc<dyn LayerStore>, "other");
        store.remove(&OssSnapshotArtifactLayout::managed_layer_key(&digest(2)));
        let error = protect_snapshot_layers(&other, &id, &committed, MANAGED_URL)
            .await
            .expect_err("missing layer");
        assert!(
            matches!(&error, RepositoryError::ArtifactNotFound { artifact } if artifact.contains(&digest(2))),
            "{error:?}"
        );
    }

    #[tokio::test]
    async fn validate_managed_layers_reports_missing_layer() {
        let indexed_layers = [(3, digest(3)), (7, digest(7)), (11, digest(11))];
        let missing_key = OssSnapshotArtifactLayout::managed_layer_key(&indexed_layers[2].1);
        let expected_artifact = format!(
            "rootfs: managed layer 11 '{}' is missing",
            indexed_layers[2].1
        );

        let error = validate_managed_layers(&indexed_layers, "rootfs", move |key| {
            let missing_key = missing_key.clone();
            async move { Ok(key != missing_key) }
        })
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            RepositoryError::ArtifactNotFound { artifact } if artifact == expected_artifact
        ));
    }

    #[tokio::test]
    async fn validate_managed_layers_limits_concurrency_to_sixteen() {
        let indexed_layers = (0..32)
            .map(|index| (index, digest(index)))
            .collect::<Vec<_>>();
        let in_flight = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(MANAGED_LAYER_EXISTS_CONCURRENCY));

        validate_managed_layers(&indexed_layers, "memory", {
            let in_flight = Arc::clone(&in_flight);
            let maximum = Arc::clone(&maximum);
            let barrier = Arc::clone(&barrier);
            move |_| {
                let in_flight = Arc::clone(&in_flight);
                let maximum = Arc::clone(&maximum);
                let barrier = Arc::clone(&barrier);
                async move {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    maximum.fetch_max(current, Ordering::SeqCst);
                    tokio::time::timeout(Duration::from_secs(1), barrier.wait())
                        .await
                        .map_err(|_| anyhow::anyhow!("concurrency barrier timed out"))?;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok(true)
                }
            }
        })
        .await
        .unwrap();

        assert_eq!(MANAGED_LAYER_EXISTS_CONCURRENCY, 16);
        assert_eq!(
            maximum.load(Ordering::SeqCst),
            MANAGED_LAYER_EXISTS_CONCURRENCY
        );
    }
}
