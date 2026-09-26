use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    application::{
        error::{AppError, AppResult},
        ports::{
            inbound::{
                AddCommentInput, AssigneeFilter, CreateTicketInput, EventKind, ListTicketsInput, Page, TicketDetail,
                TicketSummary, TicketUseCases, TimelineItem, UpdateTicketInput, ViewCounts,
            },
            outbound::{
                AttachmentRepository, Clock, FileStorage, NewComment, NewEvent, NewTicket, TicketQuery,
                TicketRepository, UserRepository,
            },
        },
        principal::Principal,
        services::{
            attachments::verify_drafts,
            notifications::{Notifier, TicketActivity},
        },
    },
    domain::{
        ticket::{Channel, MessageBody, Priority, Status, Subject, Ticket, TicketChange, TicketNumber},
        user::{Email, NewUser, PersonName, Role, UserId},
    },
};

const DEFAULT_PAGE_SIZE: u32 = 25;
const MAX_PAGE_SIZE: u32 = 100;

pub struct TicketService {
    tickets: Arc<dyn TicketRepository>,
    users: Arc<dyn UserRepository>,
    attachments: Arc<dyn AttachmentRepository>,
    storage: Arc<dyn FileStorage>,
    notifier: Arc<Notifier>,
    clock: Arc<dyn Clock>,
}

impl TicketService {
    pub fn new(
        tickets: Arc<dyn TicketRepository>,
        users: Arc<dyn UserRepository>,
        attachments: Arc<dyn AttachmentRepository>,
        storage: Arc<dyn FileStorage>,
        notifier: Arc<Notifier>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { tickets, users, attachments, storage, notifier, clock }
    }

    /// Queues notification emails for a change that was just saved.
    async fn notify(&self, actor: &Principal, ticket: &TicketDetail, activity: TicketActivity<'_>) {
        let name = match self.users.find_by_id(actor.user_id).await {
            Ok(Some(user)) => user.name.as_str().to_owned(),
            _ => "Our support team".to_owned(),
        };
        self.notifier.notify(actor, &name, ticket, activity).await;
    }

    /// Loads a ticket the actor may see. Customers get `NotFound` for other
    /// people's tickets, so ticket numbers don't reveal anything.
    async fn load(&self, actor: &Principal, number: TicketNumber) -> AppResult<Ticket> {
        let ticket = self.tickets.find_by_number(number).await?.ok_or(AppError::NotFound)?;
        if !actor.role.is_staff() && ticket.requester_id != actor.user_id {
            return Err(AppError::NotFound);
        }
        Ok(ticket)
    }

    async fn detail(&self, actor: &Principal, ticket: &Ticket) -> AppResult<TicketDetail> {
        let staff = actor.role.is_staff();
        let mut timeline = self.tickets.timeline(ticket.id, staff).await?;
        if !staff {
            // Customers see the conversation and status changes, not internal bookkeeping.
            timeline.retain(|item| match item {
                TimelineItem::Comment { internal, .. } => !internal,
                TimelineItem::Event { kind, .. } => *kind == EventKind::StatusChanged,
            });
        }
        // Put each file under the message it was sent with. Files on hidden
        // comments are dropped together with the comment.
        let mut description_files = Vec::new();
        let mut by_comment: std::collections::HashMap<uuid::Uuid, Vec<_>> = std::collections::HashMap::new();
        for file in self.attachments.for_ticket(ticket.id).await? {
            match file.comment_id {
                None => description_files.push(file.view),
                Some(comment_id) => by_comment.entry(comment_id).or_default().push(file.view),
            }
        }
        for item in &mut timeline {
            if let TimelineItem::Comment { id, attachments, .. } = item {
                *attachments = by_comment.remove(id).unwrap_or_default();
            }
        }

        Ok(TicketDetail {
            summary: self.tickets.summary(ticket.id).await?,
            description: ticket.description.as_str().to_owned(),
            attachments: description_files,
            resolved_at: ticket.resolved_at,
            timeline,
            requester_ticket_count: self.tickets.count_for_requester(ticket.requester_id).await?,
        })
    }

