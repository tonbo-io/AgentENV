use std::path::Path;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use anyhow::{Context, Result};
use bytes::Bytes;
use futures::{stream, TryStreamExt};
use object_store_operator::{
    build_object_store_operator, run_with_refresh, AddressingStyle, CachedCredentialSource,
    CredentialSource, ObjectStoreOperatorConfig, ObjectStoreOperatorError, OperatorWithCredential,
};
use opendal::{Error as OpenDalError, ErrorKind as OpenDalErrorKind, Operator};
use overlaybd::backend::oss::upload_file_streaming;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::info;
use url::Url;

use crate::observability::prometheus::MetricGuard;

/// Prefer enough parts to use the upload concurrency for ordinary snapshot
/// layers while growing the part size for large objects so S3's 10,000-part
/// limit never becomes the repository's snapshot-size limit.
const MIN_CHUNK_SIZE: usize = 16 * 1024 * 1024;
const CHUNK_ALIGNMENT: usize = 1024 * 1024;
const MAX_MULTIPART_PARTS: u64 = 10_000;
/// Number of multipart parts uploaded concurrently per file. A single
/// sequential stream tops out at roughly 100 MB/s to the OSS internal
/// endpoint; concurrent parts multiply effective throughput.
const UPLOAD_CONCURRENCY: usize = 8;
const OSS_OPERATION_DURATION: &str = "agentenv_snapshot_oss_operation_duration_seconds";

/// Snapshot artifacts uploaded to OSS. Used as the `artifact` label on upload
/// metrics and in upload completion logs so memory layers can be told apart
/// from rootfs/attached-drive layers.
#[derive(Clone, Copy, Debug)]
pub(crate) enum OssUploadArtifact {
    RootfsLayer,
    AttachedDriveLayer,
    MemoryLayer,
    ToolsDrive,
    VmState,
    FirecrackerManifest,
    CatalogRecord,
    Alias,
    LayerGc,
}

impl OssUploadArtifact {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RootfsLayer => "rootfs_layer",
            Self::AttachedDriveLayer => "attached_drive_layer",
            Self::MemoryLayer => "memory_layer",
            Self::ToolsDrive => "tools_drive",
            Self::VmState => "vm_state",
            Self::FirecrackerManifest => "manifest",
            Self::CatalogRecord => "record",
            Self::Alias => "alias",
            Self::LayerGc => "layer_gc",
        }
    }
}

/// One listed or stat'ed object with the metadata managed-layer GC relies on.
///
/// `last_modified` is the object store's own timestamp, so comparisons between
/// two of these values never depend on node wall clocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredObject {
    /// Repository-relative key (the configured backend prefix is stripped).
    pub(crate) key: String,
    pub(crate) size: u64,
    pub(crate) last_modified: SystemTime,
}

/// How many requests one operation may send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attempts {
    /// OpenDAL's retry layer retries temporary errors.
    Retried,
    /// No retry layer: a reported success means the only request landed, so
    /// no earlier attempt of the same operation can land after it. Managed-
    /// layer GC protocol writes rely on this.
    Single,
}

/// Thin wrapper around the OSS client used by the repository and resolver.
#[derive(Clone, Debug)]
pub(crate) struct OssClient {
    operator_config: ObjectStoreOperatorConfig,
    /// `operator_config` with retries disabled, for [`Attempts::Single`].
    single_attempt_config: ObjectStoreOperatorConfig,
    prefix: String,
    credentials: Arc<CachedCredentialSource>,
    cached_operator: Arc<RwLock<Option<OperatorWithCredential>>>,
    cached_single_attempt_operator: Arc<RwLock<Option<OperatorWithCredential>>>,
}

