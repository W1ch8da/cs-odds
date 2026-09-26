use sha2::{Digest, Sha256};

use crate::application::ports::outbound::OpaqueTokenGenerator;

/// 256-bit random tokens from the OS-seeded CSPRNG, hex encoded. Stored as
/// SHA-256 hashes: the tokens are high-entropy, so a slow hash isn't needed.
pub struct RandomTokens;

impl OpaqueTokenGenerator for RandomTokens {
    fn generate(&self) -> String {
        let mut bytes = [0u8; 32];
        rand::fill(&mut bytes);
        hex::encode(bytes)
    }

    fn hash(&self, token: &str) -> String {
        hex::encode(Sha256::digest(token.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_hash_is_stable() {
        let (a, b) = (RandomTokens.generate(), RandomTokens.generate());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(RandomTokens.hash(&a), RandomTokens.hash(&a));
        assert_ne!(RandomTokens.hash(&a), a);
    }
}
