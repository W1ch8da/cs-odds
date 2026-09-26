//! Full-stack auth tests against real Postgres.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrator = "cs_odds_backend::adapters::outbound::postgres::MIGRATOR")]
async fn customer_session_lifecycle(pool: PgPool) {
    let app = app(pool.clone()).await;

    let reg = call(&app, "POST", "/auth/register",
        Some(json!({ "email": "Jordan@Northwind.com", "name": "Jordan Blake", "password": PASSWORD })), None).await;
    assert_eq!(reg.status, StatusCode::CREATED, "{:?}", reg.body);
    assert!(reg.cookies.iter().all(|c| c.contains("Secure") && c.contains("HttpOnly")));

    // Stored hashed with argon2id, never in plain text.
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE email = 'jordan@northwind.com'")
        .fetch_one(&pool).await.unwrap();
    assert!(hash.starts_with("$argon2id$"));

    let dup = call(&app, "POST", "/auth/register",
        Some(json!({ "email": "jordan@northwind.com", "name": "J", "password": PASSWORD })), None).await;
    assert_eq!(dup.status, StatusCode::CONFLICT);

    let me = call(&app, "GET", "/auth/me", None, Some(&reg.cookie_header())).await;
    assert_eq!(me.body["email"], "jordan@northwind.com");
    assert_eq!(me.body["role"], "customer");

    let refreshed = call(&app, "POST", "/auth/refresh", None, Some(&reg.cookie_header())).await;
    assert_eq!(refreshed.status, StatusCode::OK);

    // Reusing the first refresh token revokes every session for the user.
    let replay = call(&app, "POST", "/auth/refresh", None, Some(&reg.cookie_header())).await;
    assert_eq!(replay.status, StatusCode::UNAUTHORIZED);
    let after = call(&app, "POST", "/auth/refresh", None, Some(&refreshed.cookie_header())).await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
    let active: i64 = sqlx::query_scalar("SELECT count(*) FROM refresh_tokens WHERE revoked_at IS NULL")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(active, 0);

    // Signing in again starts a fresh session; logout revokes it.
    let login = call(&app, "POST", "/auth/login",
        Some(json!({ "email": "jordan@northwind.com", "password": PASSWORD })), None).await;
    assert_eq!(login.status, StatusCode::OK);
    let logout = call(&app, "POST", "/auth/logout", None, Some(&login.cookie_header())).await;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);
    let dead = call(&app, "POST", "/auth/refresh", None, Some(&login.cookie_header())).await;
    assert_eq!(dead.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "cs_odds_backend::adapters::outbound::postgres::MIGRATOR")]
async fn admin_manages_team(pool: PgPool) {
    let app = app(pool.clone()).await;
    // Building the app twice must not create a second admin.
    let _ = common::app(pool.clone()).await;
    let admins: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE role = 'admin'").fetch_one(&pool).await.unwrap();
    assert_eq!(admins, 1);

    let admin = call(&app, "POST", "/auth/login", Some(json!({ "email": ADMIN_EMAIL, "password": PASSWORD })), None).await;
    assert_eq!(admin.status, StatusCode::OK);
    assert_eq!(admin.body["user"]["role"], "admin");

    let agent = json!({ "email": "maya@cs-odds.example", "name": "Maya Chen", "password": PASSWORD, "role": "agent" });
    let created = call(&app, "POST", "/users", Some(agent), Some(&admin.cookie_header())).await;
    assert_eq!(created.status, StatusCode::CREATED, "{:?}", created.body);

    let customer = register(&app, "jordan@northwind.com", "Jordan Blake").await;

    let staff = call(&app, "GET", "/agents", None, Some(&admin.cookie_header())).await;
    let names: Vec<&str> = staff.body.as_array().unwrap().iter().map(|u| u["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Admin", "Maya Chen"]);

    let customers = call(&app, "GET", "/users?role=customer", None, Some(&admin.cookie_header())).await;
    assert_eq!(customers.body.as_array().unwrap().len(), 1);

    let forbidden = call(&app, "GET", "/users", None, Some(&customer)).await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    // The new agent can sign in.
    let maya = call(&app, "POST", "/auth/login", Some(json!({ "email": "maya@cs-odds.example", "password": PASSWORD })), None).await;
    assert_eq!(maya.body["user"]["role"], "agent");
}
