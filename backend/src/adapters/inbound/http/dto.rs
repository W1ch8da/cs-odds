//! JSON request and response shapes. camelCase to match the TypeScript client.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::ports::inbound::{
        AuthSession, EventKind, Page, TicketDetail, TicketSummary, TimelineItem, UserRef, ViewCounts,
    },
    domain::user::User,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: &'static str,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self {
            id: u.id.0,
            email: u.email.as_str().to_owned(),
            name: u.name.as_str().to_owned(),
            role: u.role.as_str(),
            created_at: u.created_at,
        }
    }
}

/// Tokens travel in cookies only; the body tells the client who signed in
/// and when to refresh.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub user: UserDto,
    pub access_expires_at: DateTime<Utc>,
}

impl From<AuthSession> for SessionDto {
    fn from(s: AuthSession) -> Self {
        Self { user: s.user.into(), access_expires_at: s.access_expires_at }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub role: Option<String>,
}

// ---------- Tickets ----------


#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRefDto {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: &'static str,
}

impl From<UserRef> for UserRefDto {
    fn from(u: UserRef) -> Self {
        Self { id: u.id.0, name: u.name, email: u.email, role: u.role.as_str() }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketSummaryDto {
    pub number: i64,
    pub subject: String,
    pub status: &'static str,
    pub priority: &'static str,
    pub channel: &'static str,
    pub requester: UserRefDto,
    pub assignee: Option<UserRefDto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TicketSummary> for TicketSummaryDto {
    fn from(t: TicketSummary) -> Self {
        Self {
            number: t.number.0,
            subject: t.subject,
            status: t.status.as_str(),
            priority: t.priority.as_str(),
            channel: t.channel.as_str(),
            requester: t.requester.into(),
            assignee: t.assignee.map(Into::into),
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum TimelineItemDto {
    Comment { id: Uuid, author: UserRefDto, body: String, internal: bool, created_at: DateTime<Utc> },
    Event {
        id: Uuid,
        actor: Option<UserRefDto>,
        kind: &'static str,
        old_value: Option<String>,
        new_value: Option<String>,
        created_at: DateTime<Utc>,
    },
}

impl From<TimelineItem> for TimelineItemDto {
    fn from(item: TimelineItem) -> Self {
        match item {
            TimelineItem::Comment { id, author, body, internal, created_at } => {
                Self::Comment { id, author: author.into(), body, internal, created_at }
            }
            TimelineItem::Event { id, actor, kind, old_value, new_value, created_at } => Self::Event {
                id,
                actor: actor.map(Into::into),
                kind: EventKind::as_str(kind),
                old_value,
                new_value,
                created_at,
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketDetailDto {
    #[serde(flatten)]
    pub summary: TicketSummaryDto,
    pub description: String,
    pub resolved_at: Option<DateTime<Utc>>,
    pub timeline: Vec<TimelineItemDto>,
    pub requester_ticket_count: i64,
}

impl From<TicketDetail> for TicketDetailDto {
    fn from(d: TicketDetail) -> Self {
        Self {
            summary: d.summary.into(),
            description: d.description,
            resolved_at: d.resolved_at,
            timeline: d.timeline.into_iter().map(Into::into).collect(),
            requester_ticket_count: d.requester_ticket_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

impl From<Page<TicketSummary>> for PageDto<TicketSummaryDto> {
    fn from(p: Page<TicketSummary>) -> Self {
        Self { items: p.items.into_iter().map(Into::into).collect(), total: p.total, page: p.page, per_page: p.per_page }
    }
}

#[derive(Debug, Serialize)]
pub struct ViewCountsDto {
    pub mine: i64,
    pub unassigned: i64,
    pub open: i64,
    pub solved: i64,
}

impl From<ViewCounts> for ViewCountsDto {
    fn from(c: ViewCounts) -> Self {
        Self { mine: c.mine, unassigned: c.unassigned, open: c.open, solved: c.solved }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTicketRequest {
    pub subject: String,
    pub description: String,
    pub priority: Option<String>,
    pub requester_email: Option<String>,
    pub requester_name: Option<String>,
    pub assignee_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListTicketsQuery {
    /// Comma-separated statuses, e.g. `open,pending,on_hold`.
    pub status: Option<String>,
    pub priority: Option<String>,
    /// `me`, `unassigned` or a user id.
    pub assignee: Option<String>,
    pub q: Option<String>,
    /// `priority` (default) or `recent`.
    pub sort: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

/// Distinguishes a missing field (`None`) from an explicit `null` (`Some(None)`).
fn double_option<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(de).map(Some)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTicketRequest {
    pub status: Option<String>,
    pub priority: Option<String>,
    /// `null` unassigns; leaving it out keeps the current assignee.
    #[serde(default, deserialize_with = "double_option")]
    pub assignee_id: Option<Option<Uuid>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCommentRequest {
    pub body: String,
    #[serde(default)]
    pub internal: bool,
    /// Status to set together with the reply (staff only).
    pub status: Option<String>,
}
