use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    application::{
        error::AppResult,
        ports::inbound::{EventKind, Page, TicketSort, TicketSummary, TimelineItem, ViewCounts},
    },
    domain::{
        ticket::{Channel, MessageBody, Priority, Status, Subject, Ticket, TicketId, TicketNumber},
        user::UserId,
    },
};

#[derive(Debug, Clone)]
pub struct NewTicket {
    pub subject: Subject,
    pub description: MessageBody,
    pub priority: Priority,
    pub channel: Channel,
    pub requester_id: UserId,
    pub assignee_id: Option<UserId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewComment {
    pub author_id: UserId,
    pub body: MessageBody,
    pub internal: bool,
    pub created_at: DateTime<Utc>,
}

/// An audit entry. Assignee values are user ids; the read side resolves names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvent {
    pub actor_id: Option<UserId>,
    pub kind: EventKind,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Fully resolved filters: the service has already applied permissions.
#[derive(Debug, Clone, Default)]
pub struct TicketQuery {
    pub statuses: Vec<Status>,
    pub priority: Option<Priority>,
    pub unassigned_only: bool,
    pub assignee_id: Option<UserId>,
    pub requester_id: Option<UserId>,
    pub search: Option<String>,
    pub sort: TicketSort,
    pub limit: i64,
    pub offset: i64,
}

/// Each write method is one transaction, so a ticket and its history never
/// disagree.
#[async_trait]
pub trait TicketRepository: Send + Sync {
    async fn insert(&self, ticket: NewTicket) -> AppResult<Ticket>;
    async fn find_by_number(&self, number: TicketNumber) -> AppResult<Option<Ticket>>;
    /// Saves the ticket's current fields and appends the events.
    async fn update(&self, ticket: &Ticket, events: &[NewEvent]) -> AppResult<()>;
    /// Adds the comment, saves the ticket and appends the events.
    async fn add_comment(&self, ticket: &Ticket, comment: NewComment, events: &[NewEvent]) -> AppResult<()>;

    async fn summary(&self, id: TicketId) -> AppResult<TicketSummary>;
    async fn list(&self, query: &TicketQuery) -> AppResult<Page<TicketSummary>>;
    /// Oldest first. Internal notes are left out unless asked for.
    async fn timeline(&self, id: TicketId, include_internal: bool) -> AppResult<Vec<TimelineItem>>;
    async fn count_for_requester(&self, requester_id: UserId) -> AppResult<i64>;
    async fn view_counts(&self, me: UserId) -> AppResult<ViewCounts>;
}
