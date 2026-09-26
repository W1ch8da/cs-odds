use async_trait::async_trait;
use chrono::Duration;

use crate::application::error::AppResult;

#[derive(Debug, Clone)]
pub struct PresignedPut {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

/// Object storage for file bytes (S3, MinIO). Clients move the bytes directly
/// through presigned URLs; the API only signs and checks.
#[async_trait]
pub trait FileStorage: Send + Sync {
    async fn presign_put(&self, key: &str, content_type: &str, ttl: Duration) -> AppResult<PresignedPut>;
    /// A GET URL that makes browsers download the file under `filename`.
    async fn presign_get(&self, key: &str, filename: &str, content_type: &str, ttl: Duration) -> AppResult<String>;
    /// The stored object's size in bytes, or `None` if nothing was uploaded.
    async fn object_size(&self, key: &str) -> AppResult<Option<i64>>;
}
