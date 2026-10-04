use anyhow::Result;
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::Client as S3Client;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

pub const MINIO_USER: &str = "minioadmin";
pub const MINIO_PASS: &str = "minioadmin";
pub const REGION: &str = "us-east-1";
pub const BUCKET: &str = "test-bucket";

pub struct MinioFixture {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub client: S3Client,
    _container: ContainerAsync<GenericImage>,
}

impl MinioFixture {
    pub async fn start() -> Result<Self> {
        // MinIO no longer publishes pullable server images on Docker Hub or
        // Quay. Bitnami's frozen legacy build (MinIO 2025.7.23) stays public;
        // pin its multi-platform manifest and run the server binary directly
        // instead of Bitnami's setup entrypoint. The image runs as UID 1001,
        // so the data directory lives under /tmp.
        let container = GenericImage::new(
            "bitnamilegacy/minio",
            "2025.7.23-debian-12-r5@sha256:6dabb4a2088c9a79908de3bc05f4586c23ad2182c8908e7e3acbf61c1467fb20",
        )
        .with_entrypoint("/opt/bitnami/minio/bin/minio")
        .with_exposed_port(9000.tcp())
        // MinIO releases since 2023 print the startup banner on stderr.
        .with_wait_for(WaitFor::message_on_stderr("API:"))
        .with_env_var("MINIO_ROOT_USER", MINIO_USER)
        .with_env_var("MINIO_ROOT_PASSWORD", MINIO_PASS)
        .with_cmd(["server", "/tmp/minio-data"])
        .start()
        .await?;
        let port = container.get_host_port_ipv4(9000).await?;
        let endpoint = format!("http://127.0.0.1:{port}");
        let client = build_s3_client(&endpoint).await;
        client.create_bucket().bucket(BUCKET).send().await?;

        Ok(Self {
            endpoint,
            bucket: BUCKET.to_string(),
            region: REGION.to_string(),
            client,
            _container: container,
        })
    }

    pub async fn object_exists(&self, key: &str) -> Result<bool> {
        let result = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;
        Ok(result.is_ok())
    }

    pub async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .send()
            .await?;
        Ok(())
    }

    /// Keys under `prefix` (first page of up to 1000 keys).
    pub async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let output = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await?;
        Ok(output
            .contents()
            .iter()
            .filter_map(|object| object.key().map(str::to_string))
            .collect())
    }

    pub fn object_url(&self, key: &str) -> String {
        format!(
            "s3://{}/{}?endpoint={}&region={}",
            self.bucket, key, self.endpoint, self.region
        )
    }
}

async fn build_s3_client(endpoint: &str) -> S3Client {
    let region_provider = RegionProviderChain::default_provider().or_else(REGION);
    let creds = Credentials::new(MINIO_USER, MINIO_PASS, None, None, "agentenv-tests");
    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .load()
        .await;
    S3Client::new(&shared_config)
}
