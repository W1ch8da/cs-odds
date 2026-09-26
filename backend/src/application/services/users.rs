use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::{
        error::{AppError, AppResult},
        ports::{
            inbound::{CreateStaffInput, UserAdminUseCases},
            outbound::{PasswordHasher, UserRepository},
        },
        principal::Principal,
    },
    domain::user::{Email, NewPassword, NewUser, PersonName, Role, User},
};

pub struct UserAdminService {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl UserAdminService {
    pub fn new(users: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { users, hasher }
    }

    async fn create(&self, email: &str, name: &str, password: &str, role: Role) -> AppResult<User> {
        let email = Email::parse(email)?;
        let name = PersonName::parse(name)?;
        let password = NewPassword::parse(password)?;
        let password_hash = Some(self.hasher.hash(password.expose()).await?);
        self.users.create(NewUser { email, name, role, password_hash }).await
    }
}

#[async_trait]
impl UserAdminUseCases for UserAdminService {
    async fn create_staff(&self, actor: &Principal, input: CreateStaffInput) -> AppResult<User> {
        actor.require_admin()?;
        if !input.role.is_staff() {
            return Err(AppError::Validation(
                "Team members must be agents or admins. Customers sign up from the portal.".into(),
            ));
        }
        self.create(&input.email, &input.name, &input.password, input.role).await
    }

    async fn list_users(&self, actor: &Principal, role: Option<Role>) -> AppResult<Vec<User>> {
        actor.require_admin()?;
        self.users.list(role).await
    }

    async fn list_assignable(&self, actor: &Principal) -> AppResult<Vec<User>> {
        actor.require_staff()?;
        let mut staff = self.users.list(None).await?;
        staff.retain(|u| u.role.is_staff());
        staff.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        Ok(staff)
    }

    async fn bootstrap_admin(&self, email: &str, name: &str, password: &str) -> AppResult<bool> {
        if self.users.any_with_role(Role::Admin).await? {
            return Ok(false);
        }
        self.create(email, name, password, Role::Admin).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::test_support::*;

    fn service(env: &TestEnv) -> UserAdminService {
        UserAdminService::new(env.users.clone(), Arc::new(FakeHasher))
    }

    fn staff(role: Role) -> CreateStaffInput {
        CreateStaffInput { email: "maya@cs-odds.example".into(), name: "Maya Chen".into(), password: "correct horse".into(), role }
    }

    #[tokio::test]
    async fn only_admins_create_staff() {
        let env = TestEnv::default();
        let svc = service(&env);
        let agent = env.seed(Role::Agent).await;
        assert!(matches!(svc.create_staff(&agent, staff(Role::Agent)).await, Err(AppError::Forbidden)));

        let admin = env.seed(Role::Admin).await;
        let created = svc.create_staff(&admin, staff(Role::Agent)).await.unwrap();
        assert_eq!(created.role, Role::Agent);
    }

    #[tokio::test]
    async fn staff_accounts_cannot_be_customers() {
        let env = TestEnv::default();
        let admin = env.seed(Role::Admin).await;
        assert!(matches!(service(&env).create_staff(&admin, staff(Role::Customer)).await, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn listing_permissions() {
        let env = TestEnv::default();
        let svc = service(&env);
        let customer = env.seed(Role::Customer).await;
        let agent = env.seed(Role::Agent).await;
        let admin = env.seed(Role::Admin).await;

        assert!(matches!(svc.list_users(&agent, None).await, Err(AppError::Forbidden)));
        assert_eq!(svc.list_users(&admin, Some(Role::Customer)).await.unwrap().len(), 1);

        assert!(matches!(svc.list_assignable(&customer).await, Err(AppError::Forbidden)));
        let assignable = svc.list_assignable(&agent).await.unwrap();
        assert_eq!(assignable.len(), 2);
        assert!(assignable.iter().all(|u| u.role.is_staff()));
    }

    #[tokio::test]
    async fn bootstrap_admin_runs_once() {
        let env = TestEnv::default();
        let svc = service(&env);
        assert!(svc.bootstrap_admin("admin@cs-odds.example", "Admin", "correct horse").await.unwrap());
        assert!(!svc.bootstrap_admin("other@cs-odds.example", "Other", "correct horse").await.unwrap());
    }
}
