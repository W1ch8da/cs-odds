use crate::{
    application::error::{AppError, AppResult},
    domain::user::{Role, UserId},
};

/// The authenticated caller of a use case. Authorization decisions are made
/// in services against this, never in adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Principal {
    pub user_id: UserId,
    pub role: Role,
}

impl Principal {
    pub fn require_admin(&self) -> AppResult<()> {
        if self.role == Role::Admin { Ok(()) } else { Err(AppError::Forbidden) }
    }

    pub fn require_staff(&self) -> AppResult<()> {
        if self.role.is_staff() { Ok(()) } else { Err(AppError::Forbidden) }
    }
}
