use chrono::{DateTime, Utc};

use crate::{application::error::AppResult, application::principal::Principal, domain::user::User};

#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

/// Short-lived, self-contained access tokens (e.g. JWT).
pub trait AccessTokenCodec: Send + Sync {
    fn issue(&self, user: &User, now: DateTime<Utc>) -> AppResult<IssuedToken>;
    /// Fails with `AppError::Unauthorized` for invalid or expired tokens.
    fn verify(&self, token: &str) -> AppResult<Principal>;
}
