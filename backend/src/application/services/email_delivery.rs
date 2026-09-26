use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;

use crate::application::{
    error::AppResult,
    ports::{
        inbound::{DeliverEmails, DeliveryReport},
        outbound::{EmailOutbox, MailSender},
    },
};

const BATCH_SIZE: i64 = 20;
/// How long a worker holds an email before another worker may retry it.
const LOCK_MINUTES: i64 = 5;
/// Wait after the n-th failed attempt. With 5 retries an email gets 6
/// attempts over about 7.5 hours, then is marked failed.
const BACKOFF_MINUTES: [i64; 5] = [1, 5, 15, 60, 360];

/// Sends queued email, retrying failures with backoff.
pub struct EmailDelivery {
    outbox: Arc<dyn EmailOutbox>,
    sender: Arc<dyn MailSender>,
}

impl EmailDelivery {
    pub fn new(outbox: Arc<dyn EmailOutbox>, sender: Arc<dyn MailSender>) -> Self {
        Self { outbox, sender }
    }
}

#[async_trait]
impl DeliverEmails for EmailDelivery {
    async fn deliver_due(&self) -> AppResult<DeliveryReport> {
        let batch = self.outbox.claim_due(Duration::minutes(LOCK_MINUTES), BATCH_SIZE).await?;
        let mut report = DeliveryReport::default();
        for queued in batch {
            match self.sender.send(&queued.email).await {
                Ok(()) => {
                    self.outbox.mark_sent(queued.id).await?;
                    report.sent += 1;
                }
                Err(err) => {
                    let message = format!("{err:?}");
                    let retry_in = usize::try_from(queued.attempts - 1)
                        .ok()
                        .and_then(|i| BACKOFF_MINUTES.get(i))
                        .map(|m| Duration::minutes(*m));
                    tracing::warn!(to = %queued.email.to_email, attempt = queued.attempts, gave_up = retry_in.is_none(), error = %message, "email delivery failed");
                    self.outbox.mark_failed(queued.id, &message, retry_in).await?;
                    if retry_in.is_some() { report.retrying += 1 } else { report.failed += 1 }
                }
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use uuid::Uuid;

    use super::*;
    use crate::application::{
        error::AppError,
        ports::outbound::{NewEmail, QueuedEmail},
    };

    #[derive(Default)]
    struct FakeOutbox {
        queued: Mutex<Vec<QueuedEmail>>,
        sent: Mutex<Vec<Uuid>>,
        failed: Mutex<Vec<(Uuid, Option<Duration>)>>,
    }

    #[async_trait]
    impl EmailOutbox for FakeOutbox {
        async fn enqueue(&self, _: &[NewEmail]) -> AppResult<()> {
            Ok(())
        }
        async fn claim_due(&self, _: Duration, _: i64) -> AppResult<Vec<QueuedEmail>> {
            Ok(std::mem::take(&mut *self.queued.lock().unwrap()))
        }
        async fn mark_sent(&self, id: Uuid) -> AppResult<()> {
            self.sent.lock().unwrap().push(id);
            Ok(())
        }
        async fn mark_failed(&self, id: Uuid, _: &str, retry_in: Option<Duration>) -> AppResult<()> {
            self.failed.lock().unwrap().push((id, retry_in));
            Ok(())
        }
    }

    /// Fails for addresses containing "bounce".
    struct FakeSender;

    #[async_trait]
    impl MailSender for FakeSender {
        async fn send(&self, email: &NewEmail) -> AppResult<()> {
            if email.to_email.contains("bounce") { Err(AppError::Unexpected(anyhow::anyhow!("550 no such user"))) } else { Ok(()) }
        }
    }

    fn queued(to: &str, attempts: i32) -> QueuedEmail {
        QueuedEmail {
            id: Uuid::new_v4(),
            attempts,
            email: NewEmail {
                to_email: to.into(),
                to_name: "Someone".into(),
                subject: "s".into(),
                body_text: "t".into(),
                body_html: "h".into(),
                message_id: "<m@x>".into(),
                ticket_id: None,
            },
        }
    }

    #[tokio::test]
    async fn sends_retries_with_backoff_then_gives_up() {
        let outbox = Arc::new(FakeOutbox::default());
        let ok = queued("jordan@example.com", 1);
        let first_failure = queued("bounce@example.com", 1);
        let fourth_failure = queued("bounce@example.com", 4);
        let last_failure = queued("bounce@example.com", 6);
        *outbox.queued.lock().unwrap() = vec![ok.clone(), first_failure.clone(), fourth_failure.clone(), last_failure.clone()];

        let delivery = EmailDelivery::new(outbox.clone(), Arc::new(FakeSender));
        let report = delivery.deliver_due().await.unwrap();
        assert_eq!(report, DeliveryReport { sent: 1, retrying: 2, failed: 1 });

        assert_eq!(*outbox.sent.lock().unwrap(), [ok.id]);
        let failed = outbox.failed.lock().unwrap().clone();
        assert_eq!(failed, [
            (first_failure.id, Some(Duration::minutes(1))),
            (fourth_failure.id, Some(Duration::minutes(60))),
            (last_failure.id, None),
        ]);
    }
}
