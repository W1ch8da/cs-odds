use anyhow::anyhow;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher as _, PasswordVerifier as _, phc::PasswordHash},
};
use async_trait::async_trait;

use crate::application::{error::AppResult, ports::outbound::PasswordHasher};

/// Argon2id with the crate's recommended defaults. Runs on the blocking
/// thread pool because hashing takes tens of milliseconds by design.
pub struct Argon2Hasher;

#[async_trait]
impl PasswordHasher for Argon2Hasher {
    async fn hash(&self, password: &str) -> AppResult<String> {
        let password = password.to_owned();
        let hash = tokio::task::spawn_blocking(move || {
            Argon2::default()
                .hash_password(password.as_bytes())
                .map(|h| h.to_string())
                .map_err(|e| anyhow!("argon2 hash failed: {e}"))
        })
        .await
        .map_err(|e| anyhow!(e))??;
        Ok(hash)
    }

    async fn verify(&self, password: &str, hash: &str) -> AppResult<bool> {
        let (password, hash) = (password.to_owned(), hash.to_owned());
        let ok = tokio::task::spawn_blocking(move || {
            let parsed = PasswordHash::new(&hash).map_err(|e| anyhow!("stored password hash is invalid: {e}"))?;
            Ok::<_, anyhow::Error>(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        })
        .await
        .map_err(|e| anyhow!(e))??;
        Ok(ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hashes_and_verifies() {
        let hash = Argon2Hasher.hash("correct horse").await.unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(Argon2Hasher.verify("correct horse", &hash).await.unwrap());
        assert!(!Argon2Hasher.verify("wrong horse", &hash).await.unwrap());
    }
}
