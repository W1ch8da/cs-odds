use std::sync::Arc;

use async_trait::async_trait;

use crate::application::{
    error::AppResult,
    ports::{
        inbound::{CheckHealth, HealthReport},
        outbound::DatabaseProbe,
    },
};

pub struct HealthService {
    db: Arc<dyn DatabaseProbe>,
}

impl HealthService {
    pub fn new(db: Arc<dyn DatabaseProbe>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CheckHealth for HealthService {
    async fn check(&self) -> AppResult<HealthReport> {
        self.db.ping().await?;
        Ok(HealthReport {
            status: "ok",
            database: "ok",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::AppError;

    struct FakeProbe {
        healthy: bool,
    }

    #[async_trait]
    impl DatabaseProbe for FakeProbe {
        async fn ping(&self) -> AppResult<()> {
            if self.healthy {
                Ok(())
            } else {
                Err(AppError::Unexpected(anyhow::anyhow!("db down")))
            }
        }
    }

    #[tokio::test]
    async fn reports_ok_when_database_reachable() {
        let service = HealthService::new(Arc::new(FakeProbe { healthy: true }));
        let report = service.check().await.unwrap();
        assert_eq!(report.database, "ok");
    }

    #[tokio::test]
    async fn fails_when_database_unreachable() {
        let service = HealthService::new(Arc::new(FakeProbe { healthy: false }));
        assert!(matches!(service.check().await, Err(AppError::Unexpected(_))));
    }
}
