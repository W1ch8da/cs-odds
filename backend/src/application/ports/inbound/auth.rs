use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    application::{error::AppResult, principal::Principal},
    domain::user::User,
};

/// Raw input; validated into domain types by the service.
#[derive(Debug, Clone)]
pub struct RegisterCustomerInput {
    pub email: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

/// A signed-in session: a short-lived access token and a long-lived,
/// single-use refresh token.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub user: User,
    pub access_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_token: String,
    pub refresh_expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait AuthUseCases: Send + Sync {
    /// Self-service sign-up. Always creates a customer.
    async fn register_customer(&self, input: RegisterCustomerInput) -> AppResult<AuthSession>;
    async fn login(&self, input: LoginInput) -> AppResult<AuthSession>;
    /// Exchanges a refresh token for a new session and revokes the old token.
    /// Presenting an already-used token revokes every session of that user.
    async fn refresh(&self, refresh_token: &str) -> AppResult<AuthSession>;
    async fn logout(&self, refresh_token: &str) -> AppResult<()>;
    fn authenticate(&self, access_token: &str) -> AppResult<Principal>;
    async fn current_user(&self, principal: &Principal) -> AppResult<User>;
}
