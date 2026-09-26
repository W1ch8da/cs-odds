use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;

use crate::{
    application::{
        error::{AppError, AppResult},
        ports::{
            inbound::{AuthSession, AuthUseCases, LoginInput, RegisterCustomerInput},
            outbound::{
                AccessTokenCodec, Clock, OpaqueTokenGenerator, PasswordHasher, RefreshTokenRepository,
                UserRepository,
            },
        },
        principal::Principal,
    },
    domain::user::{Email, NewPassword, NewUser, PersonName, Role, User},
};

pub struct AuthService {
    users: Arc<dyn UserRepository>,
    refresh_tokens: Arc<dyn RefreshTokenRepository>,
    hasher: Arc<dyn PasswordHasher>,
    access_tokens: Arc<dyn AccessTokenCodec>,
    token_generator: Arc<dyn OpaqueTokenGenerator>,
    clock: Arc<dyn Clock>,
    refresh_ttl: Duration,
}

impl AuthService {
    pub fn new(
        users: Arc<dyn UserRepository>,
        refresh_tokens: Arc<dyn RefreshTokenRepository>,
        hasher: Arc<dyn PasswordHasher>,
        access_tokens: Arc<dyn AccessTokenCodec>,
        token_generator: Arc<dyn OpaqueTokenGenerator>,
        clock: Arc<dyn Clock>,
        refresh_ttl: Duration,
    ) -> Self {
        Self { users, refresh_tokens, hasher, access_tokens, token_generator, clock, refresh_ttl }
    }

    async fn start_session(&self, user: User) -> AppResult<AuthSession> {
        let now = self.clock.now();
        let access = self.access_tokens.issue(&user, now)?;
        let refresh_token = self.token_generator.generate();
        let refresh_expires_at = now + self.refresh_ttl;
        self.refresh_tokens
            .insert(user.id, &self.token_generator.hash(&refresh_token), refresh_expires_at)
            .await?;
        Ok(AuthSession {
            user,
            access_token: access.token,
            access_expires_at: access.expires_at,
            refresh_token,
            refresh_expires_at,
        })
    }
}

#[async_trait]
impl AuthUseCases for AuthService {
    async fn register_customer(&self, input: RegisterCustomerInput) -> AppResult<AuthSession> {
        let email = Email::parse(&input.email)?;
        let name = PersonName::parse(&input.name)?;
        let password = NewPassword::parse(&input.password)?;
        let password_hash = self.hasher.hash(password.expose()).await?;
        let user = self
            .users
            .create(NewUser { email, name, role: Role::Customer, password_hash: Some(password_hash) })
            .await?;
        self.start_session(user).await
    }

    async fn login(&self, input: LoginInput) -> AppResult<AuthSession> {
        // Every failure looks the same so the response doesn't reveal which emails have accounts.
        let email = Email::parse(&input.email).map_err(|_| AppError::InvalidCredentials)?;
        let Some((user, Some(hash))) = self.users.find_credentials(&email).await? else {
            return Err(AppError::InvalidCredentials);
        };
        if !self.hasher.verify(&input.password, &hash).await? {
            return Err(AppError::InvalidCredentials);
        }
        self.start_session(user).await
    }

    async fn refresh(&self, refresh_token: &str) -> AppResult<AuthSession> {
        let now = self.clock.now();
        let hash = self.token_generator.hash(refresh_token);
        let record = self.refresh_tokens.find_by_hash(&hash).await?.ok_or(AppError::Unauthorized)?;

        // A used token coming back means it was copied: end every session for that user.
        if record.revoked_at.is_some() || !self.refresh_tokens.revoke(record.id, now).await? {
            tracing::warn!(user_id = %record.user_id, "refresh token reuse detected; revoking all sessions");
            self.refresh_tokens.revoke_all_for_user(record.user_id, now).await?;
            return Err(AppError::Unauthorized);
        }
        if record.expires_at <= now {
            return Err(AppError::Unauthorized);
        }

        let user = self.users.find_by_id(record.user_id).await?.ok_or(AppError::Unauthorized)?;
        self.start_session(user).await
    }

    async fn logout(&self, refresh_token: &str) -> AppResult<()> {
        let hash = self.token_generator.hash(refresh_token);
        if let Some(record) = self.refresh_tokens.find_by_hash(&hash).await? {
            self.refresh_tokens.revoke(record.id, self.clock.now()).await?;
        }
        Ok(())
    }

    fn authenticate(&self, access_token: &str) -> AppResult<Principal> {
        self.access_tokens.verify(access_token)
    }

