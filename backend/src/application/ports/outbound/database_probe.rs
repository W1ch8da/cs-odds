use async_trait::async_trait;

use crate::application::error::AppResult;

/// Checks that the primary data store can be reached.
#[async_trait]
pub trait DatabaseProbe: Send + Sync {
    async fn ping(&self) -> AppResult<()>;
}
