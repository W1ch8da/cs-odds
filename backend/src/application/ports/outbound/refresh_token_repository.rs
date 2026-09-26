use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{application::error::AppResult, domain::user::UserId};

#[derive(Debug, Clone)]
pub struct RefreshTokenRecord {
    pub id: Uuid,
    pub user_id: UserId,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Stores only hashes of refresh tokens, never the tokens themselves.
#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn insert(&self, user_id: UserId, token_hash: &str, expires_at: DateTime<Utc>) -> AppResult<()>;
    async fn find_by_hash(&self, token_hash: &str) -> AppResult<Option<RefreshTokenRecord>>;
    /// Revokes one token. Returns false if it was already revoked, so a
    /// concurrent second use of the same token is detected.
    async fn revoke(&self, id: Uuid, at: DateTime<Utc>) -> AppResult<bool>;
    async fn revoke_all_for_user(&self, user_id: UserId, at: DateTime<Utc>) -> AppResult<()>;
}
