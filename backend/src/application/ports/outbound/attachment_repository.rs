use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::{error::AppResult, ports::inbound::AttachmentView},
    domain::{
        attachment::{AttachmentId, ContentType, FileName},
        ticket::TicketId,
        user::UserId,
    },
};

#[derive(Debug, Clone)]
pub struct NewAttachment {
    pub id: AttachmentId,
    pub uploader_id: UserId,
    pub storage_key: String,
    pub filename: FileName,
    pub content_type: ContentType,
    pub size: i64,
    pub created_at: DateTime<Utc>,
}

/// An attachment with what's needed for permission checks.
#[derive(Debug, Clone)]
pub struct AttachmentRecord {
    pub view: AttachmentView,
    pub uploader_id: UserId,
    pub storage_key: String,
    /// `None` while it is still a draft.
    pub ticket_id: Option<TicketId>,
    pub comment_id: Option<Uuid>,
    /// Whether the comment it belongs to is an internal note.
    pub internal: bool,
}

/// A linked attachment and where it appears in the conversation.
#[derive(Debug, Clone)]
pub struct TicketAttachment {
    pub view: AttachmentView,
    /// `None` for files attached to the ticket description.
    pub comment_id: Option<Uuid>,
}

#[async_trait]
pub trait AttachmentRepository: Send + Sync {
    async fn insert_draft(&self, attachment: NewAttachment) -> AppResult<AttachmentRecord>;
    async fn find(&self, id: AttachmentId) -> AppResult<Option<AttachmentRecord>>;
    async fn find_many(&self, ids: &[AttachmentId]) -> AppResult<Vec<AttachmentRecord>>;
    /// Linked attachments of a ticket, oldest first.
    async fn for_ticket(&self, ticket_id: TicketId) -> AppResult<Vec<TicketAttachment>>;
}