impl OssClient {
    pub(crate) fn new(
        bucket: String,
        endpoint: String,
        region: String,
        prefix: String,
        credential_source: CredentialSource,
        addressing_override: Option<AddressingStyle>,
    ) -> Result<Self> {
        // Detection also validates the endpoint URL, so it always runs; an
        // explicit config override then wins over the detected style.
        let detected_style = detect_addressing_style(&endpoint, &bucket)?;
        let addressing_style = addressing_override.unwrap_or(detected_style);
        let operator_config = ObjectStoreOperatorConfig {
            addressing_style,
            bucket,
            endpoint,
            region,
            timeout: None,
            max_retries: None,
        };
        let single_attempt_config = ObjectStoreOperatorConfig {
            max_retries: Some(0),
            ..operator_config.clone()
        };
        Ok(Self {
            operator_config,
            single_attempt_config,
            prefix,
            credentials: Arc::new(CachedCredentialSource::new(credential_source)),
            cached_operator: Arc::new(RwLock::new(None)),
            cached_single_attempt_operator: Arc::new(RwLock::new(None)),
        })
    }

    fn full_key(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix, key)
        }
    }

    pub(crate) fn managed_layers_repo_blob_url(&self) -> String {
        // overlaybd expects an S3-compatible repo blob URL here, including for
        // Alibaba OSS, so the scheme remains `s3://` rather than `oss://`.
        if self.prefix.is_empty() {
            format!("s3://{}/managed-layers", self.operator_config.bucket)
        } else {
            format!(
                "s3://{}/{}/managed-layers",
                self.operator_config.bucket, self.prefix
            )
        }
    }

    /// Read a small object entirely into memory.
    pub(crate) async fn get_bytes(&self, key: &str) -> Result<Bytes> {
        let mut metric = MetricGuard::operation(OSS_OPERATION_DURATION, "get_bytes");
        let result = self
            .run_with_key(key, |operator, key| async move {
                operator.read(&key).await.map(|buffer| buffer.to_bytes())
            })
            .await
            .with_context(|| format!("oss get '{key}'"));
        metric.finish(&result);
        result
    }

    /// Download an object directly to a local file (atomic: temp + rename).
    pub(crate) async fn get_to_file(&self, key: &str, dest: &Path) -> Result<u64> {
        let mut metric = MetricGuard::operation(OSS_OPERATION_DURATION, "get_to_file");
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("create cache dir '{}'", parent.display()))?;
        }

        let dest = dest.to_path_buf();
        let oss_key = self.full_key(key);
        let result = self
            .run_with_operator(|operator| {
                let dest = dest.clone();
                let oss_key = oss_key.clone();
                async move { download_object_to_file(&operator, &oss_key, &dest).await }
            })
            .await
            .with_context(|| format!("oss download '{key}'"));
        metric.finish(&result);
        result
    }

    /// Check whether an object exists.
    pub(crate) async fn exists(&self, key: &str) -> Result<bool> {
        let mut metric = MetricGuard::operation(OSS_OPERATION_DURATION, "exists");
        let result = self
            .run_with_key(
                key,
                |operator, key| async move { operator.exists(&key).await },
            )
            .await
            .with_context(|| format!("oss exists '{key}'"));
        metric.finish(&result);
        result
    }

    /// List all files recursively under a prefix.
    pub(crate) async fn list_keys_recursive(&self, prefix: &str) -> Result<Vec<String>> {
        let keys = self
            .run_with_key(prefix, |operator, prefix| async move {
                let entries = operator.list_with(&prefix).recursive(true).await?;

                Ok(entries
                    .into_iter()
                    .filter(|entry| !entry.metadata().mode().is_dir())
                    .map(|entry| entry.path().to_string())
                    .collect())
            })
            .await
            .with_context(|| format!("oss list '{prefix}'"))?;

        if self.prefix.is_empty() {
            return Ok(keys);
        }
        let strip = format!("{}/", self.prefix);
        Ok(keys
            .into_iter()
            .map(|p| p.strip_prefix(&strip).unwrap_or(&p).to_string())
            .collect())
    }

    /// List all files recursively under a prefix with their size and
    /// object-store `LastModified`. Fails closed when the store omits
    /// `LastModified` for any entry.
    pub(crate) async fn list_objects_recursive(&self, prefix: &str) -> Result<Vec<StoredObject>> {
        let mut metric = MetricGuard::operation(OSS_OPERATION_DURATION, "list_objects");
        let result = self
            .run_with_key(prefix, |operator, prefix| async move {
                let entries = operator.list_with(&prefix).recursive(true).await?;
                Ok(entries
                    .into_iter()
                    .filter(|entry| !entry.metadata().mode().is_dir())
                    .map(|entry| {
                        let metadata = entry.metadata();
                        (
                            entry.path().to_string(),
                            metadata.content_length(),
                            metadata.last_modified().map(SystemTime::from),
                        )
                    })
                    .collect::<Vec<_>>())
            })
            .await
            .with_context(|| format!("oss list objects '{prefix}'"));
        metric.finish(&result);
        result?
            .into_iter()
            .map(|(path, size, last_modified)| {
                let key = self.strip_prefix(&path);
                let last_modified = last_modified.ok_or_else(|| {
                    anyhow::anyhow!("oss listing omitted LastModified for '{key}'")
                })?;
                Ok(StoredObject {
                    key,
                    size,
                    last_modified,
                })
            })
            .collect()
    }

    /// Read one object's size and `LastModified`. Returns `None` when the
    /// object does not exist.
    pub(crate) async fn stat(&self, key: &str) -> Result<Option<StoredObject>> {
        let mut metric = MetricGuard::operation(OSS_OPERATION_DURATION, "stat");
        let result = self
            .run_with_key(
                key,
                |operator, key| async move { operator.stat(&key).await },
            )
            .await;
        let result = match result {
            Ok(metadata) => metadata
                .last_modified()
                .map(SystemTime::from)
                .ok_or_else(|| anyhow::anyhow!("oss stat omitted LastModified for '{key}'"))
                .map(|last_modified| {
                    Some(StoredObject {
                        key: key.to_string(),
                        size: metadata.content_length(),
                        last_modified,
                    })
                }),
            Err(error) if Self::is_not_found_error(&error) => Ok(None),
            Err(error) => Err(error.context(format!("oss stat '{key}'"))),
        };
        metric.finish(&result);
        result
    }

    fn strip_prefix(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            return path.to_string();
        }
        let strip = format!("{}/", self.prefix);
        path.strip_prefix(&strip).unwrap_or(path).to_string()
    }

    /// Write small data (catalog JSON, alias JSON, etc.).
    pub(crate) async fn put_bytes(
        &self,
        key: &str,
        data: impl Into<Bytes>,
        artifact: OssUploadArtifact,
    ) -> Result<()> {
        self.put_bytes_with(key, data.into(), artifact, Attempts::Retried)
            .await
    }

    /// Write small data with exactly one PUT request: no client retry, so a
    /// reported success means that request landed and a failure (including
    /// a timeout) may still land later. Used by managed-layer GC protocol
    /// writes.
    pub(crate) async fn put_bytes_single_attempt(
        &self,
        key: &str,
        data: Bytes,
        artifact: OssUploadArtifact,
    ) -> Result<()> {
        self.put_bytes_with(key, data, artifact, Attempts::Single)
            .await
    }

    async fn put_bytes_with(
        &self,
        key: &str,
        data: Bytes,
        artifact: OssUploadArtifact,
        attempts: Attempts,
    ) -> Result<()> {
        let size = data.len() as u64;
        let oss_key = self.full_key(key);
        let mut metric =
            MetricGuard::operation_artifact(OSS_OPERATION_DURATION, "put_bytes", artifact.as_str());
        let result = self
            .run_with_operator_attempts(attempts, |operator| {
                let data = data.clone();
                let oss_key = oss_key.clone();
                async move { write_bytes_to_operator(&operator, &oss_key, data).await }
            })
            .await
            .with_context(|| format!("oss put '{key}'"));
        metric.finish(&result);
        if result.is_ok() {
            metrics::counter!(
                "agentenv_snapshot_oss_upload_bytes_total",
                "operation" => "put_bytes",
                "artifact" => artifact.as_str(),
            )
            .increment(size);
        }
        result?;
        Ok(())
    }

    /// Upload a local file to OSS.
    pub(crate) async fn put_file(
        &self,
        key: &str,
        path: &Path,
        artifact: OssUploadArtifact,
    ) -> Result<()> {
        let oss_key = self.full_key(key);
        let path = path.to_path_buf();
        let mut metric =
            MetricGuard::operation_artifact(OSS_OPERATION_DURATION, "put_file", artifact.as_str());
        let start = Instant::now();
        let result: Result<u64> = async {
            let size = tokio::fs::metadata(&path)
                .await
                .with_context(|| format!("stat oss upload source file '{}'", path.display()))?
                .len();
            self.run_with_operator(|operator| {
                let oss_key = oss_key.clone();
                let path = path.clone();
                async move {
                    upload_file_streaming(
                        &operator,
                        &oss_key,
                        &path,
                        upload_chunk_size(size),
                        UPLOAD_CONCURRENCY,
                        None,
                    )
                    .await
                }
            })
            .await
            .with_context(|| format!("oss put file '{key}'"))?;
            Ok(size)
        }
        .await;
        metric.finish(&result);
        match result {
            Ok(size) => {
                metrics::counter!(
                    "agentenv_snapshot_oss_upload_bytes_total",
                    "operation" => "put_file",
                    "artifact" => artifact.as_str(),
                )
                .increment(size);
                info!(
                    key = %oss_key,
                    artifact = artifact.as_str(),
                    size_bytes = size,
                    elapsed_ms = start.elapsed().as_millis(),
                    "oss file uploaded"
                );
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// Delete a single object. Idempotent – missing objects are not errors.
    pub(crate) async fn delete(&self, key: &str) -> Result<()> {
        self.delete_with(key, Attempts::Retried).await
    }

    /// Delete a single object with exactly one DELETE request (see
    /// [`Self::put_bytes_single_attempt`]). Used by managed-layer GC.
    pub(crate) async fn delete_single_attempt(&self, key: &str) -> Result<()> {
        self.delete_with(key, Attempts::Single).await
    }

    async fn delete_with(&self, key: &str, attempts: Attempts) -> Result<()> {
        let oss_key = self.full_key(key);
        self.run_with_operator_attempts(attempts, |operator| {
            let oss_key = oss_key.clone();
            async move {
                match operator.delete(&oss_key).await {
                    Ok(()) => Ok(()),
                    Err(err) if err.kind() == OpenDalErrorKind::NotFound => Ok(()),
                    Err(err) => Err(err),
                }
            }
        })
        .await
        .with_context(|| format!("oss delete '{key}'"))
    }

    /// Delete all objects under a prefix.
    pub(crate) async fn delete_prefix(&self, prefix: &str) -> Result<()> {
        // `list_keys_recursive()` returns repository-relative keys with the
        // configured backend prefix stripped, while `delete()` expects that
        // same repository-relative form and re-applies the backend prefix.
        let keys = self.list_keys_recursive(prefix).await?;
        stream::iter(keys.into_iter().map(Ok::<_, anyhow::Error>))
            .try_for_each_concurrent(16, |key| async move { self.delete(&key).await })
            .await
    }

    pub(crate) fn is_not_found_error(error: &anyhow::Error) -> bool {
        error.chain().any(|cause| {
            if let Some(opendal_error) = cause.downcast_ref::<OpenDalError>() {
                return opendal_error.kind() == OpenDalErrorKind::NotFound;
            }
            if let Some(ObjectStoreOperatorError::OpenDal(opendal_error)) =
                cause.downcast_ref::<ObjectStoreOperatorError>()
            {
                return opendal_error.kind() == OpenDalErrorKind::NotFound;
            }
            false
        })
    }

    async fn run_with_key<T, F, Fut>(&self, key: &str, operation: F) -> Result<T>
    where
        F: Fn(Operator, String) -> Fut,
        Fut: std::future::Future<Output = opendal::Result<T>>,
    {
        let key = self.full_key(key);
        self.run_with_operator(|operator| {
            let key = key.clone();
            operation(operator, key)
        })
        .await
    }

    async fn run_with_operator<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn(Operator) -> Fut,
        Fut: std::future::Future<Output = opendal::Result<T>>,
    {
        self.run_with_operator_attempts(Attempts::Retried, operation)
            .await
    }

    async fn run_with_operator_attempts<T, F, Fut>(
        &self,
        attempts: Attempts,
        operation: F,
    ) -> Result<T>
    where
        F: Fn(Operator) -> Fut,
        Fut: std::future::Future<Output = opendal::Result<T>>,
    {
        // Centralizes one-shot operator construction plus credential-refresh
        // retry semantics so individual OSS operations don't each have to
        // reason about cached credentials and operator replacement. The
        // refresh path sends a second request only after the store answered
        // the first with permission denied, so it never leaves an earlier
        // request of a single-attempt operation in flight.
        let (config, cache) = self.operator_slot(attempts);
        let current = self.ensure_fresh_operator(config, cache).await?;
        let (value, refreshed) =
            run_with_refresh(&current, Some(self.credentials.as_ref()), config, operation)
                .await
                .map_err(anyhow::Error::from)?;
        if let Some(refreshed) = refreshed {
            *cache.write().await = Some(refreshed);
        }
        Ok(value)
    }

    fn operator_slot(
        &self,
        attempts: Attempts,
    ) -> (
        &ObjectStoreOperatorConfig,
        &RwLock<Option<OperatorWithCredential>>,
    ) {
        match attempts {
            Attempts::Retried => (&self.operator_config, &self.cached_operator),
            Attempts::Single => (
                &self.single_attempt_config,
                &self.cached_single_attempt_operator,
            ),
        }
    }

    async fn ensure_fresh_operator(
        &self,
        config: &ObjectStoreOperatorConfig,
        cache: &RwLock<Option<OperatorWithCredential>>,
    ) -> Result<OperatorWithCredential> {
        let credential = self.credentials.current().await?.ok_or_else(|| {
            anyhow::anyhow!("snapshot OSS client requires non-anonymous credentials")
        })?;

        {
            let cached = cache.read().await;
            if let Some(state) = cached.as_ref() {
                if state.credential() == Some(&credential) {
                    return Ok(state.clone());
                }
            }
        }

        let entry = OperatorWithCredential::new(
            build_object_store_operator(config, Some(&credential))?,
            Some(credential),
        );
        *cache.write().await = Some(entry.clone());
        Ok(entry)
    }
}

fn detect_addressing_style(endpoint: &str, bucket: &str) -> Result<AddressingStyle> {
    let url = Url::parse(endpoint).context("parse snapshot OSS endpoint for addressing style")?;
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("snapshot OSS endpoint host is missing"))?;
    let bucket_host = format!("{bucket}.");
    let is_bucket_virtual_host = host.starts_with(&bucket_host);
    let is_aliyun_endpoint = host.ends_with(".aliyuncs.com") || host.ends_with(".aliyun-inc.com");

    if is_bucket_virtual_host {
        return Ok(AddressingStyle::Virtual);
    }
    if is_aliyun_endpoint {
        return Ok(AddressingStyle::Virtual);
    }
    Ok(AddressingStyle::Path)
}

async fn download_object_to_file(
    operator: &Operator,
    key: &str,
    dest: &Path,
) -> opendal::Result<u64> {
    // Keep the tempfile handle alive until the final rename so any early
    // return still benefits from `NamedTempFile`'s automatic cleanup.
    let tmp = tempfile::NamedTempFile::new_in(dest.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|err| io_error_to_opendal(err, "create temporary download file"))?;
    let tmp_path = tmp.path().to_path_buf();
    let std_file = tmp
        .reopen()
        .map_err(|err| io_error_to_opendal(err, "reopen temporary download file"))?;
    let mut file = tokio::fs::File::from_std(std_file);
    let mut size = 0_u64;
    let mut stream = operator.reader(key).await?.into_stream(..).await?;
    while let Some(buffer) = stream.try_next().await? {
        for chunk in buffer {
            size += chunk.len() as u64;
            file.write_all(chunk.as_ref())
                .await
                .map_err(|err| io_error_to_opendal(err, "write downloaded object chunk"))?;
        }
    }

    file.flush()
        .await
        .map_err(|err| io_error_to_opendal(err, "flush downloaded object file"))?;
    file.sync_all()
        .await
        .map_err(|err| io_error_to_opendal(err, "sync downloaded object file"))?;
    drop(file);
    tokio::fs::rename(&tmp_path, dest)
        .await
        .map_err(|err| io_error_to_opendal(err, "rename downloaded object into place"))?;

    Ok(size)
}

async fn write_bytes_to_operator(
    operator: &Operator,
    key: &str,
    data: Bytes,
) -> opendal::Result<()> {
    operator.write(key, data).await.map(|_| ())
}

fn upload_chunk_size(size: u64) -> usize {
    let required = usize::try_from(size.div_ceil(MAX_MULTIPART_PARTS)).unwrap_or(usize::MAX);
    required
        .max(MIN_CHUNK_SIZE)
        .div_ceil(CHUNK_ALIGNMENT)
        .saturating_mul(CHUNK_ALIGNMENT)
}

fn io_error_to_opendal(error: std::io::Error, message: &'static str) -> OpenDalError {
    OpenDalError::new(OpenDalErrorKind::Unexpected, message).set_source(error)
}

#[cfg(test)]
mod tests {
    use super::{upload_chunk_size, Attempts, OssClient, MAX_MULTIPART_PARTS, MIN_CHUNK_SIZE};
    use object_store_operator::{AddressingStyle, CredentialSource};

    #[test]
    fn explicit_override_takes_precedence_over_detection() {
        let detected = OssClient::new(
            "snapshots".to_string(),
            "https://t3.storage.dev".to_string(),
            "auto".to_string(),
            String::new(),
            CredentialSource::Anonymous,
            None,
        )
        .expect("build client with detected style");
        assert_eq!(
            detected.operator_config.addressing_style,
            AddressingStyle::Path
        );

        let overridden = OssClient::new(
            "snapshots".to_string(),
            "https://t3.storage.dev".to_string(),
            "auto".to_string(),
            String::new(),
            CredentialSource::Anonymous,
            Some(AddressingStyle::Virtual),
        )
        .expect("build client with override");
        assert_eq!(
            overridden.operator_config.addressing_style,
            AddressingStyle::Virtual
        );
    }

    #[test]
    fn single_attempt_operations_disable_retries_on_the_same_target() {
        let client = OssClient::new(
            "snapshots".to_string(),
            "https://t3.storage.dev".to_string(),
            "auto".to_string(),
            "prefix".to_string(),
            CredentialSource::Anonymous,
            None,
        )
        .expect("build client");
        let (retried, _) = client.operator_slot(Attempts::Retried);
        let (single, _) = client.operator_slot(Attempts::Single);
        assert_eq!(retried.max_retries, None);
        assert_eq!(single.max_retries, Some(0));
        assert_eq!(single.bucket, retried.bucket);
        assert_eq!(single.endpoint, retried.endpoint);
        assert_eq!(single.region, retried.region);
        assert_eq!(single.addressing_style, retried.addressing_style);
        assert_eq!(single.timeout, retried.timeout);
    }

    #[test]
    fn explicit_override_still_validates_endpoint() {
        OssClient::new(
            "snapshots".to_string(),
            "not a valid endpoint".to_string(),
            "auto".to_string(),
            String::new(),
            CredentialSource::Anonymous,
            Some(AddressingStyle::Virtual),
        )
        .expect_err("malformed endpoint must fail even with an explicit override");
    }

    #[test]
    fn upload_chunk_size_uses_concurrency_without_exceeding_the_part_limit() {
        let ordinary_snapshot_layer = 252 * 1024 * 1024;
        assert_eq!(upload_chunk_size(ordinary_snapshot_layer), MIN_CHUNK_SIZE);

        let large_snapshot_layer = 512_u64 * 1024 * 1024 * 1024;
        let chunk_size = upload_chunk_size(large_snapshot_layer);
        assert!(chunk_size > MIN_CHUNK_SIZE);
        assert!(large_snapshot_layer.div_ceil(chunk_size as u64) <= MAX_MULTIPART_PARTS);
    }
}