    async fn ensure_assignable(&self, user_id: UserId) -> AppResult<()> {
        match self.users.find_by_id(user_id).await? {
            Some(user) if user.role.is_staff() => Ok(()),
            _ => Err(AppError::Validation("Tickets can only be assigned to agents or admins.".into())),
        }
    }

    /// Staff-created tickets: find the customer by email, or create them
    /// without a password (they can register or reply by email later).
    async fn resolve_requester(&self, input: &CreateTicketInput) -> AppResult<UserId> {
        let raw_email = input.requester_email.as_deref().unwrap_or_default();
        if raw_email.trim().is_empty() {
            return Err(AppError::Validation("Enter the customer's email address.".into()));
        }
        let email = Email::parse(raw_email)?;
        if let Some(user) = self.users.find_by_email(&email).await? {
            if user.role.is_staff() {
                return Err(AppError::Validation(
                    "That email belongs to a team member. Enter the customer's email instead.".into(),
                ));
            }
            return Ok(user.id);
        }
        let name = input.requester_name.as_deref().unwrap_or_default();
        if name.trim().is_empty() {
            return Err(AppError::Validation("This is a new customer. Enter their name too.".into()));
        }
        let user = self
            .users
            .create(NewUser { email, name: PersonName::parse(name)?, role: Role::Customer, password_hash: None })
            .await?;
        Ok(user.id)
    }
}

fn event(actor: &Principal, change: TicketChange, at: DateTime<Utc>) -> NewEvent {
    let (kind, old_value, new_value) = match change {
        TicketChange::Status { from, to } => {
            (EventKind::StatusChanged, Some(from.as_str().to_owned()), Some(to.as_str().to_owned()))
        }
        TicketChange::Priority { from, to } => {
            (EventKind::PriorityChanged, Some(from.as_str().to_owned()), Some(to.as_str().to_owned()))
        }
        TicketChange::Assignee { from, to } => {
            (EventKind::AssigneeChanged, from.map(|u| u.0.to_string()), to.map(|u| u.0.to_string()))
        }
    };
    NewEvent { actor_id: Some(actor.user_id), kind, old_value, new_value, created_at: at }
}

#[async_trait]
impl TicketUseCases for TicketService {
    async fn create(&self, actor: &Principal, input: CreateTicketInput) -> AppResult<TicketDetail> {
        let subject = Subject::parse(&input.subject)?;
        let description = MessageBody::parse(&input.description)
            .map_err(|_| AppError::Validation("Describe the problem so the team can help.".into()))?;
        let priority = input.priority.unwrap_or(Priority::Normal);

        let (requester_id, channel, assignee_id) = if actor.role.is_staff() {
            if let Some(assignee) = input.assignee_id {
                self.ensure_assignable(assignee).await?;
            }
            (self.resolve_requester(&input).await?, Channel::Agent, input.assignee_id)
        } else {
            if input.assignee_id.is_some() || input.requester_email.is_some() {
                return Err(AppError::Forbidden);
            }
            (actor.user_id, Channel::Portal, None)
        };

        let attachment_ids = verify_drafts(actor, &input.attachment_ids, &*self.attachments, &*self.storage).await?;
        let ticket = self
            .tickets
            .insert(NewTicket {
                subject,
                description,
                priority,
                channel,
                requester_id,
                assignee_id,
                created_at: self.clock.now(),
                attachment_ids,
            })
            .await?;
        let detail = self.detail(actor, &ticket).await?;
        self.notify(actor, &detail, TicketActivity::Created).await;
        Ok(detail)
    }

