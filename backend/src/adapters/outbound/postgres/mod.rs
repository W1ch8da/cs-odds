//! sqlx/Postgres adapter. The only place that knows about `sqlx`.

mod health_probe;
mod refresh_token_repository;
mod ticket_repository;
mod types;
mod user_repository;

use anyhow::Context;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::application::error::AppError;

pub use health_probe::PgDatabaseProbe;
pub use refresh_token_repository::PgRefreshTokenRepository;
pub use ticket_repository::PgTicketRepository;
pub use user_repository::PgUserRepository;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .context("failed to connect to Postgres")?;
    MIGRATOR.run(&pool).await.context("failed to run migrations")?;
    Ok(pool)
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    err.as_database_error().is_some_and(|e| e.is_unique_violation())
}

/// Keeps `sqlx::Error` from leaking past the adapter boundary.
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            other => Self::Unexpected(other.into()),
        }
    }
}
