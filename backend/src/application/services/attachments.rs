use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use uuid::Uuid;

use crate::{
    application::{
        error::{AppError, AppResult},
        ports::{
            inbound::{AttachmentUseCases, StartUploadInput, UploadSlot},
            outbound::{AttachmentRecord, AttachmentRepository, Clock, FileStorage, NewAttachment, TicketRepository},
        },
        principal::Principal,
    },
    domain::attachment::{
        AttachmentId, ContentType, FileName, MAX_FILES_PER_MESSAGE, validate_size,
    },
};

const UPLOAD_TTL_MINUTES: i64 = 15;
const DOWNLOAD_TTL_MINUTES: i64 = 5;

pub struct AttachmentService {
    attachments: Arc<dyn AttachmentRepository>,
    tickets: Arc<dyn TicketRepository>,
    storage: Arc<dyn FileStorage>,
    clock: Arc<dyn Clock>,
}

impl AttachmentService {
    pub fn new(
        attachments: Arc<dyn AttachmentRepository>,
        tickets: Arc<dyn TicketRepository>,
        storage: Arc<dyn FileStorage>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { attachments, tickets, storage, clock }
    }

    /// Whether `actor` may see this attachment. Answers `NotFound` rather
    /// than `Forbidden` so ids reveal nothing.
    async fn ensure_visible(&self, actor: &Principal, record: &AttachmentRecord) -> AppResult<()> {
        let Some(ticket_id) = record.ticket_id else {
            // Drafts belong to whoever uploaded them.
            return if record.uploader_id == actor.user_id { Ok(()) } else { Err(AppError::NotFound) };
        };
        if actor.role.is_staff() {
            return Ok(());
        }
        let ticket = self.tickets.find_by_id(ticket_id).await?.ok_or(AppError::NotFound)?;
        if ticket.requester_id == actor.user_id && !record.internal { Ok(()) } else { Err(AppError::NotFound) }
    }
}

#[async_trait]
impl AttachmentUseCases for AttachmentService {
    async fn start_upload(&self, actor: &Principal, input: StartUploadInput) -> AppResult<UploadSlot> {
        let filename = FileName::parse(&input.filename)?;
        let content_type = ContentType::parse_or_default(&input.content_type);
        validate_size(input.size)?;

        let id = AttachmentId(Uuid::new_v4());
        // The key never contains user input; the name travels in metadata.
        let storage_key = format!("attachments/{}", id.0);
        let ttl = Duration::minutes(UPLOAD_TTL_MINUTES);
        let now = self.clock.now();
        let put = self.storage.presign_put(&storage_key, content_type.as_str(), ttl).await?;
        let record = self
            .attachments
            .insert_draft(NewAttachment {
                id,
                uploader_id: actor.user_id,
                storage_key,
                filename,
                content_type,
                size: input.size,
                created_at: now,
            })
            .await?;
        Ok(UploadSlot { attachment: record.view, upload_url: put.url, upload_headers: put.headers, expires_at: now + ttl })
    }

    async fn download_url(&self, actor: &Principal, id: AttachmentId) -> AppResult<String> {
        let record = self.attachments.find(id).await?.ok_or(AppError::NotFound)?;
        self.ensure_visible(actor, &record).await?;
        self.storage
            .presign_get(
                &record.storage_key,
                &record.view.filename,
                &record.view.content_type,
                Duration::minutes(DOWNLOAD_TTL_MINUTES),
            )
            .await
    }
}

