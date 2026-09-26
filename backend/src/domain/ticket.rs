use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{error::DomainError, user::UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TicketId(pub Uuid);

/// Human-facing sequential number, shown as `TKT-<n>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TicketNumber(pub i64);

impl fmt::Display for TicketNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TKT-{}", self.0)
    }
}

macro_rules! string_enum {
    ($name:ident, $what:literal, { $($variant:ident => $s:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $s),+ }
            }
        }

        impl FromStr for $name {
            type Err = DomainError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($s => Ok(Self::$variant),)+
                    other => Err(DomainError::InvalidValue(format!(
                        concat!("Unknown ", $what, " \"{}\". Use one of: {}."),
                        other,
                        [$($s),+].join(", ")
                    ))),
                }
            }
        }
    };
}

string_enum!(Status, "status", {
    Open => "open",
    Pending => "pending",
    OnHold => "on_hold",
    Solved => "solved",
    Closed => "closed",
});

string_enum!(Priority, "priority", {
    Low => "low",
    Normal => "normal",
    High => "high",
    Urgent => "urgent",
});

string_enum!(Channel, "channel", {
    Portal => "portal",
    Email => "email",
    Agent => "agent",
});

impl Status {
    /// Open, pending or on hold: someone still has work to do.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Open | Self::Pending | Self::OnHold)
    }

    pub const ACTIVE: &'static [Self] = &[Self::Open, Self::Pending, Self::OnHold];
    pub const DONE: &'static [Self] = &[Self::Solved, Self::Closed];
}

/// A ticket subject: one line, 1–200 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject(String);

