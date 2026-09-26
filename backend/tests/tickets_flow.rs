//! Full-stack ticket tests against real Postgres.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{Value, json};
use sqlx::PgPool;

fn timeline_types(ticket: &Value) -> Vec<String> {
    ticket["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| match i["type"].as_str().unwrap() {
            "comment" => format!("comment:{}", if i["internal"].as_bool().unwrap() { "internal" } else { "public" }),
            _ => format!("event:{}", i["kind"].as_str().unwrap()),
        })
        .collect()
}

#[sqlx::test(migrator = "cs_odds_backend::adapters::outbound::postgres::MIGRATOR")]
async fn portal_conversation_end_to_end(pool: PgPool) {
    let app = app(pool).await;
    let jordan = register(&app, "jordan@northwind.com", "Jordan Blake").await;
    let admin = login(&app, ADMIN_EMAIL).await;

    // Customer opens a request.
    let created = call(&app, "POST", "/tickets", Some(json!({
        "subject": "Password reset link says it has expired",
        "description": "Every reset link says expired.\nI have a demo at 3pm.",
        "priority": "high"
    })), Some(&jordan)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{:?}", created.body);
    let n = created.body["number"].as_i64().unwrap();
    assert_eq!(created.body["channel"], "portal");
    assert_eq!(created.body["status"], "open");
    assert_eq!(created.body["requester"]["name"], "Jordan Blake");
    assert!(created.body["assignee"].is_null());

    // Staff: counts and the unassigned queue.
    let counts = call(&app, "GET", "/tickets/counts", None, Some(&admin)).await;
    assert_eq!(counts.body, json!({ "mine": 0, "unassigned": 1, "open": 1, "solved": 0 }));
    let queue = call(&app, "GET", "/tickets?status=open,pending,on_hold&assignee=unassigned", None, Some(&admin)).await;
    assert_eq!(queue.body["total"], 1);
    assert_eq!(queue.body["items"][0]["number"], n);

    // Internal note, priority bump, then a public reply that waits on the customer.
    let uri = format!("/tickets/TKT-{n}");
    call(&app, "POST", &format!("{uri}/comments"), Some(json!({ "body": "Known Outlook Safe Links issue.", "internal": true })), Some(&admin)).await;
    let bumped = call(&app, "PATCH", &uri, Some(json!({ "priority": "urgent" })), Some(&admin)).await;
    assert_eq!(bumped.body["priority"], "urgent");
    let replied = call(&app, "POST", &format!("{uri}/comments"), Some(json!({ "body": "Here is a one-time link.", "status": "pending" })), Some(&admin)).await;
    assert_eq!(replied.status, StatusCode::CREATED);
    assert_eq!(replied.body["status"], "pending");
    assert_eq!(replied.body["assignee"]["name"], "Admin", "replying claims the ticket");
    assert_eq!(
        timeline_types(&replied.body),
        ["comment:internal", "event:priority_changed", "comment:public", "event:assignee_changed", "event:status_changed"]
    );
    let assign = &replied.body["timeline"][3];
    assert_eq!((assign["oldValue"].clone(), assign["newValue"].clone()), (Value::Null, json!("Admin")));

    // The customer sees only the public reply and the status change.
    let customer_view = call(&app, "GET", &uri, None, Some(&jordan)).await;
    assert_eq!(timeline_types(&customer_view.body), ["comment:public", "event:status_changed"]);
    assert!(!customer_view.body.to_string().contains("Outlook"));

    // Customer replies: back to open. Then marks it solved; replying later reopens it.
    let reply = call(&app, "POST", &format!("{uri}/comments"), Some(json!({ "body": "That worked, thanks!" })), Some(&jordan)).await;
    assert_eq!(reply.body["status"], "open");
    let solved = call(&app, "PATCH", &uri, Some(json!({ "status": "solved" })), Some(&jordan)).await;
    assert_eq!(solved.body["status"], "solved");
    assert!(solved.body["resolvedAt"].is_string());
    let reopened = call(&app, "POST", &format!("{uri}/comments"), Some(json!({ "body": "It broke again." })), Some(&jordan)).await;
    assert_eq!(reopened.body["status"], "open");
    assert!(reopened.body["resolvedAt"].is_null());

    // Customers can't reprioritise, post notes or see other people's tickets.
    let forbidden = call(&app, "PATCH", &uri, Some(json!({ "priority": "low" })), Some(&jordan)).await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);
    let note = call(&app, "POST", &format!("{uri}/comments"), Some(json!({ "body": "x", "internal": true })), Some(&jordan)).await;
    assert_eq!(note.status, StatusCode::FORBIDDEN);
    let sofia = register(&app, "sofia@brightline.studio", "Sofia Marquez").await;
    assert_eq!(call(&app, "GET", &uri, None, Some(&sofia)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(call(&app, "GET", "/tickets", None, Some(&sofia)).await.body["total"], 0);
    assert_eq!(call(&app, "GET", "/tickets/counts", None, Some(&sofia)).await.status, StatusCode::FORBIDDEN);

    // Closing is final.
    call(&app, "PATCH", &uri, Some(json!({ "status": "closed" })), Some(&admin)).await;
    let late = call(&app, "POST", &format!("{uri}/comments"), Some(json!({ "body": "Hello?" })), Some(&jordan)).await;
    assert_eq!(late.status, StatusCode::CONFLICT);
    assert_eq!(late.body["error"]["code"], "conflict");
}

#[sqlx::test(migrator = "cs_odds_backend::adapters::outbound::postgres::MIGRATOR")]
async fn agent_queue_filters_search_and_paging(pool: PgPool) {
    let app = app(pool).await;
    let admin = login(&app, ADMIN_EMAIL).await;
    let maya = call(&app, "POST", "/users", Some(json!({
        "email": "maya@cs-odds.example", "name": "Maya Chen", "password": PASSWORD, "role": "agent"
    })), Some(&admin)).await;
    let maya_id = maya.body["id"].as_str().unwrap().to_owned();

    // Staff create tickets on behalf of customers, new and existing.
    let tickets = [
        ("Refund for duplicate charge on INV-2291", "urgent", "sofia@brightline.studio", "Sofia Marquez"),
        ("CSV export times out above 10,000 rows", "normal", "ethan@ridgeview.io", "Ethan Cole"),
        ("How do I add a second admin?", "low", "liam@oakmont.com", "Liam Turner"),
        ("Webhook deliveries failing with 401", "high", "sofia@brightline.studio", ""),
    ];
    let mut numbers = Vec::new();
    for (subject, priority, email, name) in tickets {
        let res = call(&app, "POST", "/tickets", Some(json!({
            "subject": subject, "description": "Logged from a phone call.", "priority": priority,
            "requesterEmail": email, "requesterName": name, "assigneeId": maya_id,
        })), Some(&admin)).await;
        assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.body);
        assert_eq!(res.body["channel"], "agent");
        numbers.push(res.body["number"].as_i64().unwrap());
    }
    let sofia_second = call(&app, "GET", &format!("/tickets/{}", numbers[3]), None, Some(&admin)).await;
    assert_eq!(sofia_second.body["requesterTicketCount"], 2);

    // Priority sort: urgent, high, normal, low.
    let all = call(&app, "GET", "/tickets", None, Some(&admin)).await;
    let order: Vec<&str> = all.body["items"].as_array().unwrap().iter().map(|t| t["priority"].as_str().unwrap()).collect();
    assert_eq!(order, ["urgent", "high", "normal", "low"]);

    // Filters.
    let high = call(&app, "GET", "/tickets?priority=high", None, Some(&admin)).await;
    assert_eq!(high.body["total"], 1);
    let by_maya = call(&app, "GET", &format!("/tickets?assignee={maya_id}"), None, Some(&admin)).await;
    assert_eq!(by_maya.body["total"], 4);
    assert_eq!(call(&app, "GET", "/tickets?assignee=me", None, Some(&admin)).await.body["total"], 0);

    // Search by subject, customer, email and ticket number. `%` is literal.
    for (q, expected) in [("csv", 1), ("sofia", 2), ("ridgeview", 1), (&*format!("TKT-{}", numbers[2]), 1), ("100%", 0)] {
        let res = call(&app, "GET", &format!("/tickets?q={}", urlencode(q)), None, Some(&admin)).await;
        assert_eq!(res.body["total"], expected, "search {q:?}");
    }

    // Paging.
    let page2 = call(&app, "GET", "/tickets?perPage=3&page=2", None, Some(&admin)).await;
    assert_eq!((page2.body["total"].clone(), page2.body["items"].as_array().unwrap().len()), (json!(4), 1));
    assert_eq!(page2.body["perPage"], 3);

    // Bad input gets clear validation errors.
    for bad in ["/tickets?status=done", "/tickets?sort=oldest", "/tickets?assignee=bob"] {
        let res = call(&app, "GET", bad, None, Some(&admin)).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    assert_eq!(call(&app, "GET", "/tickets/not-a-number", None, Some(&admin)).await.status, StatusCode::NOT_FOUND);

    // Unassigning with an explicit null.
    let unassigned = call(&app, "PATCH", &format!("/tickets/{}", numbers[0]), Some(json!({ "assigneeId": null })), Some(&admin)).await;
    assert!(unassigned.body["assignee"].is_null());
    let kept = call(&app, "PATCH", &format!("/tickets/{}", numbers[1]), Some(json!({ "status": "on_hold" })), Some(&admin)).await;
    assert_eq!(kept.body["assignee"]["name"], "Maya Chen", "leaving assigneeId out keeps the assignee");
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
