//! S3-compatible object storage (AWS S3, MinIO, R2). The only place that
//! knows about the AWS SDK.

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region},
    error::DisplayErrorContext,
    presigning::PresigningConfig,
};
use chrono::Duration;

use crate::application::{
    error::AppResult,
    ports::outbound::{FileStorage, PresignedPut},
};

#[derive(Clone)]
pub struct S3Settings {
    /// Address the API uses, e.g. `http://minio:9000` inside Docker.
    pub endpoint: String,
    /// Address browsers use for presigned URLs, e.g. `http://localhost:9000`.
    pub public_endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

impl std::fmt::Debug for S3Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Settings")
            .field("endpoint", &self.endpoint)
            .field("public_endpoint", &self.public_endpoint)
            .field("region", &self.region)
            .field("bucket", &self.bucket)
            .finish_non_exhaustive()
    }
}

pub struct S3Storage {
    client: Client,
    /// Signs URLs with the public host, since the host is part of the signature.
    presigner: Client,
    bucket: String,
}

fn client(endpoint: &str, s: &S3Settings) -> Client {
    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(s.region.clone()))
        .endpoint_url(endpoint)
        .credentials_provider(Credentials::new(&s.access_key, &s.secret_key, None, None, "cs-odds-config"))
        // MinIO serves buckets as paths, not subdomains.
        .force_path_style(true)
        .build();
    Client::from_conf(config)
}

fn sdk_error(context: &str, err: impl std::error::Error) -> anyhow::Error {
    anyhow!("{context}: {}", DisplayErrorContext(err))
}

fn presigning(ttl: Duration) -> anyhow::Result<PresigningConfig> {
    PresigningConfig::expires_in(ttl.to_std().context("negative TTL")?).context("invalid presigning TTL")
}

/// `attachment; filename="..."; filename*=UTF-8''...` so every browser keeps
/// the original name, including non-ASCII characters.
fn content_disposition(filename: &str) -> String {
    let ascii: String = filename.chars().map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '_' }).collect();
    let encoded: String = filename
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

impl S3Storage {
    pub fn new(settings: &S3Settings) -> Self {
        Self {
            client: client(&settings.endpoint, settings),
            presigner: client(&settings.public_endpoint, settings),
            bucket: settings.bucket.clone(),
        }
    }

    /// Creates the bucket if it doesn't exist, so local setups work out of the box.
    pub async fn ensure_bucket(&self) -> anyhow::Result<()> {
        if self.client.head_bucket().bucket(&self.bucket).send().await.is_ok() {
            return Ok(());
        }
        match self.client.create_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                tracing::info!(bucket = %self.bucket, "created storage bucket");
                Ok(())
            }
            Err(e) if e.as_service_error().is_some_and(|s| s.is_bucket_already_owned_by_you()) => Ok(()),
            Err(e) => Err(sdk_error(&format!("could not create bucket {}", self.bucket), e)),
        }
    }
}

#[async_trait]
impl FileStorage for S3Storage {
    async fn presign_put(&self, key: &str, content_type: &str, ttl: Duration) -> AppResult<PresignedPut> {
        let request = self
            .presigner
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning(ttl)?)
            .await
            .map_err(|e| sdk_error("could not presign upload", e))?;
        // Browsers set Host themselves and refuse to let scripts send it.
        let headers = request
            .headers()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("host"))
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect();
        Ok(PresignedPut { url: request.uri().to_owned(), headers })
    }

    async fn presign_get(&self, key: &str, filename: &str, content_type: &str, ttl: Duration) -> AppResult<String> {
        let request = self
            .presigner
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .response_content_disposition(content_disposition(filename))
            .response_content_type(content_type)
            .presigned(presigning(ttl)?)
            .await
            .map_err(|e| sdk_error("could not presign download", e))?;
        Ok(request.uri().to_owned())
    }

    async fn object_size(&self, key: &str) -> AppResult<Option<i64>> {
        match self.client.head_object().bucket(&self.bucket).key(key).send().await {
            Ok(head) => Ok(head.content_length()),
            Err(e) if e.as_service_error().is_some_and(|s| s.is_not_found()) => Ok(None),
            Err(e) => Err(sdk_error("could not check uploaded file", e).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disposition_keeps_unicode_names() {
        assert_eq!(content_disposition("report.pdf"), "attachment; filename=\"report.pdf\"; filename*=UTF-8''report.pdf");
        assert_eq!(
            content_disposition("ใบเสร็จ 1.pdf"),
            "attachment; filename=\"_______ 1.pdf\"; filename*=UTF-8''%E0%B9%83%E0%B8%9A%E0%B9%80%E0%B8%AA%E0%B8%A3%E0%B9%87%E0%B8%88%201.pdf"
        );
    }
}
