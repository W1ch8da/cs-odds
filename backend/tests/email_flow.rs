//! Full-stack email tests: real Postgres outbox and real SMTP delivery into
//! the compose Mailpit service, checked through Mailpit's HTTP API.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{Value, json};
use sqlx::PgPool;

const MAILPIT: &str = "http://localhost:8025/api/v1";

/// Finds delivered messages whose subject contains `needle`.
async fn mailpit_search(needle: &str) -> Vec<Value> {
    let url = format!("{MAILPIT}/search?query={}", urlencode(&format!("subject:\"{needle}\"")));
    let res: Value = reqwest::get(url).await.unwrap().json().await.unwrap();
    res["messages"].as_array().cloned().unwrap_or_default()
}

async fn mailpit_message(id: &str) -> Value {
    reqwest::get(format!("{MAILPIT}/message/{id}")).await.unwrap().json().await.unwrap()
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[sqlx::test(migrator = "cs_odds_backend::adapters::outbound::postgres::MIGRATOR")]
async fn notifications_are_delivered_over_smtp(pool: PgPool) {
    let app = full_app(pool.clone()).await;
    let jordan = register(&app.router, "jordan@northwind.example", "Jordan Blake").await;
    let admin = login(&app.router, ADMIN_EMAIL).await;

    // A unique subject keeps this test's mail apart from anything else in Mailpit.
    let subject = format!("Printer on fire {}", uuid::Uuid::new_v4().simple());
    let created = call(&app.router, "POST", "/tickets", Some(json!({ "subject": subject, "description": "Smoke everywhere." })), Some(&jordan)).await;
    let n = created.body["number"].as_i64().unwrap();
    call(&app.router, "POST", &format!("/tickets/{n}/comments"), Some(json!({ "body": "Unplug it now, please.", "status": "pending" })), Some(&admin)).await;
    call(&app.router, "POST", &format!("/tickets/{n}/comments"), Some(json!({ "body": "Checking the fuse box.", "internal": true })), Some(&admin)).await;

    // Nothing is sent during requests; the worker delivers the queue.
    let report = app.email_delivery.deliver_due().await.unwrap();
    assert_eq!((report.sent, report.retrying, report.failed), (2, 0, 0));
    assert_eq!(app.email_delivery.deliver_due().await.unwrap().sent, 0, "nothing is sent twice");

    let mut messages = mailpit_search(&subject).await;
    messages.sort_by_key(|m| m["Subject"].as_str().unwrap().to_owned());
    let subjects: Vec<&str> = messages.iter().map(|m| m["Subject"].as_str().unwrap()).collect();
    assert_eq!(subjects, [format!("Re: [#TKT-{n}] {subject}"), format!("[#TKT-{n}] {subject}")]);
    for m in &messages {
        assert_eq!(m["To"][0]["Address"], "jordan@northwind.example");
        assert_eq!(m["From"]["Address"], "support@cs-odds.test");
    }

    // The reply email carries the reply text, the status note and a portal link; never the internal note.
    let reply = mailpit_message(messages[0]["ID"].as_str().unwrap()).await;
    let text = reply["Text"].as_str().unwrap();
    assert!(text.contains("> Unplug it now, please."), "{text}");
    assert!(text.contains("waiting for your reply"));
    assert!(text.contains(&format!("http://localhost:3000/portal/requests/{n}")));
    assert!(!text.contains("fuse box"));
    assert!(reply["HTML"].as_str().unwrap().contains("Unplug it now"));
    assert!(reply["MessageID"].as_str().unwrap().ends_with("@cs-odds.test"));

    let row: (String, i32) = sqlx::query_as("SELECT status::text, attempts FROM email_outbox LIMIT 1").fetch_one(&pool).await.unwrap();
    assert_eq!(row, ("sent".to_owned(), 1));
}

#[sqlx::test(migrator = "cs_odds_backend::adapters::outbound::postgres::MIGRATOR")]
async fn failed_delivery_is_retried_later(pool: PgPool) {
    // Nothing listens on port 1.
    let app = full_app_with(pool.clone(), |c| c.mail.smtp_url = "smtp://127.0.0.1:1".into()).await;
    let jordan = register(&app.router, "jordan@northwind.example", "Jordan Blake").await;
    let res = call(&app.router, "POST", "/tickets", Some(json!({ "subject": "Mail server down", "description": "Test." })), Some(&jordan)).await;
    assert_eq!(res.status, StatusCode::CREATED, "a broken mail server must not break the request");

    let report = app.email_delivery.deliver_due().await.unwrap();
    assert_eq!((report.sent, report.retrying, report.failed), (0, 1, 0));

    let (status, attempts, error, wait): (String, i32, Option<String>, f64) = sqlx::query_as(
        "SELECT status::text, attempts, last_error, EXTRACT(EPOCH FROM next_attempt_at - now())::float8 FROM email_outbox",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((status.as_str(), attempts), ("pending", 1));
    assert!(error.unwrap().contains("SMTP"));
    assert!((50.0..70.0).contains(&wait), "retries in about a minute, got {wait}s");

    // Not due yet, so the next run leaves it alone.
    assert_eq!(app.email_delivery.deliver_due().await.unwrap(), Default::default());
}
