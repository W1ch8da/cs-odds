//! Helpers shared by the full-stack tests: real wiring, real Postgres (a
//! fresh database per test), real argon2 and JWT.
#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use cs_odds_backend::{
    config::{AuthConfig, BootstrapAdmin, Config},
    wiring,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

pub const ADMIN_EMAIL: &str = "admin@cs-odds.example";
pub const PASSWORD: &str = "correct horse battery";

pub async fn app(pool: PgPool) -> Router {
    let config = Config {
        database_url: String::new(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        frontend_origin: "http://localhost:3000".into(),
        auth: AuthConfig {
            jwt_secret: "integration-test-secret-0123456789abcdef".into(),
            access_ttl_minutes: 15,
            refresh_ttl_days: 30,
            cookie_secure: true,
        },
        bootstrap_admin: Some(BootstrapAdmin { email: ADMIN_EMAIL.into(), name: "Admin".into(), password: PASSWORD.into() }),
    };
    wiring::build_app(pool, &config).await.unwrap()
}

pub struct Res {
    pub status: StatusCode,
    pub body: Value,
    pub cookies: Vec<String>,
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
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    Res { status, body, cookies }
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
