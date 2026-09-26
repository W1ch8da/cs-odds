use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::{error::AppResult, ports::outbound::DatabaseProbe};

pub struct PgDatabaseProbe {
    pool: PgPool,
}

impl PgDatabaseProbe {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DatabaseProbe for PgDatabaseProbe {
    async fn ping(&self) -> AppResult<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
