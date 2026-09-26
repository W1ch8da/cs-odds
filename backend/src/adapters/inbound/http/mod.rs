//! Axum HTTP adapter. Handlers only parse requests, call inbound ports and
//! map results to responses; no business logic lives here.

mod cookies;
mod dto;
mod error;
mod extractors;
mod handlers;
mod state;

use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::{get, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub use state::{CookieSettings, HttpState};

pub fn router(state: HttpState, frontend_origin: HeaderValue) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(frontend_origin)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);

    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/refresh", post(handlers::auth::refresh))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::me))
        .route("/users", get(handlers::users::list).post(handlers::users::create))
        .route("/agents", get(handlers::users::agents))
        .route("/tickets", get(handlers::tickets::list).post(handlers::tickets::create))
        .route("/tickets/counts", get(handlers::tickets::counts))
        .route("/tickets/{number}", get(handlers::tickets::get).patch(handlers::tickets::update))
        .route("/tickets/{number}/comments", post(handlers::tickets::add_comment))
        .route("/attachments", post(handlers::attachments::start_upload))
        .route("/attachments/{id}/download", get(handlers::attachments::download))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header::COOKIE, header::SET_COOKIE},
    };
    use chrono::Duration;
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;
    use crate::{
        application::{
            error::{AppError, AppResult},
            ports::inbound::{CheckHealth, HealthReport, UserAdminUseCases},
            services::{AttachmentService, AuthService, UserAdminService, test_support::*},
        },
    };

    struct StubHealth(bool);

    #[async_trait]
    impl CheckHealth for StubHealth {
        async fn check(&self) -> AppResult<HealthReport> {
            if self.0 {
                Ok(HealthReport { status: "ok", database: "ok" })
            } else {
                Err(AppError::Unexpected(anyhow::anyhow!("db down")))
            }
        }
    }

    struct TestApp {
        app: Router,
        users: Arc<UserAdminService>,
    }

    fn test_app(healthy: bool) -> TestApp {
        let env = TestEnv::default();
        let auth = AuthService::new(
            env.users.clone(),
            env.refresh_tokens.clone(),
            Arc::new(FakeHasher),
            Arc::new(FakeCodec),
            Arc::new(SeqTokens::default()),
            env.clock.clone(),
            Duration::days(30),
        );
        let users = Arc::new(UserAdminService::new(env.users.clone(), Arc::new(FakeHasher)));
        let tickets = Arc::new(env.ticket_service());
        let attachments = Arc::new(AttachmentService::new(
            env.attachments.clone(),
            env.tickets.clone(),
            env.storage.clone(),
            env.clock.clone(),
        ));
        let state = HttpState {
            health: Arc::new(StubHealth(healthy)),
            auth: Arc::new(auth),
            users: users.clone(),
            tickets,
            attachments,
            cookies: CookieSettings { secure: false },
        };
        TestApp { app: router(state, HeaderValue::from_static("http://localhost:3000")), users }
    }

    struct Res {
        status: StatusCode,
        body: Value,
        cookies: Vec<String>,
    }

    impl Res {
        /// `name=value` pairs to send back, like a browser would.
        fn cookie_header(&self) -> String {
            self.cookies.iter().map(|c| c.split(';').next().unwrap().to_string()).collect::<Vec<_>>().join("; ")
        }
    }

    async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>, cookie: Option<&str>) -> Res {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(c) = cookie {
            req = req.header(COOKIE, c);
        }
        let req = match body {
            Some(b) => req.header("content-type", "application/json").body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let cookies = res.headers().get_all(SET_COOKIE).iter().map(|v| v.to_str().unwrap().to_string()).collect();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
        Res { status, body, cookies }
    }

    fn jordan() -> Value {
        json!({ "email": "jordan@northwind.com", "name": "Jordan Blake", "password": "correct horse" })
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let t = test_app(true);
        let res = call(&t.app, "GET", "/health", None, None).await;
        assert_eq!(res.status, StatusCode::OK);
        assert_eq!(res.body["status"], "ok");
    }

    #[tokio::test]
    async fn internal_errors_are_not_leaked() {
        let t = test_app(false);
        let res = call(&t.app, "GET", "/health", None, None).await;
        assert_eq!(res.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(res.body["error"]["code"], "internal");
        assert!(!res.body["error"]["message"].as_str().unwrap().contains("db down"));
    }

    #[tokio::test]
    async fn register_sets_httponly_cookies_and_me_works() {
        let t = test_app(true);
        let res = call(&t.app, "POST", "/auth/register", Some(jordan()), None).await;
        assert_eq!(res.status, StatusCode::CREATED);
        assert_eq!(res.body["user"]["role"], "customer");
        assert!(res.body.get("accessToken").is_none(), "tokens must not appear in the body");
        assert_eq!(res.cookies.len(), 2);
        assert!(res.cookies.iter().all(|c| c.contains("HttpOnly") && c.contains("SameSite=Lax") && c.contains("Path=/")));

        let me = call(&t.app, "GET", "/auth/me", None, Some(&res.cookie_header())).await;
        assert_eq!(me.status, StatusCode::OK);
        assert_eq!(me.body["email"], "jordan@northwind.com");
        assert_eq!(me.body["createdAt"].is_string(), true);
    }

    #[tokio::test]
    async fn me_requires_authentication() {
        let t = test_app(true);
        let res = call(&t.app, "GET", "/auth/me", None, None).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
        assert_eq!(res.body["error"]["code"], "unauthorized");
    }

    #[tokio::test]
    async fn validation_and_bad_json_use_standard_error_shape() {
        let t = test_app(true);
        let weak = json!({ "email": "jordan@northwind.com", "name": "Jordan", "password": "short" });
        let res = call(&t.app, "POST", "/auth/register", Some(weak), None).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(res.body["error"]["code"], "validation");

        let res = call(&t.app, "POST", "/auth/login", Some(json!({ "email": 1 })), None).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(res.body["error"]["code"], "validation");
    }

    #[tokio::test]
    async fn login_refresh_logout_flow() {
        let t = test_app(true);
        call(&t.app, "POST", "/auth/register", Some(jordan()), None).await;

        let bad = call(&t.app, "POST", "/auth/login", Some(json!({ "email": "jordan@northwind.com", "password": "nope nope nope" })), None).await;
        assert_eq!(bad.status, StatusCode::UNAUTHORIZED);
        assert_eq!(bad.body["error"]["code"], "invalid_credentials");

        let login = call(&t.app, "POST", "/auth/login", Some(json!({ "email": "jordan@northwind.com", "password": "correct horse" })), None).await;
        assert_eq!(login.status, StatusCode::OK);

        let refreshed = call(&t.app, "POST", "/auth/refresh", None, Some(&login.cookie_header())).await;
        assert_eq!(refreshed.status, StatusCode::OK);
        assert_ne!(refreshed.cookie_header(), login.cookie_header());

        // Replaying the old refresh token fails and clears the cookies.
        let replay = call(&t.app, "POST", "/auth/refresh", None, Some(&login.cookie_header())).await;
        assert_eq!(replay.status, StatusCode::UNAUTHORIZED);
        assert!(replay.cookies.iter().all(|c| c.contains("Max-Age=0")));

        let logout = call(&t.app, "POST", "/auth/logout", None, Some(&refreshed.cookie_header())).await;
        assert_eq!(logout.status, StatusCode::NO_CONTENT);
        assert!(logout.cookies.iter().all(|c| c.contains("Max-Age=0")));
    }

    #[tokio::test]
    async fn only_admins_manage_users() {
        let t = test_app(true);
        t.users.bootstrap_admin("admin@cs-odds.example", "Admin", "correct horse").await.unwrap();
        let admin = call(&t.app, "POST", "/auth/login", Some(json!({ "email": "admin@cs-odds.example", "password": "correct horse" })), None).await;
        let customer = call(&t.app, "POST", "/auth/register", Some(jordan()), None).await;

        let new_agent = json!({ "email": "maya@cs-odds.example", "name": "Maya Chen", "password": "correct horse", "role": "agent" });
        let denied = call(&t.app, "POST", "/users", Some(new_agent.clone()), Some(&customer.cookie_header())).await;
        assert_eq!(denied.status, StatusCode::FORBIDDEN);

        let created = call(&t.app, "POST", "/users", Some(new_agent), Some(&admin.cookie_header())).await;
        assert_eq!(created.status, StatusCode::CREATED);
        assert_eq!(created.body["role"], "agent");

        let customers = call(&t.app, "GET", "/users?role=customer", None, Some(&admin.cookie_header())).await;
        assert_eq!(customers.body.as_array().unwrap().len(), 1);

        let bad_role = call(&t.app, "GET", "/users?role=owner", None, Some(&admin.cookie_header())).await;
        assert_eq!(bad_role.status, StatusCode::UNPROCESSABLE_ENTITY);

        let agents = call(&t.app, "GET", "/agents", None, Some(&admin.cookie_header())).await;
        assert_eq!(agents.body.as_array().unwrap().len(), 2);
        let agents_as_customer = call(&t.app, "GET", "/agents", None, Some(&customer.cookie_header())).await;
        assert_eq!(agents_as_customer.status, StatusCode::FORBIDDEN);
    }
}
