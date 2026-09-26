use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    application::{error::AppResult, principal::Principal},
    domain::attachment::AttachmentId,
};

#[derive(Debug, Clone)]
pub struct StartUploadInput {
    pub filename: String,
    pub content_type: String,
    pub size: i64,
}

/// A file as shown to users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentView {
    pub id: AttachmentId,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub created_at: DateTime<Utc>,
}

/// Where and how the client uploads the file bytes.
#[derive(Debug, Clone)]
pub struct UploadSlot {
    pub attachment: AttachmentView,
    /// Presigned URL for a single HTTP `PUT` of the file.
    pub upload_url: String,
    /// Headers the `PUT` must carry exactly (they are part of the signature).
    pub upload_headers: Vec<(String, String)>,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait AttachmentUseCases: Send + Sync {
    /// Reserves a draft attachment and returns a short-lived upload URL. The
    /// draft becomes visible once a ticket or reply that lists it is sent.
    async fn start_upload(&self, actor: &Principal, input: StartUploadInput) -> AppResult<UploadSlot>;
    /// A short-lived download URL, if the actor may see the file.
    async fn download_url(&self, actor: &Principal, id: AttachmentId) -> AppResult<String>;
}
