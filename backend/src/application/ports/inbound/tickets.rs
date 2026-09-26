use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::attachments::AttachmentView;
use crate::{
    application::{error::AppResult, principal::Principal},
    domain::{
        attachment::AttachmentId,
        ticket::{Channel, Priority, Status, TicketId, TicketNumber},
        user::{Role, UserId},
    },
};

// ---------- Inputs ----------

#[derive(Debug, Clone, Default)]
pub struct CreateTicketInput {
    pub subject: String,
    pub description: String,
    pub priority: Option<Priority>,
    /// Staff only: the customer the ticket is for (found or created by email).
    pub requester_email: Option<String>,
    pub requester_name: Option<String>,
    /// Staff only.
    pub assignee_id: Option<UserId>,
    /// Uploaded drafts to attach to the description.
    pub attachment_ids: Vec<AttachmentId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssigneeFilter {
    Me,
    Unassigned,
    User(UserId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TicketSort {
    /// Most urgent first, then oldest first (a fair work queue).
    #[default]
    Priority,
    /// Most recently updated first.
    Recent,
}

#[derive(Debug, Clone, Default)]
pub struct ListTicketsInput {
    /// Empty means any status.
    pub statuses: Vec<Status>,
    pub priority: Option<Priority>,
    pub assignee: Option<AssigneeFilter>,
    /// Matches subject, requester name or email, or the ticket number.
    pub search: Option<String>,
    pub sort: TicketSort,
    pub page: u32,
    pub per_page: u32,
}

/// Fields to change; `None` leaves a field as it is.
#[derive(Debug, Clone, Default)]
pub struct UpdateTicketInput {
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    /// `Some(None)` unassigns.
    pub assignee_id: Option<Option<UserId>>,
}

#[derive(Debug, Clone, Default)]
pub struct AddCommentInput {
    pub body: String,
    /// Staff only: visible to the team, never to the customer.
    pub internal: bool,
    /// Staff only: status to set together with the reply.
    pub status_after: Option<Status>,
    /// Uploaded drafts to attach to this message.
    pub attachment_ids: Vec<AttachmentId>,
}

// ---------- Read models ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRef {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct TicketSummary {
    pub id: TicketId,
    pub number: TicketNumber,
    pub subject: String,
    pub status: Status,
    pub priority: Priority,
    pub channel: Channel,
    pub requester: UserRef,
    pub assignee: Option<UserRef>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    StatusChanged,
    PriorityChanged,
    AssigneeChanged,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StatusChanged => "status_changed",
            Self::PriorityChanged => "priority_changed",
            Self::AssigneeChanged => "assignee_changed",
        }
    }
}

#[derive(Debug, Clone)]
pub enum TimelineItem {
    Comment {
        id: Uuid,
        author: UserRef,
        body: String,
        internal: bool,
        attachments: Vec<AttachmentView>,
        created_at: DateTime<Utc>,
    },
    /// For assignee changes the values are display names, otherwise codes
    /// such as `pending` or `high`.
    Event {
        id: Uuid,
        actor: Option<UserRef>,
        kind: EventKind,
        old_value: Option<String>,
        new_value: Option<String>,
        created_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone)]
pub struct TicketDetail {
    pub summary: TicketSummary,
    pub description: String,
    /// Files attached to the description.
    pub attachments: Vec<AttachmentView>,
    pub resolved_at: Option<DateTime<Utc>>,
    /// Oldest first. Customers get public replies and status changes only.
    pub timeline: Vec<TimelineItem>,
    /// How many tickets this requester has in total.
    pub requester_ticket_count: i64,
}

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

/// Sidebar counts for the agent workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewCounts {
    pub mine: i64,
    pub unassigned: i64,
    pub open: i64,
    pub solved: i64,
}

#[async_trait]
pub trait TicketUseCases: Send + Sync {
    /// Customers create tickets for themselves; staff create them for a customer.
    async fn create(&self, actor: &Principal, input: CreateTicketInput) -> AppResult<TicketDetail>;
    /// Customers only ever see their own tickets.
    async fn list(&self, actor: &Principal, input: ListTicketsInput) -> AppResult<Page<TicketSummary>>;
    async fn get(&self, actor: &Principal, number: TicketNumber) -> AppResult<TicketDetail>;
    /// Staff change anything; customers may only mark their own ticket solved.
    async fn update(&self, actor: &Principal, number: TicketNumber, input: UpdateTicketInput) -> AppResult<TicketDetail>;
    async fn add_comment(&self, actor: &Principal, number: TicketNumber, input: AddCommentInput) -> AppResult<TicketDetail>;
    /// Staff only.
    async fn view_counts(&self, actor: &Principal) -> AppResult<ViewCounts>;
}
