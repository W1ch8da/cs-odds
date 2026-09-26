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
            postgres::{PgDatabaseProbe, PgRefreshTokenRepository, PgTicketRepository, PgUserRepository},
            security::{Argon2Hasher, JwtCodec, RandomTokens},
            system::SystemClock,
        },
    },
    application::{
        ports::inbound::UserAdminUseCases,
        services::{AuthService, HealthService, TicketService, UserAdminService},
    },
    config::Config,
};

pub async fn build_app(pool: PgPool, config: &Config) -> anyhow::Result<Router> {
    // Outbound adapters
    let users = Arc::new(PgUserRepository::new(pool.clone()));
    let refresh_tokens = Arc::new(PgRefreshTokenRepository::new(pool.clone()));
    let hasher = Arc::new(Argon2Hasher);
    let jwt = Arc::new(JwtCodec::new(
        config.auth.jwt_secret.as_bytes(),
        Duration::minutes(config.auth.access_ttl_minutes),
    ));

    let clock = Arc::new(SystemClock);

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
    let tickets = Arc::new(TicketService::new(Arc::new(PgTicketRepository::new(pool.clone())), users.clone(), clock));
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
        cookies: CookieSettings { secure: config.auth.cookie_secure },
    };
    Ok(http::router(state, origin))
}
