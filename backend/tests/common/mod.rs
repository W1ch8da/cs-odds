//! Helpers shared by the full-stack tests: real wiring, real Postgres (a
//! fresh database per test), real argon2 and JWT.
#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use cs_odds_backend::{
    adapters::outbound::s3::S3Settings,
    config::{AuthConfig, BootstrapAdmin, Config, MailConfig},
    wiring::{self, App},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

pub const ADMIN_EMAIL: &str = "admin@cs-odds.example";
pub const PASSWORD: &str = "correct horse battery";

pub async fn app(pool: PgPool) -> Router {
    full_app(pool).await.router
}

/// The app plus background services, which tests drive by hand.
pub async fn full_app(pool: PgPool) -> App {
    full_app_with(pool, |_| {}).await
}

/// Like `full_app`, with config changes (e.g. a broken SMTP server).
pub async fn full_app_with(pool: PgPool, customize: impl FnOnce(&mut Config)) -> App {
    let mut config = Config {
        database_url: String::new(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        frontend_origin: "http://localhost:3000".into(),
        auth: AuthConfig {
            jwt_secret: "integration-test-secret-0123456789abcdef".into(),
            access_ttl_minutes: 15,
            refresh_ttl_days: 30,
            cookie_secure: true,
        },
        // The compose "s3" service. Keys are random ids, so tests can share a bucket.
        storage: S3Settings {
            endpoint: "http://localhost:9000".into(),
            public_endpoint: "http://localhost:9000".into(),
            region: "us-east-1".into(),
            bucket: "cs-odds-test".into(),
            access_key: "csodds".into(),
            secret_key: "csodds-dev-secret".into(),
        },
        // The compose Mailpit service.
        mail: MailConfig {
            smtp_url: "smtp://localhost:1025".into(),
            from: "CS-ODDS Support <support@cs-odds.test>".into(),
            app_url: "http://localhost:3000".into(),
            worker_interval_seconds: 5,
        },
        bootstrap_admin: Some(BootstrapAdmin { email: ADMIN_EMAIL.into(), name: "Admin".into(), password: PASSWORD.into() }),
    };
    customize(&mut config);
    wiring::build_app(pool, &config).await.unwrap()
}

pub struct Res {
    pub status: StatusCode,
    pub body: Value,
    pub cookies: Vec<String>,
    pub location: Option<String>,
}

impl Res {
    /// `name=value` pairs to send back, like a browser would.
    pub fn cookie_header(&self) -> String {
        self.cookies.iter().map(|c| c.split(';').next().unwrap()).collect::<Vec<_>>().join("; ")
    }
}

pub async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>, cookie: Option<&str>) -> Res {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let req = match body {
        Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let cookies = res.headers().get_all(header::SET_COOKIE).iter().map(|v| v.to_str().unwrap().to_owned()).collect();
    let location = res.headers().get(header::LOCATION).map(|v| v.to_str().unwrap().to_owned());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    Res { status, body, cookies, location }
}

/// Registers a customer and returns their cookie header.
pub async fn register(app: &Router, email: &str, name: &str) -> String {
    let res = call(app, "POST", "/auth/register", Some(json!({ "email": email, "name": name, "password": PASSWORD })), None).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.body);
    res.cookie_header()
}

/// Signs in and returns the cookie header.
pub async fn login(app: &Router, email: &str) -> String {
    let res = call(app, "POST", "/auth/login", Some(json!({ "email": email, "password": PASSWORD })), None).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
    res.cookie_header()
}
