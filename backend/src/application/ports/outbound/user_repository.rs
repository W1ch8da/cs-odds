use async_trait::async_trait;

use crate::{
    application::error::AppResult,
    domain::user::{Email, NewUser, Role, User, UserId},
};

#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Fails with `AppError::Conflict` if the email is already registered.
    async fn create(&self, user: NewUser) -> AppResult<User>;
    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>>;
    async fn find_by_email(&self, email: &Email) -> AppResult<Option<User>>;
    /// Returns the user with their password hash (if they have one).
    async fn find_credentials(&self, email: &Email) -> AppResult<Option<(User, Option<String>)>>;
    /// Newest first, optionally filtered by role.
    async fn list(&self, role: Option<Role>) -> AppResult<Vec<User>>;
    async fn any_with_role(&self, role: Role) -> AppResult<bool>;
}
