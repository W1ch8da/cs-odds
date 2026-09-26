//! In-memory fakes of outbound ports for service unit tests.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use uuid::Uuid;

use crate::{
    application::{
        error::{AppError, AppResult},
        ports::{
            inbound::{EventKind, Page, TicketSort, TicketSummary, TimelineItem, UserRef, ViewCounts},
            outbound::{
                AccessTokenCodec, Clock, IssuedToken, NewComment, NewEvent, NewTicket, OpaqueTokenGenerator,
                PasswordHasher, RefreshTokenRecord, RefreshTokenRepository, TicketQuery, TicketRepository,
                UserRepository,
            },
        },
        principal::Principal,
    },
    domain::{
        ticket::{Status, Ticket, TicketId, TicketNumber},
        user::{Email, NewUser, PersonName, Role, User, UserId},
    },
};

#[derive(Default)]
pub struct InMemoryUsers(Mutex<Vec<(User, Option<String>)>>);

#[async_trait]
impl UserRepository for InMemoryUsers {
    async fn create(&self, new: NewUser) -> AppResult<User> {
        let mut rows = self.0.lock().unwrap();
        if rows.iter().any(|(u, _)| u.email == new.email) {
            return Err(AppError::Conflict("An account with this email already exists.".into()));
        }
        let user = User { id: UserId(Uuid::new_v4()), email: new.email, name: new.name, role: new.role, created_at: Utc::now() };
        rows.push((user.clone(), new.password_hash));
        Ok(user)
    }

    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>> {
        Ok(self.0.lock().unwrap().iter().find(|(u, _)| u.id == id).map(|(u, _)| u.clone()))
    }

    async fn find_by_email(&self, email: &Email) -> AppResult<Option<User>> {
        Ok(self.0.lock().unwrap().iter().find(|(u, _)| &u.email == email).map(|(u, _)| u.clone()))
    }

    async fn find_credentials(&self, email: &Email) -> AppResult<Option<(User, Option<String>)>> {
        Ok(self.0.lock().unwrap().iter().find(|(u, _)| &u.email == email).cloned())
    }

    async fn list(&self, role: Option<Role>) -> AppResult<Vec<User>> {
        Ok(self.0.lock().unwrap().iter().map(|(u, _)| u.clone()).filter(|u| role.is_none_or(|r| u.role == r)).collect())
    }

    async fn any_with_role(&self, role: Role) -> AppResult<bool> {
        Ok(self.0.lock().unwrap().iter().any(|(u, _)| u.role == role))
    }
}

#[derive(Default)]
pub struct InMemoryRefreshTokens(Mutex<Vec<(String, RefreshTokenRecord)>>);

#[async_trait]
impl RefreshTokenRepository for InMemoryRefreshTokens {
    async fn insert(&self, user_id: UserId, token_hash: &str, expires_at: DateTime<Utc>) -> AppResult<()> {
        let record = RefreshTokenRecord { id: Uuid::new_v4(), user_id, expires_at, revoked_at: None };
        self.0.lock().unwrap().push((token_hash.to_string(), record));
        Ok(())
    }

    async fn find_by_hash(&self, token_hash: &str) -> AppResult<Option<RefreshTokenRecord>> {
        Ok(self.0.lock().unwrap().iter().find(|(h, _)| h == token_hash).map(|(_, r)| r.clone()))
    }

