use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    application::{
        error::{AppError, AppResult},
        ports::{
            inbound::AttachmentView,
            outbound::{AttachmentRecord, AttachmentRepository, NewAttachment, TicketAttachment},
        },
    },
    domain::{attachment::AttachmentId, ticket::TicketId, user::UserId},
};

pub struct PgAttachmentRepository {
    pool: PgPool,
}

impl PgAttachmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct Row {
    id: Uuid,
    uploader_id: Uuid,
    ticket_id: Option<Uuid>,
    comment_id: Option<Uuid>,
    internal: bool,
    storage_key: String,
    filename: String,
    content_type: String,
    size_bytes: i64,
    created_at: DateTime<Utc>,
}

impl From<Row> for AttachmentRecord {
    fn from(r: Row) -> Self {
        AttachmentRecord {
            view: AttachmentView {
                id: AttachmentId(r.id),
                filename: r.filename,
                content_type: r.content_type,
                size: r.size_bytes,
                created_at: r.created_at,
            },
            uploader_id: UserId(r.uploader_id),
            storage_key: r.storage_key,
            ticket_id: r.ticket_id.map(TicketId),
            comment_id: r.comment_id,
            internal: r.internal,
        }
    }
}

/// Links verified drafts inside the caller's transaction. Fails with
/// `Conflict` (rolling everything back) if any of them was linked meanwhile.
pub(super) async fn link_attachments(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[AttachmentId],
    ticket_id: TicketId,
    comment_id: Option<Uuid>,
    at: DateTime<Utc>,
) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let raw: Vec<Uuid> = ids.iter().map(|a| a.0).collect();
    let linked = sqlx::query!(
        "UPDATE attachments SET ticket_id = $2, comment_id = $3, linked_at = $4 WHERE id = ANY($1) AND ticket_id IS NULL",
        &raw,
        ticket_id.0,
        comment_id,
        at,
    )
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if linked as usize != ids.len() {
        return Err(AppError::Conflict("One of the files was already sent. Remove it and try again.".into()));
    }
    Ok(())
}

#[async_trait]
impl AttachmentRepository for PgAttachmentRepository {
    async fn insert_draft(&self, a: NewAttachment) -> AppResult<AttachmentRecord> {
        let row = sqlx::query_as!(
            Row,
            r#"INSERT INTO attachments (id, uploader_id, storage_key, filename, content_type, size_bytes, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id, uploader_id, ticket_id, comment_id, false AS "internal!", storage_key, filename,
                         content_type, size_bytes, created_at"#,
            a.id.0,
            a.uploader_id.0,
            a.storage_key,
            a.filename.as_str(),
            a.content_type.as_str(),
            a.size,
            a.created_at,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn find(&self, id: AttachmentId) -> AppResult<Option<AttachmentRecord>> {
        Ok(self.find_many(&[id]).await?.into_iter().next())
    }

    async fn find_many(&self, ids: &[AttachmentId]) -> AppResult<Vec<AttachmentRecord>> {
        let raw: Vec<Uuid> = ids.iter().map(|a| a.0).collect();
        let rows = sqlx::query_as!(
            Row,
            r#"SELECT a.id AS "id!", a.uploader_id AS "uploader_id!", a.ticket_id, a.comment_id,
                      COALESCE(c.is_internal, false) AS "internal!", a.storage_key AS "storage_key!",
                      a.filename AS "filename!", a.content_type AS "content_type!", a.size_bytes AS "size_bytes!",
                      a.created_at AS "created_at!"
               FROM attachments a
               LEFT JOIN ticket_comments c ON c.id = a.comment_id
               WHERE a.id = ANY($1)"#,
            &raw,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn for_ticket(&self, ticket_id: TicketId) -> AppResult<Vec<TicketAttachment>> {
        let rows = sqlx::query!(
            r#"SELECT id, comment_id, filename, content_type, size_bytes, created_at
               FROM attachments WHERE ticket_id = $1
               ORDER BY created_at, filename"#,
            ticket_id.0,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| TicketAttachment {
                view: AttachmentView {
                    id: AttachmentId(r.id),
                    filename: r.filename,
                    content_type: r.content_type,
                    size: r.size_bytes,
                    created_at: r.created_at,
                },
                comment_id: r.comment_id,
            })
            .collect())
    }
}
