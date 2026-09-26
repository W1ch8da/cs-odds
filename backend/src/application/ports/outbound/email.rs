use async_trait::async_trait;
use chrono::Duration;
use uuid::Uuid;

use crate::{application::error::AppResult, domain::ticket::TicketId};

/// A fully rendered email, ready to queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEmail {
    pub to_email: String,
    pub to_name: String,
    pub subject: String,
    pub body_text: String,
    pub body_html: String,
    /// `<id@domain>`, unique per email.
    pub message_id: String,
    pub ticket_id: Option<TicketId>,
}

/// An email claimed for delivery.
#[derive(Debug, Clone)]
pub struct QueuedEmail {
    pub id: Uuid,
    pub email: NewEmail,
    /// Including the current attempt.
    pub attempts: i32,
}

/// Durable queue of outgoing email. The queue keeps its own clock (all times
/// are relative), so app servers and the database never disagree about what
/// is due.
#[async_trait]
pub trait EmailOutbox: Send + Sync {
    async fn enqueue(&self, emails: &[NewEmail]) -> AppResult<()>;
    /// Locks up to `limit` due emails for this worker for `lock_for`, and
    /// counts the attempt. Emails abandoned by a crashed worker become due
    /// again once their lock expires.
    async fn claim_due(&self, lock_for: Duration, limit: i64) -> AppResult<Vec<QueuedEmail>>;
    async fn mark_sent(&self, id: Uuid) -> AppResult<()>;
    /// `retry_in: None` gives up on the email for good.
    async fn mark_failed(&self, id: Uuid, error: &str, retry_in: Option<Duration>) -> AppResult<()>;
}

/// Delivers one email (SMTP).
#[async_trait]
pub trait MailSender: Send + Sync {
    async fn send(&self, email: &NewEmail) -> AppResult<()>;
}
