use async_trait::async_trait;

use crate::application::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeliveryReport {
    pub sent: usize,
    /// Failed this time, will be retried later.
    pub retrying: usize,
    /// Failed for the last time.
    pub failed: usize,
}

/// Driven by a timer (the background worker), not by users.
#[async_trait]
pub trait DeliverEmails: Send + Sync {
    /// Sends one batch of due emails.
    async fn deliver_due(&self) -> AppResult<DeliveryReport>;
}
