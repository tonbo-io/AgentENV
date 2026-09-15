use anyhow::Result;
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::Client as S3Client;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::minio::MinIO;

pub const MINIO_USER: &str = "minioadmin";
pub const MINIO_PASS: &str = "minioadmin";
pub const REGION: &str = "us-east-1";
pub const BUCKET: &str = "test-bucket";

pub struct MinioFixture {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub client: S3Client,
    _container: ContainerAsync<MinIO>,
}

impl MinioFixture {
    pub async fn start() -> Result<Self> {
        // Docker Hub removed this historical image. Keep the same release from
        // the publisher's Quay registry, pinned to its multi-platform manifest.
        let container = MinIO::default()
            .with_name("quay.io/minio/minio")
            .with_tag("RELEASE.2022-02-07T08-17-33Z@sha256:7dda745aefd6152f0d04fdd23377f9e52549df3fc4307f16b8bc562ae2b8119f")
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
