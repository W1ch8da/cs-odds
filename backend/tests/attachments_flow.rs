//! Full-stack attachment tests: real Postgres and real S3-compatible storage
//! (the compose `s3` service). Bytes go over the presigned URLs exactly as a
//! browser would send them.

mod common;

use axum::http::{StatusCode, header};
use common::*;
use serde_json::{Value, json};
use sqlx::PgPool;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n pretend screenshot bytes";

async fn start_upload(app: &axum::Router, who: &str, filename: &str, size: usize) -> Value {
    let res = call(app, "POST", "/attachments", Some(json!({ "filename": filename, "contentType": "image/png", "size": size })), Some(who)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.body);
    res.body
}

/// Uploads like the browser does: a single PUT with exactly the signed headers.
async fn put_bytes(slot: &Value, bytes: &[u8]) {
    let client = reqwest::Client::new();
    let mut req = client.put(slot["uploadUrl"].as_str().unwrap()).body(bytes.to_vec());
    for (name, value) in slot["uploadHeaders"].as_object().unwrap() {
        req = req.header(name, value.as_str().unwrap());
    }
    let res = req.send().await.unwrap();
    assert!(res.status().is_success(), "upload failed: {} {}", res.status(), res.text().await.unwrap_or_default());
}

fn redirect_target(res: &Res) -> String {
    assert_eq!(res.status, StatusCode::SEE_OTHER, "{:?}", res.body);
    res.location.clone().expect("Location header")
}

#[sqlx::test(migrator = "cs_odds_backend::adapters::outbound::postgres::MIGRATOR")]
async fn upload_attach_and_download(pool: PgPool) {
    let app = app(pool).await;
    let jordan = register(&app, "jordan@northwind.example", "Jordan Blake").await;
    let sofia = register(&app, "sofia@brightline.example", "Sofia Marquez").await;
    let admin = login(&app, ADMIN_EMAIL).await;

    // Customer attaches a screenshot to a new request.
    let slot = start_upload(&app, &jordan, "crash screen.png", PNG.len()).await;
    let id = slot["attachment"]["id"].as_str().unwrap().to_owned();
    put_bytes(&slot, PNG).await;
    let ticket = call(&app, "POST", "/tickets", Some(json!({
        "subject": "App crashes when opening images",
        "description": "Screenshot attached.",
        "attachmentIds": [id],
    })), Some(&jordan)).await;
    assert_eq!(ticket.status, StatusCode::CREATED, "{:?}", ticket.body);
    assert_eq!(ticket.body["attachments"][0]["filename"], "crash screen.png");
    assert_eq!(ticket.body["attachments"][0]["size"], PNG.len());
    let n = ticket.body["number"].as_i64().unwrap();

    // Download: the API checks access, then redirects to storage.
    let res = call(&app, "GET", &format!("/attachments/{id}/download"), None, Some(&jordan)).await;
    let file = reqwest::get(redirect_target(&res)).await.unwrap();
    assert!(file.status().is_success());
    let disposition = file.headers()[header::CONTENT_DISPOSITION.as_str()].to_str().unwrap().to_owned();
    assert!(disposition.starts_with("attachment;") && disposition.contains("crash%20screen.png"), "{disposition}");
    assert_eq!(file.bytes().await.unwrap().as_ref(), PNG);

    // Staff can download it; another customer can't even tell it exists.
    assert_eq!(call(&app, "GET", &format!("/attachments/{id}/download"), None, Some(&admin)).await.status, StatusCode::SEE_OTHER);
    assert_eq!(call(&app, "GET", &format!("/attachments/{id}/download"), None, Some(&sofia)).await.status, StatusCode::NOT_FOUND);

    // A file can't be attached twice.
    let reuse = call(&app, "POST", &format!("/tickets/{n}/comments"), Some(json!({ "body": "again", "attachmentIds": [id] })), Some(&jordan)).await;
    assert_eq!(reuse.status, StatusCode::UNPROCESSABLE_ENTITY);

    // An internal note's file stays hidden from the customer.
    let log = start_upload(&app, &admin, "server.log", 5).await;
    put_bytes(&log, b"trace").await;
    let log_id = log["attachment"]["id"].as_str().unwrap().to_owned();
    let noted = call(&app, "POST", &format!("/tickets/{n}/comments"), Some(json!({
        "body": "Crash trace attached", "internal": true, "attachmentIds": [log_id],
    })), Some(&admin)).await;
    assert_eq!(noted.body["timeline"][0]["attachments"][0]["filename"], "server.log");
    let customer_view = call(&app, "GET", &format!("/tickets/{n}"), None, Some(&jordan)).await;
    assert!(!customer_view.body.to_string().contains("server.log"));
    assert_eq!(call(&app, "GET", &format!("/attachments/{log_id}/download"), None, Some(&jordan)).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "cs_odds_backend::adapters::outbound::postgres::MIGRATOR")]
async fn rejects_unfinished_oversized_and_bad_uploads(pool: PgPool) {
    let app = app(pool).await;
    let jordan = register(&app, "jordan@northwind.example", "Jordan Blake").await;

    // Slot issued but nothing uploaded.
    let pending = start_upload(&app, &jordan, "later.png", 10).await;
    let res = call(&app, "POST", "/tickets", Some(json!({
        "subject": "Missing file", "description": "Where did it go?", "attachmentIds": [pending["attachment"]["id"]],
    })), Some(&jordan)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res.body["error"]["message"].as_str().unwrap().contains("hasn't finished uploading"));

    // Uploaded bytes don't match the declared size.
    let liar = start_upload(&app, &jordan, "small.png", 3).await;
    put_bytes(&liar, PNG).await;
    let res = call(&app, "POST", "/tickets", Some(json!({
        "subject": "Size mismatch", "description": "Declared 3 bytes.", "attachmentIds": [liar["attachment"]["id"]],
    })), Some(&jordan)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Too big, empty name, not signed in, unknown id.
    let big = call(&app, "POST", "/attachments", Some(json!({ "filename": "huge.zip", "size": 30_000_000 })), Some(&jordan)).await;
    assert_eq!(big.status, StatusCode::UNPROCESSABLE_ENTITY);
    let nameless = call(&app, "POST", "/attachments", Some(json!({ "filename": " ", "size": 1 })), Some(&jordan)).await;
    assert_eq!(nameless.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(call(&app, "POST", "/attachments", Some(json!({ "filename": "a.png", "size": 1 })), None).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "GET", "/attachments/not-a-uuid/download", None, Some(&jordan)).await.status, StatusCode::NOT_FOUND);
}
