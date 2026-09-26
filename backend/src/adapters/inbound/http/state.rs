use std::sync::Arc;

use crate::application::ports::inbound::{AttachmentUseCases, AuthUseCases, CheckHealth, TicketUseCases, UserAdminUseCases};

/// Handlers depend on inbound ports (traits), never on concrete services.
#[derive(Clone)]
pub struct HttpState {
    pub health: Arc<dyn CheckHealth>,
    pub auth: Arc<dyn AuthUseCases>,
    pub users: Arc<dyn UserAdminUseCases>,
    pub tickets: Arc<dyn TicketUseCases>,
    pub attachments: Arc<dyn AttachmentUseCases>,
    pub cookies: CookieSettings,
}

#[derive(Debug, Clone, Copy)]
pub struct CookieSettings {
    /// Send cookies over HTTPS only. Must be true in production.
    pub secure: bool,
}
