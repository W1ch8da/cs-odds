use async_trait::async_trait;

use crate::application::error::AppResult;

/// Hashing is deliberately slow, so implementations should run it off the
/// async executor.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> AppResult<String>;
    async fn verify(&self, password: &str, hash: &str) -> AppResult<bool>;
}
