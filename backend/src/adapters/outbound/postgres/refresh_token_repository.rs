use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        error::AppResult,
        ports::outbound::{RefreshTokenRecord, RefreshTokenRepository},
    },
    domain::user::UserId,
};

pub struct PgRefreshTokenRepository {
    pool: PgPool,
}

impl PgRefreshTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefreshTokenRepository for PgRefreshTokenRepository {
    async fn insert(&self, user_id: UserId, token_hash: &str, expires_at: DateTime<Utc>) -> AppResult<()> {
        sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
            user_id.0,
            token_hash,
            expires_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_hash(&self, token_hash: &str) -> AppResult<Option<RefreshTokenRecord>> {
        let row = sqlx::query!(
            "SELECT id, user_id, expires_at, revoked_at FROM refresh_tokens WHERE token_hash = $1",
            token_hash,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| RefreshTokenRecord {
            id: r.id,
            user_id: UserId(r.user_id),
            expires_at: r.expires_at,
            revoked_at: r.revoked_at,
        }))
    }

    async fn revoke(&self, id: Uuid, at: DateTime<Utc>) -> AppResult<bool> {
        // The `revoked_at IS NULL` guard makes this the atomic "use once" check.
        let result = sqlx::query!(
            "UPDATE refresh_tokens SET revoked_at = $2 WHERE id = $1 AND revoked_at IS NULL",
            id,
            at,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    async fn revoke_all_for_user(&self, user_id: UserId, at: DateTime<Utc>) -> AppResult<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET revoked_at = $2 WHERE user_id = $1 AND revoked_at IS NULL",
            user_id.0,
            at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
