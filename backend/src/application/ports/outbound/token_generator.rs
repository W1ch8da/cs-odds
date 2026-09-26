/// Random, unguessable tokens (refresh tokens) and the hash we store for them.
pub trait OpaqueTokenGenerator: Send + Sync {
    fn generate(&self) -> String;
    fn hash(&self, token: &str) -> String;
}
