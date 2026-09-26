use anyhow::{Context, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::attachment_repository::link_attachments;
use crate::{
    application::{
        error::{AppError, AppResult},
        ports::{
            inbound::{EventKind, Page, TicketSort, TicketSummary, TimelineItem, UserRef, ViewCounts},
            outbound::{NewComment, NewEvent, NewTicket, TicketQuery, TicketRepository},
        },
    },
    domain::{
        ticket::{MessageBody, Subject, Ticket, TicketId, TicketNumber},
        user::UserId,
    },
};

pub struct PgTicketRepository {
    pool: PgPool,
}

impl PgTicketRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn parse<T: std::str::FromStr>(value: &str, what: &str) -> AppResult<T> {
    value.parse().map_err(|_| AppError::Unexpected(anyhow!("invalid {what} in database: {value}")))
}

fn user_ref(id: Uuid, name: String, email: String, role: &str) -> AppResult<UserRef> {
    Ok(UserRef { id: UserId(id), name, email, role: parse(role, "role")? })
}

struct TicketRow {
    id: Uuid,
    number: i64,
    subject: String,
    description: String,
    status: String,
    priority: String,
    channel: String,
    requester_id: Uuid,
    assignee_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

impl TryFrom<TicketRow> for Ticket {
    type Error = AppError;

    fn try_from(r: TicketRow) -> AppResult<Self> {
        Ok(Ticket {
            id: TicketId(r.id),
            number: TicketNumber(r.number),
            subject: Subject::parse(&r.subject).context("invalid subject in database")?,
            description: MessageBody::parse(&r.description).context("invalid description in database")?,
            status: parse(&r.status, "status")?,
            priority: parse(&r.priority, "priority")?,
            channel: parse(&r.channel, "channel")?,
            requester_id: UserId(r.requester_id),
            assignee_id: r.assignee_id.map(UserId),
            created_at: r.created_at,
            updated_at: r.updated_at,
            resolved_at: r.resolved_at,
        })
    }
}

struct SummaryRow {
    id: Uuid,
    number: i64,
    subject: String,
    status: String,
    priority: String,
    channel: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    requester_id: Uuid,
    requester_name: String,
    requester_email: String,
    requester_role: String,
    assignee_id: Option<Uuid>,
    assignee_name: Option<String>,
    assignee_email: Option<String>,
    assignee_role: Option<String>,
}

impl TryFrom<SummaryRow> for TicketSummary {
    type Error = AppError;

    fn try_from(r: SummaryRow) -> AppResult<Self> {
        let assignee = match (r.assignee_id, r.assignee_name, r.assignee_email, r.assignee_role) {
            (Some(id), Some(name), Some(email), Some(role)) => Some(user_ref(id, name, email, &role)?),
            _ => None,
        };
        Ok(TicketSummary {
            id: TicketId(r.id),
            number: TicketNumber(r.number),
            subject: r.subject,
            status: parse(&r.status, "status")?,
            priority: parse(&r.priority, "priority")?,
            channel: parse(&r.channel, "channel")?,
            requester: user_ref(r.requester_id, r.requester_name, r.requester_email, &r.requester_role)?,
            assignee,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

/// `%term%` for ILIKE, with the pattern characters escaped.
fn like_pattern(term: &str) -> String {
    let escaped = term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

/// "1042", "#1042" and "TKT-1042" all find ticket 1042.
fn ticket_number_in(term: &str) -> Option<i64> {
    let t = term.trim();
    let digits = t
        .strip_prefix('#')
        .or_else(|| t.get(..4).filter(|p| p.eq_ignore_ascii_case("tkt-")).map(|_| &t[4..]))
        .unwrap_or(t);
    digits.parse().ok()
}

async fn save_ticket(tx: &mut Transaction<'_, Postgres>, t: &Ticket) -> AppResult<()> {
    sqlx::query!(
        r#"UPDATE tickets
           SET status = $2::text::ticket_status, priority = $3::text::ticket_priority,
               assignee_id = $4, updated_at = $5, resolved_at = $6
           WHERE id = $1"#,
        t.id.0,
        t.status.as_str(),
        t.priority.as_str(),
        t.assignee_id.map(|u| u.0),
        t.updated_at,
        t.resolved_at,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_events(tx: &mut Transaction<'_, Postgres>, ticket_id: TicketId, events: &[NewEvent]) -> AppResult<()> {
    for e in events {
        sqlx::query!(
            r#"INSERT INTO ticket_events (ticket_id, actor_id, kind, old_value, new_value, created_at)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
            ticket_id.0,
            e.actor_id.map(|u| u.0),
            e.kind.as_str(),
            e.old_value,
            e.new_value,
            e.created_at,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

#[async_trait]
impl TicketRepository for PgTicketRepository {
    async fn insert(&self, t: NewTicket) -> AppResult<Ticket> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query_as!(
            TicketRow,
            r#"INSERT INTO tickets (subject, description, priority, channel, requester_id, assignee_id, created_at, updated_at)
               VALUES ($1, $2, $3::text::ticket_priority, $4::text::ticket_channel, $5, $6, $7, $7)
               RETURNING id, number, subject, description, status::text AS "status!", priority::text AS "priority!",
                         channel::text AS "channel!", requester_id, assignee_id, created_at, updated_at, resolved_at"#,
            t.subject.as_str(),
            t.description.as_str(),
            t.priority.as_str(),
            t.channel.as_str(),
            t.requester_id.0,
            t.assignee_id.map(|u| u.0),
            t.created_at,
        )
        .fetch_one(&mut *tx)
        .await?;
        let ticket = Ticket::try_from(row)?;
        link_attachments(&mut tx, &t.attachment_ids, ticket.id, None, t.created_at).await?;
        tx.commit().await?;
        Ok(ticket)
    }

    async fn find_by_number(&self, number: TicketNumber) -> AppResult<Option<Ticket>> {
        sqlx::query_as!(
            TicketRow,
            r#"SELECT id, number, subject, description, status::text AS "status!", priority::text AS "priority!",
                      channel::text AS "channel!", requester_id, assignee_id, created_at, updated_at, resolved_at
               FROM tickets WHERE number = $1"#,
            number.0,
        )
        .fetch_optional(&self.pool)
        .await?
        .map(Ticket::try_from)
        .transpose()
    }

    async fn find_by_id(&self, id: TicketId) -> AppResult<Option<Ticket>> {
        sqlx::query_as!(
            TicketRow,
            r#"SELECT id, number, subject, description, status::text AS "status!", priority::text AS "priority!",
                      channel::text AS "channel!", requester_id, assignee_id, created_at, updated_at, resolved_at
               FROM tickets WHERE id = $1"#,
            id.0,
        )
        .fetch_optional(&self.pool)
        .await?
        .map(Ticket::try_from)
        .transpose()
    }

    async fn update(&self, ticket: &Ticket, events: &[NewEvent]) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        save_ticket(&mut tx, ticket).await?;
        insert_events(&mut tx, ticket.id, events).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn add_comment(&self, ticket: &Ticket, c: NewComment, events: &[NewEvent]) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        save_ticket(&mut tx, ticket).await?;
        let comment_id = sqlx::query_scalar!(
            "INSERT INTO ticket_comments (ticket_id, author_id, body, is_internal, created_at)
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
            ticket.id.0,
            c.author_id.0,
            c.body.as_str(),
            c.internal,
            c.created_at,
        )
        .fetch_one(&mut *tx)
        .await?;
        link_attachments(&mut tx, &c.attachment_ids, ticket.id, Some(comment_id), c.created_at).await?;
        insert_events(&mut tx, ticket.id, events).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn summary(&self, id: TicketId) -> AppResult<TicketSummary> {
        sqlx::query_as!(
            SummaryRow,
            r#"SELECT t.id, t.number, t.subject, t.status::text AS "status!", t.priority::text AS "priority!",
                      t.channel::text AS "channel!", t.created_at, t.updated_at,
                      r.id AS requester_id, r.name AS requester_name, r.email AS requester_email,
                      r.role::text AS "requester_role!",
                      a.id AS "assignee_id?", a.name AS "assignee_name?", a.email AS "assignee_email?",
                      a.role::text AS "assignee_role?"
               FROM tickets t
               JOIN users r ON r.id = t.requester_id
               LEFT JOIN users a ON a.id = t.assignee_id
               WHERE t.id = $1"#,
            id.0,
        )
        .fetch_one(&self.pool)
        .await?
        .try_into()
    }

    async fn list(&self, q: &TicketQuery) -> AppResult<Page<TicketSummary>> {
        let statuses: Option<Vec<String>> =
            (!q.statuses.is_empty()).then(|| q.statuses.iter().map(|s| s.as_str().to_owned()).collect());
        let search = q.search.as_deref().map(like_pattern);
        let number = q.search.as_deref().and_then(ticket_number_in);

        let rows = sqlx::query!(
            r#"SELECT t.id, t.number, t.subject, t.status::text AS "status!", t.priority::text AS "priority!",
                      t.channel::text AS "channel!", t.created_at, t.updated_at,
                      r.id AS requester_id, r.name AS requester_name, r.email AS requester_email,
                      r.role::text AS "requester_role!",
                      a.id AS "assignee_id?", a.name AS "assignee_name?", a.email AS "assignee_email?",
                      a.role::text AS "assignee_role?",
                      COUNT(*) OVER () AS "total!"
               FROM tickets t
               JOIN users r ON r.id = t.requester_id
               LEFT JOIN users a ON a.id = t.assignee_id
               WHERE ($1::text[] IS NULL OR t.status::text = ANY($1))
                 AND ($2::text IS NULL OR t.priority::text = $2)
                 AND (NOT $3::bool OR t.assignee_id IS NULL)
                 AND ($4::uuid IS NULL OR t.assignee_id = $4)
                 AND ($5::uuid IS NULL OR t.requester_id = $5)
                 AND ($6::text IS NULL OR t.subject ILIKE $6 OR r.name ILIKE $6 OR r.email ILIKE $6 OR t.number = $7)
               ORDER BY
                 CASE WHEN $8::bool THEN t.priority END DESC,
                 CASE WHEN $8::bool THEN t.created_at END ASC,
                 t.updated_at DESC
               LIMIT $9 OFFSET $10"#,
            statuses.as_deref(),
            q.priority.map(|p| p.as_str()),
            q.unassigned_only,
            q.assignee_id.map(|u| u.0),
            q.requester_id.map(|u| u.0),
            search,
            number,
            q.sort == TicketSort::Priority,
            q.limit,
            q.offset,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = rows.first().map_or(0, |r| r.total);
        let items = rows
            .into_iter()
            .map(|r| {
                TicketSummary::try_from(SummaryRow {
                    id: r.id,
                    number: r.number,
                    subject: r.subject,
                    status: r.status,
                    priority: r.priority,
                    channel: r.channel,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                    requester_id: r.requester_id,
                    requester_name: r.requester_name,
                    requester_email: r.requester_email,
                    requester_role: r.requester_role,
                    assignee_id: r.assignee_id,
                    assignee_name: r.assignee_name,
                    assignee_email: r.assignee_email,
                    assignee_role: r.assignee_role,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Page { items, total, page: 0, per_page: 0 })
    }

    async fn timeline(&self, id: TicketId, include_internal: bool) -> AppResult<Vec<TimelineItem>> {
        // Comments sort before events with the same timestamp, so a reply
        // reads before the status change it caused.
        let rows = sqlx::query!(
            r#"SELECT item_id AS "id!", sort_key AS "sort_key!", created_at AS "created_at!",
                      body AS "body?", internal AS "internal?", kind AS "kind?",
                      old_value AS "old_value?", new_value AS "new_value?",
                      user_id AS "user_id?", user_name AS "user_name?", user_email AS "user_email?",
                      user_role AS "user_role?"
               FROM (
                 SELECT c.id AS item_id, 0 AS sort_key, c.created_at, c.body, c.is_internal AS internal,
                        NULL::text AS kind, NULL::text AS old_value, NULL::text AS new_value,
                        u.id AS user_id, u.name AS user_name, u.email AS user_email, u.role::text AS user_role
                 FROM ticket_comments c
                 JOIN users u ON u.id = c.author_id
                 WHERE c.ticket_id = $1 AND ($2 OR NOT c.is_internal)
                 UNION ALL
                 SELECT e.id, 1, e.created_at, NULL, NULL, e.kind,
                        CASE WHEN e.kind = 'assignee_changed' THEN ou.name ELSE e.old_value END,
                        CASE WHEN e.kind = 'assignee_changed' THEN nu.name ELSE e.new_value END,
                        u.id, u.name, u.email, u.role::text
                 FROM ticket_events e
                 LEFT JOIN users u ON u.id = e.actor_id
                 LEFT JOIN users ou ON e.kind = 'assignee_changed' AND ou.id::text = e.old_value
                 LEFT JOIN users nu ON e.kind = 'assignee_changed' AND nu.id::text = e.new_value
                 WHERE e.ticket_id = $1
               ) items
               ORDER BY created_at, sort_key"#,
            id.0,
            include_internal,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let user = match (r.user_id, r.user_name, r.user_email, r.user_role) {
                    (Some(id), Some(name), Some(email), Some(role)) => Some(user_ref(id, name, email, &role)?),
                    _ => None,
                };
                if r.sort_key == 0 {
                    Ok(TimelineItem::Comment {
                        id: r.id,
                        author: user.ok_or_else(|| anyhow!("comment without author"))?,
                        body: r.body.unwrap_or_default(),
                        internal: r.internal.unwrap_or(false),
                        // Filled in by the service from the attachment repository.
                        attachments: Vec::new(),
                        created_at: r.created_at,
                    })
                } else {
                    let kind = match r.kind.as_deref() {
                        Some("status_changed") => EventKind::StatusChanged,
                        Some("priority_changed") => EventKind::PriorityChanged,
                        Some("assignee_changed") => EventKind::AssigneeChanged,
                        other => return Err(AppError::Unexpected(anyhow!("unknown event kind {other:?}"))),
                    };
                    Ok(TimelineItem::Event {
                        id: r.id,
                        actor: user,
                        kind,
                        old_value: r.old_value,
                        new_value: r.new_value,
                        created_at: r.created_at,
                    })
                }
            })
            .collect()
    }

    async fn count_for_requester(&self, requester_id: UserId) -> AppResult<i64> {
        let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM tickets WHERE requester_id = $1"#, requester_id.0)
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }

    async fn view_counts(&self, me: UserId) -> AppResult<ViewCounts> {
        let r = sqlx::query!(
            r#"SELECT
                 count(*) FILTER (WHERE status IN ('open', 'pending', 'on_hold') AND assignee_id = $1) AS "mine!",
                 count(*) FILTER (WHERE status IN ('open', 'pending', 'on_hold') AND assignee_id IS NULL) AS "unassigned!",
                 count(*) FILTER (WHERE status IN ('open', 'pending', 'on_hold')) AS "open!",
                 count(*) FILTER (WHERE status IN ('solved', 'closed')) AS "solved!"
               FROM tickets"#,
            me.0,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(ViewCounts { mine: r.mine, unassigned: r.unassigned, open: r.open, solved: r.solved })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_terms() {
        assert_eq!(like_pattern("50%_off\\"), "%50\\%\\_off\\\\%");
        assert_eq!(ticket_number_in("1042"), Some(1042));
        assert_eq!(ticket_number_in("TKT-1042"), Some(1042));
        assert_eq!(ticket_number_in("tkt-7"), Some(7));
        assert_eq!(ticket_number_in("#12"), Some(12));
        assert_eq!(ticket_number_in("refund"), None);
    }
}