/// Checks that drafts listed in a new ticket or reply may be attached:
/// uploaded by the actor, not used before, and actually present in storage
/// at the declared size. Shared by the ticket service.
pub(crate) async fn verify_drafts(
    actor: &Principal,
    ids: &[AttachmentId],
    attachments: &dyn AttachmentRepository,
    storage: &dyn FileStorage,
) -> AppResult<Vec<AttachmentId>> {
    let mut unique: Vec<AttachmentId> = Vec::with_capacity(ids.len());
    for id in ids {
        if !unique.contains(id) {
            unique.push(*id);
        }
    }
    if unique.len() > MAX_FILES_PER_MESSAGE {
        return Err(AppError::Validation(format!("Attach up to {MAX_FILES_PER_MESSAGE} files per message.")));
    }
    if unique.is_empty() {
        return Ok(unique);
    }

    let records = attachments.find_many(&unique).await?;
    let not_ready = || {
        AppError::Validation("One of the files hasn't finished uploading or was already sent. Remove it and try again.".into())
    };
    if records.len() != unique.len() {
        return Err(not_ready());
    }
    for record in &records {
        if record.uploader_id != actor.user_id || record.ticket_id.is_some() {
            return Err(not_ready());
        }
        match storage.object_size(&record.storage_key).await? {
            Some(size) if size == record.view.size => {}
            _ => return Err(not_ready()),
        }
    }
    Ok(unique)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{
            ports::inbound::{AddCommentInput, CreateTicketInput, TicketUseCases, TimelineItem},
            services::{TicketService, test_support::*},
        },
        domain::user::Role,
    };

    struct Env {
        base: TestEnv,
        tickets: TicketService,
        files: AttachmentService,
    }

    fn env() -> Env {
        let base = TestEnv::default();
        let tickets = base.ticket_service();
        let files = AttachmentService::new(base.attachments.clone(), base.tickets.clone(), base.storage.clone(), base.clock.clone());
        Env { base, tickets, files }
    }

    fn file(name: &str, size: i64) -> StartUploadInput {
        StartUploadInput { filename: name.into(), content_type: "image/png".into(), size }
    }

    fn ticket_with(ids: Vec<AttachmentId>) -> CreateTicketInput {
        CreateTicketInput {
            subject: "App crashes on images".into(),
            description: "Screenshot attached.".into(),
            attachment_ids: ids,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn upload_then_attach_to_new_ticket() {
        let e = env();
        let jordan = e.base.seed(Role::Customer).await;
        let slot = e.files.start_upload(&jordan, file("C:\\shots\\crash.png", 2048)).await.unwrap();
        assert_eq!(slot.attachment.filename, "crash.png");
        assert!(slot.upload_url.contains(&slot.attachment.id.0.to_string()));
        e.base.storage.simulate_upload(slot.attachment.id, 2048);

        let t = e.tickets.create(&jordan, ticket_with(vec![slot.attachment.id])).await.unwrap();
        assert_eq!(t.attachments.len(), 1);
        assert_eq!(t.attachments[0].filename, "crash.png");

        // A file can only be used once.
        let again = e.tickets.create(&jordan, ticket_with(vec![slot.attachment.id])).await;
        assert!(matches!(again, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn rejects_missing_wrong_size_foreign_and_too_many_files() {
        let e = env();
        let jordan = e.base.seed(Role::Customer).await;
        let sofia = e.base.seed(Role::Customer).await;

        let never_uploaded = e.files.start_upload(&jordan, file("a.png", 10)).await.unwrap().attachment.id;
        let wrong_size = e.files.start_upload(&jordan, file("b.png", 10)).await.unwrap().attachment.id;
        e.base.storage.simulate_upload(wrong_size, 99_999);
        let sofias = e.files.start_upload(&sofia, file("c.png", 10)).await.unwrap().attachment.id;
        e.base.storage.simulate_upload(sofias, 10);

        for ids in [vec![never_uploaded], vec![wrong_size], vec![sofias], vec![AttachmentId(Uuid::new_v4())]] {
            assert!(matches!(e.tickets.create(&jordan, ticket_with(ids)).await, Err(AppError::Validation(_))));
        }
        let many: Vec<_> = (0..11).map(|_| AttachmentId(Uuid::new_v4())).collect();
        assert!(matches!(e.tickets.create(&jordan, ticket_with(many)).await, Err(AppError::Validation(m)) if m.contains("up to 10")));
    }

    #[tokio::test]
    async fn start_upload_validates_input() {
        let e = env();
        let jordan = e.base.seed(Role::Customer).await;
        assert!(matches!(e.files.start_upload(&jordan, file("big.zip", 26 * 1024 * 1024)).await, Err(AppError::Validation(_))));
        assert!(matches!(e.files.start_upload(&jordan, file("   ", 10)).await, Err(AppError::Validation(_))));
        let odd = StartUploadInput { filename: "x.bin".into(), content_type: "not a type".into(), size: 1 };
        assert_eq!(e.files.start_upload(&jordan, odd).await.unwrap().attachment.content_type, "application/octet-stream");
    }

    #[tokio::test]
    async fn download_permissions_follow_the_conversation() {
        let e = env();
        let jordan = e.base.seed(Role::Customer).await;
        let sofia = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;

        let public = e.files.start_upload(&jordan, file("statement.pdf", 5)).await.unwrap().attachment.id;
        e.base.storage.simulate_upload(public, 5);
        let n = e.tickets.create(&jordan, ticket_with(vec![public])).await.unwrap().summary.number;

        let secret = e.files.start_upload(&agent, file("internal-log.txt", 7)).await.unwrap().attachment.id;
        e.base.storage.simulate_upload(secret, 7);
        let note = AddCommentInput { body: "Server log attached".into(), internal: true, attachment_ids: vec![secret], ..Default::default() };
        let staff_view = e.tickets.add_comment(&agent, n, note).await.unwrap();
        let TimelineItem::Comment { attachments, .. } = &staff_view.timeline[0] else { panic!("expected a comment") };
        assert_eq!(attachments.len(), 1);

        assert!(e.files.download_url(&jordan, public).await.unwrap().contains("download"));
        assert!(e.files.download_url(&agent, secret).await.is_ok());
        assert!(matches!(e.files.download_url(&jordan, secret).await, Err(AppError::NotFound)), "internal note file");
        assert!(matches!(e.files.download_url(&sofia, public).await, Err(AppError::NotFound)), "other customer");

        let draft = e.files.start_upload(&sofia, file("draft.png", 3)).await.unwrap().attachment.id;
        assert!(e.files.download_url(&sofia, draft).await.is_ok());
        assert!(matches!(e.files.download_url(&agent, draft).await, Err(AppError::NotFound)), "someone else's draft");
    }
}
