/// A business rule was violated. Messages are written for end users.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("Enter a valid email address, like name@company.com.")]
    InvalidEmail,
    #[error("{0}")]
    InvalidName(&'static str),
    #[error("{0}")]
    WeakPassword(&'static str),
    #[error("{0}")]
    InvalidRole(String),
    #[error("{0}")]
    InvalidValue(String),
    #[error("{0}")]
    InvalidText(&'static str),
    #[error("{0}")]
    InvalidFile(&'static str),
    #[error("This ticket is closed, so it can't be changed. Open a new ticket instead.")]
    TicketClosed,
}
