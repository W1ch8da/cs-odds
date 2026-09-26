//! Composition root: builds adapters, injects them into services, and hands
//! the services to the HTTP adapter. Shared by `main.rs` and integration tests.

use std::sync::Arc;

use anyhow::Context;
use axum::Router;
use chrono::Duration;
use sqlx::PgPool;

use crate::{
    adapters::{
        inbound::http::{self, CookieSettings, HttpState},
        outbound::{
            postgres::{
                PgAttachmentRepository, PgDatabaseProbe, PgEmailOutbox, PgRefreshTokenRepository, PgTicketRepository,
                PgUserRepository,
            },
            s3::S3Storage,
            smtp::SmtpSender,
            security::{Argon2Hasher, JwtCodec, RandomTokens},
            system::SystemClock,
        },
    },
    application::{
        ports::inbound::{DeliverEmails, UserAdminUseCases},
        services::{
            AttachmentService, AuthService, EmailDelivery, HealthService, TicketService, UserAdminService,
            notifications::{NotificationSettings, Notifier},
        },
    },
    config::Config,
};

/// The HTTP app plus the services that background workers drive.
pub struct App {
    pub router: Router,
    pub email_delivery: Arc<dyn DeliverEmails>,
}

pub async fn build_app(pool: PgPool, config: &Config) -> anyhow::Result<App> {
    // Outbound adapters
    let users = Arc::new(PgUserRepository::new(pool.clone()));
    let refresh_tokens = Arc::new(PgRefreshTokenRepository::new(pool.clone()));
    let hasher = Arc::new(Argon2Hasher);
    let jwt = Arc::new(JwtCodec::new(
        config.auth.jwt_secret.as_bytes(),
        Duration::minutes(config.auth.access_ttl_minutes),
    ));

    let clock = Arc::new(SystemClock);
    let ticket_repo = Arc::new(PgTicketRepository::new(pool.clone()));
    let attachment_repo = Arc::new(PgAttachmentRepository::new(pool.clone()));
    let storage = Arc::new(S3Storage::new(&config.storage));
    storage.ensure_bucket().await.context("object storage is not reachable (is the s3 service running?)")?;
    let outbox = Arc::new(PgEmailOutbox::new(pool.clone()));
    let mailer = Arc::new(SmtpSender::new(&config.mail.smtp_url, &config.mail.from)?);
    let notifier = Arc::new(Notifier::new(
        outbox.clone(),
        NotificationSettings { app_url: config.mail.app_url.clone(), mail_domain: mailer.from_domain() },
    ));
    let email_delivery = Arc::new(EmailDelivery::new(outbox, mailer));

    // Services (inbound port implementations)
    let auth = Arc::new(AuthService::new(
        users.clone(),
        refresh_tokens,
        hasher.clone(),
        jwt,
        Arc::new(RandomTokens),
        clock.clone(),
        Duration::days(config.auth.refresh_ttl_days),
    ));
    let tickets = Arc::new(TicketService::new(
        ticket_repo.clone(),
        users.clone(),
        attachment_repo.clone(),
        storage.clone(),
        notifier,
        clock.clone(),
    ));
    let attachments = Arc::new(AttachmentService::new(attachment_repo, ticket_repo, storage, clock));
    let user_admin = Arc::new(UserAdminService::new(users, hasher));
    let health = Arc::new(HealthService::new(Arc::new(PgDatabaseProbe::new(pool))));

    if let Some(admin) = &config.bootstrap_admin {
        let created = user_admin
            .bootstrap_admin(&admin.email, &admin.name, &admin.password)
            .await
            .map_err(|e| anyhow::anyhow!("could not create the bootstrap admin: {e}"))?;
        if created {
            tracing::info!(email = %admin.email, "created bootstrap admin");
        }
    }

    // Inbound adapter
    let origin = config.frontend_origin.parse().context("FRONTEND_ORIGIN must be a valid header value")?;
    let state = HttpState {
        health,
        auth,
        users: user_admin,
        tickets,
        attachments,
        cookies: CookieSettings { secure: config.auth.cookie_secure },
    };
    Ok(App { router: http::router(state, origin), email_delivery })
}
