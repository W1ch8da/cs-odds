use async_trait::async_trait;
use serde::Serialize;

use crate::application::error::AppResult;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HealthReport {
    pub status: &'static str,
    pub database: &'static str,
}

#[async_trait]
pub trait CheckHealth: Send + Sync {
    async fn check(&self) -> AppResult<HealthReport>;
}
