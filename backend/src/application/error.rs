use crate::domain::error::DomainError;

/// Errors returned by use cases. Adapters translate these at the boundary:
/// the HTTP adapter maps them to status codes, and outbound adapters map
/// their own errors (e.g. `sqlx::Error`) into them.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    /// Deliberately vague so it doesn't reveal which accounts exist.
    #[error("That email and password don't match. Check them and try again.")]
    InvalidCredentials,
    #[error("Your session has ended. Sign in again.")]
    Unauthorized,
    #[error("You don't have permission to do that.")]
    Forbidden,
    #[error("We couldn't find that.")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl From<DomainError> for AppError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::TicketClosed => Self::Conflict(err.to_string()),
            other => Self::Validation(other.to_string()),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
