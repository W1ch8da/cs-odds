use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{is_unique_violation, types::DbRole};
use crate::{
    application::{
        error::{AppError, AppResult},
        ports::outbound::UserRepository,
    },
    domain::user::{Email, NewUser, PersonName, Role, User, UserId},
};

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct UserRow {
    id: Uuid,
    email: String,
    name: String,
    role: DbRole,
    created_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = AppError;

    fn try_from(row: UserRow) -> AppResult<Self> {
        Ok(User {
            id: UserId(row.id),
            email: Email::parse(&row.email).context("invalid email stored in users table")?,
            name: PersonName::parse(&row.name).context("invalid name stored in users table")?,
            role: row.role.into(),
            created_at: row.created_at,
        })
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn create(&self, user: NewUser) -> AppResult<User> {
        let row = sqlx::query_as!(
            UserRow,
            r#"INSERT INTO users (email, name, role, password_hash)
               VALUES ($1, $2, $3, $4)
               RETURNING id, email, name, role AS "role: DbRole", created_at"#,
            user.email.as_str(),
            user.name.as_str(),
            DbRole::from(user.role) as DbRole,
            user.password_hash,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if is_unique_violation(&e) {
                AppError::Conflict("An account with this email already exists. Try signing in instead.".into())
            } else {
                e.into()
            }
        })?;
        row.try_into()
    }

    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>> {
        sqlx::query_as!(
            UserRow,
            r#"SELECT id, email, name, role AS "role: DbRole", created_at FROM users WHERE id = $1"#,
            id.0,
        )
        .fetch_optional(&self.pool)
        .await?
        .map(User::try_from)
        .transpose()
    }

    async fn find_by_email(&self, email: &Email) -> AppResult<Option<User>> {
        sqlx::query_as!(
            UserRow,
            r#"SELECT id, email, name, role AS "role: DbRole", created_at FROM users WHERE lower(email) = $1"#,
            email.as_str(),
        )
        .fetch_optional(&self.pool)
        .await?
        .map(User::try_from)
        .transpose()
    }

    async fn find_credentials(&self, email: &Email) -> AppResult<Option<(User, Option<String>)>> {
        let Some(row) = sqlx::query!(
            r#"SELECT id, email, name, role AS "role: DbRole", created_at, password_hash
               FROM users WHERE lower(email) = $1"#,
            email.as_str(),
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let user = UserRow { id: row.id, email: row.email, name: row.name, role: row.role, created_at: row.created_at };
        Ok(Some((user.try_into()?, row.password_hash)))
    }

    async fn list(&self, role: Option<Role>) -> AppResult<Vec<User>> {
        sqlx::query_as!(
            UserRow,
            r#"SELECT id, email, name, role AS "role: DbRole", created_at
               FROM users
               WHERE $1::user_role IS NULL OR role = $1
               ORDER BY created_at DESC"#,
            role.map(DbRole::from) as Option<DbRole>,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(User::try_from)
        .collect()
    }

    async fn any_with_role(&self, role: Role) -> AppResult<bool> {
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM users WHERE role = $1) AS "exists!""#,
            DbRole::from(role) as DbRole,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }
}