    async fn current_user(&self, principal: &Principal) -> AppResult<User> {
        self.users.find_by_id(principal.user_id).await?.ok_or(AppError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::test_support::*;

    fn service(env: &TestEnv) -> AuthService {
        AuthService::new(
            env.users.clone(),
            env.refresh_tokens.clone(),
            Arc::new(FakeHasher),
            Arc::new(FakeCodec),
            Arc::new(SeqTokens::default()),
            env.clock.clone(),
            Duration::days(30),
        )
    }

    fn register_input(email: &str) -> RegisterCustomerInput {
        RegisterCustomerInput { email: email.into(), name: "Jordan Blake".into(), password: "correct horse".into() }
    }

    #[tokio::test]
    async fn register_creates_customer_and_session() {
        let env = TestEnv::default();
        let session = service(&env).register_customer(register_input("Jordan@Northwind.com")).await.unwrap();
        assert_eq!(session.user.role, Role::Customer);
        assert_eq!(session.user.email.as_str(), "jordan@northwind.com");
        assert_eq!(session.refresh_expires_at, env.clock.now() + Duration::days(30));
    }

    #[tokio::test]
    async fn register_rejects_duplicate_email_and_weak_password() {
        let env = TestEnv::default();
        let svc = service(&env);
        svc.register_customer(register_input("jordan@northwind.com")).await.unwrap();
        assert!(matches!(svc.register_customer(register_input("JORDAN@northwind.com")).await, Err(AppError::Conflict(_))));

        let weak = RegisterCustomerInput { password: "short".into(), ..register_input("other@northwind.com") };
        assert!(matches!(svc.register_customer(weak).await, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn login_failures_are_indistinguishable() {
        let env = TestEnv::default();
        let svc = service(&env);
        svc.register_customer(register_input("jordan@northwind.com")).await.unwrap();

        let cases = [("jordan@northwind.com", "wrong password"), ("nobody@northwind.com", "correct horse"), ("not-an-email", "x")];
        for (email, password) in cases {
            let res = svc.login(LoginInput { email: email.into(), password: password.into() }).await;
            assert!(matches!(res, Err(AppError::InvalidCredentials)), "{email}");
        }
        let ok = svc.login(LoginInput { email: " Jordan@northwind.com".into(), password: "correct horse".into() }).await;
        assert!(ok.is_ok());
    }

    #[tokio::test]
    async fn refresh_rotates_token() {
        let env = TestEnv::default();
        let svc = service(&env);
        let first = svc.register_customer(register_input("jordan@northwind.com")).await.unwrap();
        let second = svc.refresh(&first.refresh_token).await.unwrap();
        assert_ne!(first.refresh_token, second.refresh_token);
        assert!(svc.refresh(&second.refresh_token).await.is_ok());
    }

    #[tokio::test]
    async fn reusing_a_refresh_token_revokes_all_sessions() {
        let env = TestEnv::default();
        let svc = service(&env);
        let first = svc.register_customer(register_input("jordan@northwind.com")).await.unwrap();
        let second = svc.refresh(&first.refresh_token).await.unwrap();

        // An attacker replays the old token...
        assert!(matches!(svc.refresh(&first.refresh_token).await, Err(AppError::Unauthorized)));
        // ...and the legitimate newer session is ended too.
        assert!(matches!(svc.refresh(&second.refresh_token).await, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn expired_refresh_token_is_rejected() {
        let env = TestEnv::default();
        let svc = service(&env);
        let session = svc.register_customer(register_input("jordan@northwind.com")).await.unwrap();
        env.clock.advance(Duration::days(31));
        assert!(matches!(svc.refresh(&session.refresh_token).await, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn logout_revokes_refresh_token() {
        let env = TestEnv::default();
        let svc = service(&env);
        let session = svc.register_customer(register_input("jordan@northwind.com")).await.unwrap();
        svc.logout(&session.refresh_token).await.unwrap();
        assert!(svc.refresh(&session.refresh_token).await.is_err());
        // Unknown tokens are ignored.
        svc.logout("never-issued").await.unwrap();
    }

    #[tokio::test]
    async fn authenticate_round_trips_access_token() {
        let env = TestEnv::default();
        let svc = service(&env);
        let session = svc.register_customer(register_input("jordan@northwind.com")).await.unwrap();
        let principal = svc.authenticate(&session.access_token).unwrap();
        assert_eq!(principal.user_id, session.user.id);
        assert_eq!(svc.current_user(&principal).await.unwrap().id, session.user.id);
        assert!(matches!(svc.authenticate("garbage"), Err(AppError::Unauthorized)));
    }
}