    async fn revoke(&self, id: Uuid, at: DateTime<Utc>) -> AppResult<bool> {
        let mut rows = self.0.lock().unwrap();
        match rows.iter_mut().find(|(_, r)| r.id == id && r.revoked_at.is_none()) {
            Some((_, r)) => {
                r.revoked_at = Some(at);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn revoke_all_for_user(&self, user_id: UserId, at: DateTime<Utc>) -> AppResult<()> {
        for (_, r) in self.0.lock().unwrap().iter_mut().filter(|(_, r)| r.user_id == user_id && r.revoked_at.is_none()) {
            r.revoked_at = Some(at);
        }
        Ok(())
    }
}

/// Reversible "hash" so tests stay fast.
pub struct FakeHasher;

#[async_trait]
impl PasswordHasher for FakeHasher {
    async fn hash(&self, password: &str) -> AppResult<String> {
        Ok(format!("hashed:{password}"))
    }
    async fn verify(&self, password: &str, hash: &str) -> AppResult<bool> {
        Ok(hash == format!("hashed:{password}"))
    }
}

/// Token format: `access:<user id>:<role>`.
pub struct FakeCodec;

impl AccessTokenCodec for FakeCodec {
    fn issue(&self, user: &User, now: DateTime<Utc>) -> AppResult<IssuedToken> {
        Ok(IssuedToken { token: format!("access:{}:{}", user.id, user.role.as_str()), expires_at: now + Duration::minutes(15) })
    }
    fn verify(&self, token: &str) -> AppResult<Principal> {
        let mut parts = token.split(':');
        let (Some("access"), Some(id), Some(role)) = (parts.next(), parts.next(), parts.next()) else {
            return Err(AppError::Unauthorized);
        };
        Ok(Principal {
            user_id: UserId(id.parse().map_err(|_| AppError::Unauthorized)?),
            role: role.parse().map_err(|_| AppError::Unauthorized)?,
        })
    }
}

#[derive(Default)]
pub struct SeqTokens(AtomicU64);

impl OpaqueTokenGenerator for SeqTokens {
    fn generate(&self) -> String {
        format!("refresh-{}", self.0.fetch_add(1, Ordering::SeqCst))
    }
    fn hash(&self, token: &str) -> String {
        format!("h({token})")
    }
}

pub struct FixedClock(Mutex<DateTime<Utc>>);

impl FixedClock {
    pub fn advance(&self, by: Duration) {
        *self.0.lock().unwrap() += by;
    }
}

impl Default for FixedClock {
    fn default() -> Self {
        Self(Mutex::new(Utc.with_ymd_and_hms(2026, 9, 26, 9, 0, 0).unwrap()))
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

#[derive(Default)]
pub struct TestEnv {
    pub users: Arc<InMemoryUsers>,
    pub refresh_tokens: Arc<InMemoryRefreshTokens>,
    pub clock: Arc<FixedClock>,
}

impl TestEnv {
    /// Inserts a user with the given role and returns them as a principal.
    pub async fn seed(&self, role: Role) -> Principal {
        let n = self.users.0.lock().unwrap().len();
        let user = self
            .users
            .create(NewUser {
                email: Email::parse(&format!("{}{n}@example.com", role.as_str())).unwrap(),
                name: PersonName::parse(&format!("{} {n}", role.as_str())).unwrap(),
                role,
                password_hash: Some("hashed:correct horse".into()),
            })
            .await
            .unwrap();
        Principal { user_id: user.id, role }
    }
}

enum Entry {
    Comment(Uuid, NewComment),
    Event(Uuid, NewEvent),
}

#[derive(Default)]
struct TicketState {
    tickets: Vec<Ticket>,
    entries: Vec<(TicketId, Entry)>,
    last_query: Option<TicketQuery>,
}

/// Tickets in memory. Timeline order is insertion order, which matches the
/// Postgres ordering for the fixed clock used in tests.
pub struct InMemoryTickets {
    users: Arc<InMemoryUsers>,
    state: Mutex<TicketState>,
}

impl InMemoryTickets {
    pub fn new(users: Arc<InMemoryUsers>) -> Self {
        Self { users, state: Mutex::default() }
    }

    pub fn last_query(&self) -> TicketQuery {
        self.state.lock().unwrap().last_query.clone().expect("list was called")
    }

    fn user_ref(&self, id: UserId) -> UserRef {
        let rows = self.users.0.lock().unwrap();
        let (u, _) = rows.iter().find(|(u, _)| u.id == id).expect("user exists");
        UserRef { id: u.id, name: u.name.as_str().to_owned(), email: u.email.as_str().to_owned(), role: u.role }
    }

    fn summarize(&self, t: &Ticket) -> TicketSummary {
        TicketSummary {
            id: t.id,
            number: t.number,
            subject: t.subject.as_str().to_owned(),
            status: t.status,
            priority: t.priority,
            channel: t.channel,
            requester: self.user_ref(t.requester_id),
            assignee: t.assignee_id.map(|id| self.user_ref(id)),
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }

    fn save(state: &mut TicketState, ticket: &Ticket) {
        let slot = state.tickets.iter_mut().find(|t| t.id == ticket.id).expect("ticket exists");
        *slot = ticket.clone();
    }
}

#[async_trait]
impl TicketRepository for InMemoryTickets {
    async fn insert(&self, new: NewTicket) -> AppResult<Ticket> {
        let mut state = self.state.lock().unwrap();
        let ticket = Ticket {
            id: TicketId(Uuid::new_v4()),
            number: TicketNumber(1000 + state.tickets.len() as i64 + 1),
            subject: new.subject,
            description: new.description,
            status: Status::Open,
            priority: new.priority,
            channel: new.channel,
            requester_id: new.requester_id,
            assignee_id: new.assignee_id,
            created_at: new.created_at,
            updated_at: new.created_at,
            resolved_at: None,
        };
        state.tickets.push(ticket.clone());
        Ok(ticket)
    }

    async fn find_by_number(&self, number: TicketNumber) -> AppResult<Option<Ticket>> {
        Ok(self.state.lock().unwrap().tickets.iter().find(|t| t.number == number).cloned())
    }

    async fn update(&self, ticket: &Ticket, events: &[NewEvent]) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        Self::save(&mut state, ticket);
        for e in events {
            state.entries.push((ticket.id, Entry::Event(Uuid::new_v4(), e.clone())));
        }
        Ok(())
    }

    async fn add_comment(&self, ticket: &Ticket, comment: NewComment, events: &[NewEvent]) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        Self::save(&mut state, ticket);
        state.entries.push((ticket.id, Entry::Comment(Uuid::new_v4(), comment)));
        for e in events {
            state.entries.push((ticket.id, Entry::Event(Uuid::new_v4(), e.clone())));
        }
        Ok(())
    }

    async fn summary(&self, id: TicketId) -> AppResult<TicketSummary> {
        let ticket = self.state.lock().unwrap().tickets.iter().find(|t| t.id == id).cloned().ok_or(AppError::NotFound)?;
        Ok(self.summarize(&ticket))
    }

    async fn list(&self, q: &TicketQuery) -> AppResult<Page<TicketSummary>> {
        let tickets: Vec<Ticket> = {
            let mut state = self.state.lock().unwrap();
            state.last_query = Some(q.clone());
            state.tickets.clone()
        };
        let mut matching: Vec<Ticket> = tickets
            .into_iter()
            .filter(|t| q.statuses.is_empty() || q.statuses.contains(&t.status))
            .filter(|t| q.priority.is_none_or(|p| t.priority == p))
            .filter(|t| !q.unassigned_only || t.assignee_id.is_none())
            .filter(|t| q.assignee_id.is_none_or(|a| t.assignee_id == Some(a)))
            .filter(|t| q.requester_id.is_none_or(|r| t.requester_id == r))
            .filter(|t| q.search.as_ref().is_none_or(|s| t.subject.as_str().to_lowercase().contains(&s.to_lowercase())))
            .collect();
        match q.sort {
            TicketSort::Priority => matching.sort_by_key(|t| (std::cmp::Reverse(t.priority as u8), t.created_at)),
            TicketSort::Recent => matching.sort_by_key(|t| std::cmp::Reverse(t.updated_at)),
        }
        let total = matching.len() as i64;
        let items = matching.iter().skip(q.offset as usize).take(q.limit as usize).map(|t| self.summarize(t)).collect();
        Ok(Page { items, total, page: 1, per_page: q.limit as u32 })
    }

    async fn timeline(&self, id: TicketId, include_internal: bool) -> AppResult<Vec<TimelineItem>> {
        let state = self.state.lock().unwrap();
        let mut items = Vec::new();
        for (ticket_id, entry) in &state.entries {
            if *ticket_id != id {
                continue;
            }
            match entry {
                Entry::Comment(cid, c) if include_internal || !c.internal => items.push(TimelineItem::Comment {
                    id: *cid,
                    author: self.user_ref(c.author_id),
                    body: c.body.as_str().to_owned(),
                    internal: c.internal,
                    created_at: c.created_at,
                }),
                Entry::Comment(..) => {}
                Entry::Event(eid, e) => {
                    let name = |v: &Option<String>| match (e.kind, v) {
                        (EventKind::AssigneeChanged, Some(id)) => Some(self.user_ref(UserId(id.parse().unwrap())).name),
                        _ => v.clone(),
                    };
                    items.push(TimelineItem::Event {
                        id: *eid,
                        actor: e.actor_id.map(|a| self.user_ref(a)),
                        kind: e.kind,
                        old_value: name(&e.old_value),
                        new_value: name(&e.new_value),
                        created_at: e.created_at,
                    });
                }
            }
        }
        Ok(items)
    }

    async fn count_for_requester(&self, requester_id: UserId) -> AppResult<i64> {
        Ok(self.state.lock().unwrap().tickets.iter().filter(|t| t.requester_id == requester_id).count() as i64)
    }

    async fn view_counts(&self, me: UserId) -> AppResult<ViewCounts> {
        let state = self.state.lock().unwrap();
        let active = |t: &&Ticket| t.status.is_active();
        Ok(ViewCounts {
            mine: state.tickets.iter().filter(active).filter(|t| t.assignee_id == Some(me)).count() as i64,
            unassigned: state.tickets.iter().filter(active).filter(|t| t.assignee_id.is_none()).count() as i64,
            open: state.tickets.iter().filter(active).count() as i64,
            solved: state.tickets.iter().filter(|t| !t.status.is_active()).count() as i64,
        })
    }
}
