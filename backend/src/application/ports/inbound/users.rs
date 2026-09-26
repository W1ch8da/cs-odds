use async_trait::async_trait;

use crate::{
    application::{error::AppResult, principal::Principal},
    domain::user::{Role, User},
};

#[derive(Debug, Clone)]
pub struct CreateStaffInput {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: Role,
}

#[async_trait]
pub trait UserAdminUseCases: Send + Sync {
    /// Admin only. Creates an agent or another admin.
    async fn create_staff(&self, actor: &Principal, input: CreateStaffInput) -> AppResult<User>;
    /// Admin only.
    async fn list_users(&self, actor: &Principal, role: Option<Role>) -> AppResult<Vec<User>>;
    /// Staff only. Agents and admins, for assignment pickers.
    async fn list_assignable(&self, actor: &Principal) -> AppResult<Vec<User>>;
    /// Creates the first admin at startup if there is none. Returns true if created.
    async fn bootstrap_admin(&self, email: &str, name: &str, password: &str) -> AppResult<bool>;
}
