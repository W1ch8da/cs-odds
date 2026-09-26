use std::net::SocketAddr;

use anyhow::{Context, bail};

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub frontend_origin: String,
    pub auth: AuthConfig,
    pub bootstrap_admin: Option<BootstrapAdmin>,
}

#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_ttl_minutes: i64,
    pub refresh_ttl_days: i64,
    pub cookie_secure: bool,
}

// Keep the secret out of logs.
impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("jwt_secret", &"***")
            .field("access_ttl_minutes", &self.access_ttl_minutes)
            .field("refresh_ttl_days", &self.refresh_ttl_days)
            .field("cookie_secure", &self.cookie_secure)
            .finish()
    }
}

/// First admin, created at startup only if no admin exists yet.
#[derive(Clone)]
pub struct BootstrapAdmin {
    pub email: String,
    pub name: String,
    pub password: String,
}

impl std::fmt::Debug for BootstrapAdmin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BootstrapAdmin").field("email", &self.email).field("name", &self.name).finish_non_exhaustive()
    }
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn parse_var<T: std::str::FromStr>(name: &str, default: T) -> anyhow::Result<T> {
    match var(name) {
        Some(v) => v.parse().ok().with_context(|| format!("{name} has an invalid value: {v}")),
        None => Ok(default),
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let bind_addr = parse_var("BIND_ADDR", SocketAddr::from(([0, 0, 0, 0], 8080)))?;
        let frontend_origin = var("FRONTEND_ORIGIN").unwrap_or_else(|| "http://localhost:3000".into());

        let jwt_secret = var("JWT_SECRET").context("JWT_SECRET must be set (at least 32 characters)")?;
        if jwt_secret.len() < 32 {
            bail!("JWT_SECRET must be at least 32 characters");
        }
        let auth = AuthConfig {
            jwt_secret,
            access_ttl_minutes: parse_var("ACCESS_TOKEN_TTL_MINUTES", 15)?,
            refresh_ttl_days: parse_var("REFRESH_TOKEN_TTL_DAYS", 30)?,
            cookie_secure: parse_var("COOKIE_SECURE", false)?,
        };

        let bootstrap_admin = match (var("ADMIN_EMAIL"), var("ADMIN_PASSWORD")) {
            (Some(email), Some(password)) => Some(BootstrapAdmin {
                email,
                password,
                name: var("ADMIN_NAME").unwrap_or_else(|| "Administrator".into()),
            }),
            (None, None) => None,
            _ => bail!("set both ADMIN_EMAIL and ADMIN_PASSWORD, or neither"),
        };

        Ok(Self { database_url, bind_addr, frontend_origin, auth, bootstrap_admin })
    }
}