impl Subject {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let s = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        if s.is_empty() {
            return Err(DomainError::InvalidText("Add a subject so the team knows what this is about."));
        }
        if s.chars().count() > 200 {
            return Err(DomainError::InvalidText("Keep the subject under 200 characters."));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The text of a ticket description, reply or note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageBody(String);

impl MessageBody {
    pub const MAX_CHARS: usize = 20_000;

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let s = raw.trim();
        if s.is_empty() {
            return Err(DomainError::InvalidText("Write a message before sending."));
        }
        if s.chars().count() > Self::MAX_CHARS {
            return Err(DomainError::InvalidText("Keep the message under 20,000 characters."));
        }
        Ok(Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    pub id: TicketId,
    pub number: TicketNumber,
    pub subject: Subject,
    pub description: MessageBody,
    pub status: Status,
    pub priority: Priority,
    pub channel: Channel,
    pub requester_id: UserId,
    pub assignee_id: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// A change worth recording in the ticket's history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketChange {
    Status { from: Status, to: Status },
    Priority { from: Priority, to: Priority },
    Assignee { from: Option<UserId>, to: Option<UserId> },
}

impl Ticket {
    /// Closed tickets are archived: no more changes or replies.
    pub fn ensure_not_closed(&self) -> Result<(), DomainError> {
        if self.status == Status::Closed { Err(DomainError::TicketClosed) } else { Ok(()) }
    }

    pub fn set_status(&mut self, to: Status, now: DateTime<Utc>) -> Result<Option<TicketChange>, DomainError> {
        self.ensure_not_closed()?;
        let from = self.status;
        if from == to {
            return Ok(None);
        }
        self.status = to;
        if to == Status::Solved {
            self.resolved_at = Some(now);
        } else if to.is_active() {
            self.resolved_at = None;
        }
        self.updated_at = now;
        Ok(Some(TicketChange::Status { from, to }))
    }

    pub fn set_priority(&mut self, to: Priority, now: DateTime<Utc>) -> Result<Option<TicketChange>, DomainError> {
        self.ensure_not_closed()?;
        let from = self.priority;
        if from == to {
            return Ok(None);
        }
        self.priority = to;
        self.updated_at = now;
        Ok(Some(TicketChange::Priority { from, to }))
    }

    pub fn assign(&mut self, to: Option<UserId>, now: DateTime<Utc>) -> Result<Option<TicketChange>, DomainError> {
        self.ensure_not_closed()?;
        let from = self.assignee_id;
        if from == to {
            return Ok(None);
        }
        self.assignee_id = to;
        self.updated_at = now;
        Ok(Some(TicketChange::Assignee { from, to }))
    }

    /// A customer wrote back: whatever the state, the team needs to look again.
    pub fn on_customer_reply(&mut self, now: DateTime<Utc>) -> Result<Option<TicketChange>, DomainError> {
        self.ensure_not_closed()?;
        self.updated_at = now;
        self.set_status(Status::Open, now)
    }

    pub fn touch(&mut self, now: DateTime<Utc>) {
        self.updated_at = now;
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn ticket(status: Status) -> Ticket {
        let t0 = Utc.with_ymd_and_hms(2026, 9, 26, 9, 0, 0).unwrap();
        Ticket {
            id: TicketId(Uuid::new_v4()),
            number: TicketNumber(1042),
            subject: Subject::parse("Password reset link expired").unwrap(),
            description: MessageBody::parse("Every link says expired.").unwrap(),
            status,
            priority: Priority::Normal,
            channel: Channel::Portal,
            requester_id: UserId(Uuid::new_v4()),
            assignee_id: None,
            created_at: t0,
            updated_at: t0,
            resolved_at: None,
        }
    }

    #[test]
    fn number_display() {
        assert_eq!(TicketNumber(1042).to_string(), "TKT-1042");
    }

    #[test]
    fn enums_round_trip_and_reject_unknown() {
        for s in Status::ALL {
            assert_eq!(s.as_str().parse::<Status>().unwrap(), *s);
        }
        let err = "done".parse::<Status>().unwrap_err().to_string();
        assert!(err.contains("open, pending, on_hold, solved, closed"), "{err}");
        assert_eq!("urgent".parse::<Priority>().unwrap(), Priority::Urgent);
    }

    #[test]
    fn subject_and_body_validation() {
        assert_eq!(Subject::parse("  Can't   log in \n").unwrap().as_str(), "Can't log in");
        assert!(Subject::parse("   ").is_err());
        assert!(Subject::parse(&"x".repeat(201)).is_err());
        assert!(MessageBody::parse(" \n ").is_err());
        assert_eq!(MessageBody::parse("  line one\nline two  ").unwrap().as_str(), "line one\nline two");
    }

    #[test]
    fn solving_sets_and_reopening_clears_resolved_at() {
        let now = Utc::now();
        let mut t = ticket(Status::Open);
        assert_eq!(t.set_status(Status::Solved, now).unwrap(), Some(TicketChange::Status { from: Status::Open, to: Status::Solved }));
        assert_eq!(t.resolved_at, Some(now));
        t.on_customer_reply(now).unwrap();
        assert_eq!(t.status, Status::Open);
        assert_eq!(t.resolved_at, None);
    }

    #[test]
    fn unchanged_values_produce_no_change() {
        let mut t = ticket(Status::Open);
        assert_eq!(t.set_status(Status::Open, Utc::now()).unwrap(), None);
        assert_eq!(t.set_priority(Priority::Normal, Utc::now()).unwrap(), None);
        assert_eq!(t.assign(None, Utc::now()).unwrap(), None);
    }

    #[test]
    fn customer_reply_reopens_pending_and_solved() {
        for from in [Status::Pending, Status::OnHold, Status::Solved] {
            let mut t = ticket(from);
            assert!(t.on_customer_reply(Utc::now()).unwrap().is_some());
            assert_eq!(t.status, Status::Open);
        }
        let mut open = ticket(Status::Open);
        assert_eq!(open.on_customer_reply(Utc::now()).unwrap(), None);
    }

    #[test]
    fn closed_tickets_are_final() {
        let mut t = ticket(Status::Closed);
        assert_eq!(t.set_status(Status::Open, Utc::now()), Err(DomainError::TicketClosed));
        assert_eq!(t.on_customer_reply(Utc::now()), Err(DomainError::TicketClosed));
        assert_eq!(t.assign(Some(UserId(Uuid::new_v4())), Utc::now()), Err(DomainError::TicketClosed));
    }
}
