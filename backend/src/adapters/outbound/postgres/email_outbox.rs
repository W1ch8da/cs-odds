use async_trait::async_trait;
use chrono::Duration;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        error::AppResult,
        ports::outbound::{EmailOutbox, NewEmail, QueuedEmail},
    },
    domain::ticket::TicketId,
};

pub struct PgEmailOutbox {
    pool: PgPool,
}

impl PgEmailOutbox {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmailOutbox for PgEmailOutbox {
    async fn enqueue(&self, emails: &[NewEmail]) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        for e in emails {
            sqlx::query!(
                r#"INSERT INTO email_outbox (to_email, to_name, subject, body_text, body_html, message_id, ticket_id)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
                e.to_email,
                e.to_name,
                e.subject,
                e.body_text,
                e.body_html,
                e.message_id,
                e.ticket_id.map(|t| t.0),
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn claim_due(&self, lock_for: Duration, limit: i64) -> AppResult<Vec<QueuedEmail>> {
        // All queue times use the database clock. SKIP LOCKED lets several
        // workers run without sending anything twice.
        let rows = sqlx::query!(
            r#"UPDATE email_outbox
               SET status = 'sending', attempts = attempts + 1, locked_until = now() + make_interval(secs => $1)
               WHERE id IN (
                   SELECT id FROM email_outbox
                   WHERE (status = 'pending' AND next_attempt_at <= now())
                      OR (status = 'sending' AND locked_until < now())
                   ORDER BY next_attempt_at
                   LIMIT $2
                   FOR UPDATE SKIP LOCKED
               )
               RETURNING id, to_email, to_name, subject, body_text, body_html, message_id, ticket_id, attempts"#,
            lock_for.num_seconds() as f64,
            limit,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| QueuedEmail {
                id: r.id,
                attempts: r.attempts,
                email: NewEmail {
                    to_email: r.to_email,
                    to_name: r.to_name,
                    subject: r.subject,
                    body_text: r.body_text,
                    body_html: r.body_html,
                    message_id: r.message_id,
                    ticket_id: r.ticket_id.map(TicketId),
                },
            })
            .collect())
    }

    async fn mark_sent(&self, id: Uuid) -> AppResult<()> {
        sqlx::query!(
            "UPDATE email_outbox SET status = 'sent', sent_at = now(), locked_until = NULL, last_error = NULL WHERE id = $1",
            id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_failed(&self, id: Uuid, error: &str, retry_in: Option<Duration>) -> AppResult<()> {
        sqlx::query!(
            r#"UPDATE email_outbox
               SET status = CASE WHEN $3::float8 IS NULL THEN 'failed'::email_status ELSE 'pending'::email_status END,
                   next_attempt_at = COALESCE(now() + make_interval(secs => $3), next_attempt_at),
                   last_error = $2,
                   locked_until = NULL
               WHERE id = $1"#,
            id,
            error,
            retry_in.map(|d| d.num_seconds() as f64),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