    async fn list(&self, actor: &Principal, input: ListTicketsInput) -> AppResult<Page<TicketSummary>> {
        let per_page = match input.per_page {
            0 => DEFAULT_PAGE_SIZE,
            n => n.min(MAX_PAGE_SIZE),
        };
        let page = input.page.max(1);
        let mut query = TicketQuery {
            statuses: input.statuses,
            priority: input.priority,
            search: input.search.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()),
            sort: input.sort,
            limit: i64::from(per_page),
            offset: i64::from(page - 1) * i64::from(per_page),
            ..TicketQuery::default()
        };
        if actor.role.is_staff() {
            match input.assignee {
                Some(AssigneeFilter::Me) => query.assignee_id = Some(actor.user_id),
                Some(AssigneeFilter::User(id)) => query.assignee_id = Some(id),
                Some(AssigneeFilter::Unassigned) => query.unassigned_only = true,
                None => {}
            }
        } else {
            // Customers only ever see their own tickets, whatever they ask for.
            query.requester_id = Some(actor.user_id);
        }
        let mut result = self.tickets.list(&query).await?;
        result.page = page;
        result.per_page = per_page;
        Ok(result)
    }

    async fn get(&self, actor: &Principal, number: TicketNumber) -> AppResult<TicketDetail> {
        let ticket = self.load(actor, number).await?;
        self.detail(actor, &ticket).await
    }

    async fn update(&self, actor: &Principal, number: TicketNumber, input: UpdateTicketInput) -> AppResult<TicketDetail> {
        let mut ticket = self.load(actor, number).await?;
        let now = self.clock.now();

        if !actor.role.is_staff() {
            // Customers can only close the loop themselves: "my issue is resolved".
            let only_solving = input.status == Some(Status::Solved) && input.priority.is_none() && input.assignee_id.is_none();
            if !only_solving {
                return Err(AppError::Forbidden);
            }
            if !ticket.status.is_active() {
                return self.detail(actor, &ticket).await;
            }
        }

        if let Some(Some(assignee)) = input.assignee_id {
            self.ensure_assignable(assignee).await?;
        }

        let mut changes = Vec::new();
        if let Some(status) = input.status {
            changes.extend(ticket.set_status(status, now)?);
        }
        if let Some(priority) = input.priority {
            changes.extend(ticket.set_priority(priority, now)?);
        }
        if let Some(assignee) = input.assignee_id {
            changes.extend(ticket.assign(assignee, now)?);
        }

        if changes.is_empty() {
            return self.detail(actor, &ticket).await;
        }
        let events: Vec<_> = changes.iter().cloned().map(|c| event(actor, c, now)).collect();
        self.tickets.update(&ticket, &events).await?;
        let detail = self.detail(actor, &ticket).await?;
        self.notify(actor, &detail, TicketActivity::Updated { changes: &changes }).await;
        Ok(detail)
    }

    async fn add_comment(&self, actor: &Principal, number: TicketNumber, input: AddCommentInput) -> AppResult<TicketDetail> {
        let mut ticket = self.load(actor, number).await?;
        ticket.ensure_not_closed()?;
        let body = MessageBody::parse(&input.body)?;
        let attachment_ids = verify_drafts(actor, &input.attachment_ids, &*self.attachments, &*self.storage).await?;
        let now = self.clock.now();
        let mut changes = Vec::new();

        if actor.role.is_staff() {
            // Replying to a customer claims an unassigned ticket.
            if !input.internal && ticket.assignee_id.is_none() {
                changes.extend(ticket.assign(Some(actor.user_id), now)?);
            }
            if let Some(status) = input.status_after {
                changes.extend(ticket.set_status(status, now)?);
            }
        } else {
            if input.internal || input.status_after.is_some() {
                return Err(AppError::Forbidden);
            }
            changes.extend(ticket.on_customer_reply(now)?);
        }
        ticket.touch(now);

        let body_text = body.as_str().to_owned();
        let comment = NewComment { author_id: actor.user_id, body, internal: input.internal, created_at: now, attachment_ids };
        let events: Vec<_> = changes.iter().cloned().map(|c| event(actor, c, now)).collect();
        self.tickets.add_comment(&ticket, comment, &events).await?;
        let detail = self.detail(actor, &ticket).await?;
        let activity = TicketActivity::Commented { body: &body_text, internal: input.internal, changes: &changes };
        self.notify(actor, &detail, activity).await;
        Ok(detail)
    }

    async fn view_counts(&self, actor: &Principal) -> AppResult<ViewCounts> {
        actor.require_staff()?;
        self.tickets.view_counts(actor.user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::test_support::*;

    struct Env {
        base: TestEnv,
        tickets: Arc<InMemoryTickets>,
        svc: TicketService,
    }

    fn env() -> Env {
        let base = TestEnv::default();
        let tickets = base.tickets.clone();
        let svc = base.ticket_service();
        Env { base, tickets, svc }
    }

    fn portal_ticket() -> CreateTicketInput {
        CreateTicketInput {
            subject: "Password reset link expired".into(),
            description: "Every reset link says it has expired.".into(),
            priority: Some(Priority::High),
            ..Default::default()
        }
    }

    fn reply(body: &str) -> AddCommentInput {
        AddCommentInput { body: body.into(), ..Default::default() }
    }

    #[tokio::test]
    async fn customer_creates_portal_ticket_for_themselves() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let t = e.svc.create(&customer, portal_ticket()).await.unwrap();
        assert_eq!(t.summary.channel, Channel::Portal);
        assert_eq!(t.summary.requester.id, customer.user_id);
        assert_eq!(t.summary.status, Status::Open);
        assert_eq!(t.summary.priority, Priority::High);

        let sneaky = CreateTicketInput { requester_email: Some("other@example.com".into()), ..portal_ticket() };
        assert!(matches!(e.svc.create(&customer, sneaky).await, Err(AppError::Forbidden)));
    }

    #[tokio::test]
    async fn create_validates_subject_and_description() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let blank = CreateTicketInput { subject: "  ".into(), ..portal_ticket() };
        assert!(matches!(e.svc.create(&customer, blank).await, Err(AppError::Validation(_))));
        let no_body = CreateTicketInput { description: "".into(), ..portal_ticket() };
        assert!(matches!(e.svc.create(&customer, no_body).await, Err(AppError::Validation(m)) if m.contains("Describe")));
    }

    #[tokio::test]
    async fn agent_creates_ticket_for_new_or_existing_customer() {
        let e = env();
        let agent = e.base.seed(Role::Agent).await;
        let for_new = CreateTicketInput {
            requester_email: Some("Marcus@PinecrestLegal.com".into()),
            requester_name: Some("Marcus Lee".into()),
            assignee_id: Some(agent.user_id),
            ..portal_ticket()
        };
        let t = e.svc.create(&agent, for_new.clone()).await.unwrap();
        assert_eq!(t.summary.channel, Channel::Agent);
        assert_eq!(t.summary.requester.email, "marcus@pinecrestlegal.com");
        assert_eq!(t.summary.requester.role, Role::Customer);
        assert_eq!(t.summary.assignee.as_ref().map(|a| a.id), Some(agent.user_id));

        // Same email again reuses the customer; the name isn't needed.
        let again = CreateTicketInput { requester_name: None, ..for_new };
        let t2 = e.svc.create(&agent, again).await.unwrap();
        assert_eq!(t2.summary.requester.id, t.summary.requester.id);
        assert_eq!(t2.requester_ticket_count, 2);
    }

    #[tokio::test]
    async fn agent_create_rejects_bad_requesters_and_assignees() {
        let e = env();
        let agent = e.base.seed(Role::Agent).await;
        let customer = e.base.seed(Role::Customer).await;
        let staff_email = e.base.users.find_by_id(agent.user_id).await.unwrap().unwrap().email.as_str().to_owned();

        let cases = [
            CreateTicketInput { requester_email: None, ..portal_ticket() },
            CreateTicketInput { requester_email: Some(staff_email), ..portal_ticket() },
            CreateTicketInput { requester_email: Some("new@example.com".into()), ..portal_ticket() },
            CreateTicketInput {
                requester_email: Some("new@example.com".into()),
                requester_name: Some("New Person".into()),
                assignee_id: Some(customer.user_id),
                ..portal_ticket()
            },
        ];
        for input in cases {
            assert!(matches!(e.svc.create(&agent, input).await, Err(AppError::Validation(_))));
        }
    }

    #[tokio::test]
    async fn customers_only_see_their_own_tickets() {
        let e = env();
        let jordan = e.base.seed(Role::Customer).await;
        let sofia = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        let t = e.svc.create(&jordan, portal_ticket()).await.unwrap();
        e.svc.create(&sofia, portal_ticket()).await.unwrap();

        assert!(matches!(e.svc.get(&sofia, t.summary.number).await, Err(AppError::NotFound)));
        let mine = e.svc.list(&jordan, ListTicketsInput { assignee: Some(AssigneeFilter::Unassigned), ..Default::default() }).await.unwrap();
        assert_eq!(mine.total, 1);
        assert_eq!(e.svc.list(&agent, ListTicketsInput::default()).await.unwrap().total, 2);
    }

    #[tokio::test]
    async fn list_resolves_me_and_clamps_paging() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        e.svc.create(&customer, portal_ticket()).await.unwrap();
        let t = e.svc.create(&customer, portal_ticket()).await.unwrap();
        e.svc.update(&agent, t.summary.number, UpdateTicketInput { assignee_id: Some(Some(agent.user_id)), ..Default::default() }).await.unwrap();

        let mine = e.svc.list(&agent, ListTicketsInput { assignee: Some(AssigneeFilter::Me), per_page: 500, page: 0, ..Default::default() }).await.unwrap();
        assert_eq!((mine.total, mine.page, mine.per_page), (1, 1, 100));
        let query = e.tickets.last_query();
        assert_eq!(query.assignee_id, Some(agent.user_id));
        assert_eq!((query.limit, query.offset), (100, 0));
    }

    #[tokio::test]
    async fn internal_notes_and_bookkeeping_are_hidden_from_customers() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        let t = e.svc.create(&customer, portal_ticket()).await.unwrap();
        let n = t.summary.number;

        e.svc.add_comment(&agent, n, AddCommentInput { internal: true, ..reply("Known Outlook issue") }).await.unwrap();
        e.svc.update(&agent, n, UpdateTicketInput { priority: Some(Priority::Urgent), ..Default::default() }).await.unwrap();
        let staff_view = e.svc.add_comment(&agent, n, AddCommentInput { status_after: Some(Status::Pending), ..reply("Try this link") }).await.unwrap();
        // note, priority change, auto-assign, status change, reply
        assert_eq!(staff_view.timeline.len(), 5);

        let customer_view = e.svc.get(&customer, n).await.unwrap();
        let kinds: Vec<_> = customer_view
            .timeline
            .iter()
            .map(|i| match i {
                TimelineItem::Comment { internal, .. } => format!("comment internal={internal}"),
                TimelineItem::Event { kind, .. } => kind.as_str().to_owned(),
            })
            .collect();
        // A reply and the status change it made share a timestamp; the reply reads first.
        assert_eq!(kinds, ["comment internal=false", "status_changed"]);
    }

    #[tokio::test]
    async fn staff_reply_claims_ticket_and_sets_status() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        let n = e.svc.create(&customer, portal_ticket()).await.unwrap().summary.number;

        let t = e.svc.add_comment(&agent, n, AddCommentInput { status_after: Some(Status::Pending), ..reply("Try this") }).await.unwrap();
        assert_eq!(t.summary.status, Status::Pending);
        assert_eq!(t.summary.assignee.map(|a| a.id), Some(agent.user_id));

        // A note from someone else doesn't steal the ticket.
        let other = e.base.seed(Role::Agent).await;
        let t = e.svc.add_comment(&other, n, AddCommentInput { internal: true, ..reply("FYI") }).await.unwrap();
        assert_eq!(t.summary.assignee.map(|a| a.id), Some(agent.user_id));
    }

    #[tokio::test]
    async fn customer_reply_reopens_and_customer_cannot_post_notes() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        let n = e.svc.create(&customer, portal_ticket()).await.unwrap().summary.number;
        e.svc.add_comment(&agent, n, AddCommentInput { status_after: Some(Status::Solved), ..reply("Fixed!") }).await.unwrap();

        let t = e.svc.add_comment(&customer, n, reply("Still broken for me")).await.unwrap();
        assert_eq!(t.summary.status, Status::Open);
        assert_eq!(t.resolved_at, None);

        for bad in [AddCommentInput { internal: true, ..reply("x") }, AddCommentInput { status_after: Some(Status::Closed), ..reply("x") }] {
            assert!(matches!(e.svc.add_comment(&customer, n, bad).await, Err(AppError::Forbidden)));
        }
    }

    #[tokio::test]
    async fn customer_may_only_mark_solved() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let n = e.svc.create(&customer, portal_ticket()).await.unwrap().summary.number;

        let forbidden = [
            UpdateTicketInput { priority: Some(Priority::Urgent), ..Default::default() },
            UpdateTicketInput { status: Some(Status::Closed), ..Default::default() },
            UpdateTicketInput { status: Some(Status::Solved), assignee_id: Some(None), ..Default::default() },
        ];
        for input in forbidden {
            assert!(matches!(e.svc.update(&customer, n, input).await, Err(AppError::Forbidden)));
        }
        let t = e.svc.update(&customer, n, UpdateTicketInput { status: Some(Status::Solved), ..Default::default() }).await.unwrap();
        assert_eq!(t.summary.status, Status::Solved);
        assert!(t.resolved_at.is_some());
    }

    #[tokio::test]
    async fn closed_tickets_reject_changes_and_replies() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        let n = e.svc.create(&customer, portal_ticket()).await.unwrap().summary.number;
        e.svc.update(&agent, n, UpdateTicketInput { status: Some(Status::Closed), ..Default::default() }).await.unwrap();

        assert!(matches!(e.svc.add_comment(&customer, n, reply("hello?")).await, Err(AppError::Conflict(_))));
        let reopen = UpdateTicketInput { status: Some(Status::Open), ..Default::default() };
        assert!(matches!(e.svc.update(&agent, n, reopen).await, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn updates_record_events_only_for_real_changes() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        let n = e.svc.create(&customer, portal_ticket()).await.unwrap().summary.number;

        let same = UpdateTicketInput { status: Some(Status::Open), priority: Some(Priority::High), ..Default::default() };
        let t = e.svc.update(&agent, n, same).await.unwrap();
        assert!(t.timeline.is_empty());

        let change = UpdateTicketInput { status: Some(Status::OnHold), assignee_id: Some(Some(agent.user_id)), ..Default::default() };
        let t = e.svc.update(&agent, n, change).await.unwrap();
        assert_eq!(t.timeline.len(), 2);

        let bad_assignee = UpdateTicketInput { assignee_id: Some(Some(customer.user_id)), ..Default::default() };
        assert!(matches!(e.svc.update(&agent, n, bad_assignee).await, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn changes_queue_the_right_emails() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        let customer_email = e.base.users.find_by_id(customer.user_id).await.unwrap().unwrap().email.as_str().to_owned();
        let agent_email = e.base.users.find_by_id(agent.user_id).await.unwrap().unwrap().email.as_str().to_owned();

        let n = e.svc.create(&customer, portal_ticket()).await.unwrap().summary.number;
        let subject = format!("[#{n}] Password reset link expired");
        assert_eq!(e.base.outbox.take(), [(customer_email.clone(), subject.clone())]);

        // A note on an unassigned ticket emails nobody.
        e.svc.add_comment(&agent, n, AddCommentInput { internal: true, ..reply("Looking into it") }).await.unwrap();
        assert!(e.base.outbox.take().is_empty());

        // The reply reaches the customer (and assigns the agent, silently).
        e.svc.add_comment(&agent, n, AddCommentInput { status_after: Some(Status::Pending), ..reply("Try this") }).await.unwrap();
        assert_eq!(e.base.outbox.take(), [(customer_email.clone(), format!("Re: {subject}"))]);

        // The customer's answer reaches the assignee.
        e.svc.add_comment(&customer, n, reply("Worked!")).await.unwrap();
        assert_eq!(e.base.outbox.take(), [(agent_email, format!("Re: {subject}"))]);

        // No-op updates send nothing.
        e.svc.update(&agent, n, UpdateTicketInput { status: Some(Status::Open), ..Default::default() }).await.unwrap();
        assert!(e.base.outbox.take().is_empty());
    }

    #[tokio::test]
    async fn view_counts_are_staff_only() {
        let e = env();
        let customer = e.base.seed(Role::Customer).await;
        let agent = e.base.seed(Role::Agent).await;
        assert!(matches!(e.svc.view_counts(&customer).await, Err(AppError::Forbidden)));
        assert!(e.svc.view_counts(&agent).await.is_ok());
    }
}
