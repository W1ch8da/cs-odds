use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub Uuid);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    Agent,
    Customer,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Agent => "agent",
            Self::Customer => "customer",
        }
    }

    /// Agents and admins work tickets; customers only see their own.
    pub fn is_staff(self) -> bool {
        matches!(self, Self::Admin | Self::Agent)
    }
}

impl FromStr for Role {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Self::Admin),
            "agent" => Ok(Self::Agent),
            "customer" => Ok(Self::Customer),
            other => Err(DomainError::InvalidRole(format!(
                "Unknown role \"{other}\". Use admin, agent or customer."
            ))),
        }
    }
}

/// A syntactically valid, lower-cased email address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

impl Email {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let email = raw.trim().to_lowercase();
        let valid = email.len() <= 254
            && !email.chars().any(char::is_whitespace)
            && email.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty()
                    && !domain.contains('@')
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
            });
        if valid { Ok(Self(email)) } else { Err(DomainError::InvalidEmail) }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A person's display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonName(String);

impl PersonName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let name = raw.trim();
        if name.is_empty() {
            return Err(DomainError::InvalidName("Enter your name."));
        }
        if name.chars().count() > 100 {
            return Err(DomainError::InvalidName("Keep the name under 100 characters."));
        }
        Ok(Self(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A password that meets the policy. Never stored; only its hash is.
pub struct NewPassword(String);

impl NewPassword {
    pub const MIN_LEN: usize = 10;
    const MAX_LEN: usize = 128;

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let len = raw.chars().count();
        if len < Self::MIN_LEN {
            return Err(DomainError::WeakPassword("Use at least 10 characters for your password."));
        }
        if len > Self::MAX_LEN {
            return Err(DomainError::WeakPassword("Use at most 128 characters for your password."));
        }
        if raw.trim().is_empty() {
            return Err(DomainError::WeakPassword("A password can't be only spaces."));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

// Keep passwords out of logs.
impl fmt::Debug for NewPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NewPassword(***)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub name: PersonName,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

/// Data needed to create a user. `password_hash` is `None` for customers
/// created from inbound email who haven't set a password yet.
#[derive(Debug, Clone)]
pub struct NewUser {
    pub email: Email,
    pub name: PersonName,
    pub role: Role,
    pub password_hash: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_is_normalised() {
        assert_eq!(Email::parse("  Jordan.Blake@Northwind.COM ").unwrap().as_str(), "jordan.blake@northwind.com");
    }

    #[test]
    fn rejects_bad_emails() {
        for bad in ["", "jordan", "jordan@", "@northwind.com", "a@b", "a@@b.com", "a b@c.com", "a@.com", "a@com."] {
            assert_eq!(Email::parse(bad), Err(DomainError::InvalidEmail), "{bad:?}");
        }
    }

    #[test]
    fn password_policy() {
        assert!(NewPassword::parse("short").is_err());
        assert!(NewPassword::parse("          ").is_err());
        assert!(NewPassword::parse(&"x".repeat(129)).is_err());
        assert!(NewPassword::parse("correct horse").is_ok());
    }

    #[test]
    fn name_is_trimmed_and_required() {
        assert_eq!(PersonName::parse("  Maya Chen ").unwrap().as_str(), "Maya Chen");
        assert!(PersonName::parse("   ").is_err());
    }

    #[test]
    fn role_round_trips() {
        for role in [Role::Admin, Role::Agent, Role::Customer] {
            assert_eq!(role.as_str().parse::<Role>().unwrap(), role);
        }
        assert!("owner".parse::<Role>().is_err());
        assert!(Role::Agent.is_staff() && !Role::Customer.is_staff());
    }
}
