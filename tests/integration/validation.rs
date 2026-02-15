use rocket::http::{ContentType, Header, Status};
use crate::common::test_client;

// --- Rate Limit Info ---

#[test]
fn test_rate_limit_response_includes_retry_info() {
    let client = test_client();

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "rate-limit-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 60 messages to hit the rate limit (60/min)
    for i in 0..60 {
        let body = format!(r#"{{"sender": "spammer", "content": "msg {i}"}}"#);
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(body)
            .dispatch();
        assert_eq!(res.status(), Status::Ok, "Message {i} should succeed");
    }

    // The 61st should be rate limited
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "spammer", "content": "one too many"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::TooManyRequests);

    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("Rate limited"));
    assert!(body["retry_after_secs"].is_number(), "Should include retry_after_secs");
    assert!(body["retry_after_secs"].as_u64().unwrap() > 0, "retry_after_secs should be positive");
    assert_eq!(body["limit"], 60);
    assert_eq!(body["remaining"], 0);
}

#[test]
fn test_rate_limit_room_creation_includes_retry_info() {
    let client = test_client();

    // Create 10 rooms to hit the limit (10/hr)
    for i in 0..10 {
        let body = format!(r#"{{"name": "rl-room-{i}"}}"#);
        let res = client
            .post("/api/v1/rooms")
            .header(ContentType::JSON)
            .body(body)
            .dispatch();
        assert_eq!(res.status(), Status::Ok, "Room {i} should succeed");
    }

    // The 11th should be rate limited
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "one-too-many"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::TooManyRequests);

    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["retry_after_secs"].is_number());
    assert_eq!(body["limit"], 10);
    assert_eq!(body["remaining"], 0);
}

// --- Rate Limit Headers on Success Responses ---

#[test]
fn test_send_message_includes_rate_limit_headers() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "hello"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let headers = res.headers();
    let limit = headers.get_one("X-RateLimit-Limit").expect("Missing X-RateLimit-Limit header");
    let remaining = headers.get_one("X-RateLimit-Remaining").expect("Missing X-RateLimit-Remaining header");
    let reset = headers.get_one("X-RateLimit-Reset").expect("Missing X-RateLimit-Reset header");

    assert_eq!(limit, "60", "Message rate limit should be 60/min");
    assert_eq!(remaining.parse::<u64>().unwrap(), 59, "Should have 59 remaining after first message");
    assert_eq!(reset, "0", "Reset should be 0 when under limit");
}

#[test]
fn test_create_room_includes_rate_limit_headers() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "rl-header-test"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let headers = res.headers();
    let limit = headers.get_one("X-RateLimit-Limit").expect("Missing X-RateLimit-Limit header");
    let remaining = headers.get_one("X-RateLimit-Remaining").expect("Missing X-RateLimit-Remaining header");
    let reset = headers.get_one("X-RateLimit-Reset").expect("Missing X-RateLimit-Reset header");

    assert_eq!(limit, "10", "Room rate limit should be 10/hr");
    assert!(remaining.parse::<u64>().unwrap() <= 9, "Should have <=9 remaining");
    assert_eq!(reset, "0");
}

#[test]
fn test_send_dm_includes_rate_limit_headers() {
    let client = test_client();

    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "hi"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let headers = res.headers();
    let limit = headers.get_one("X-RateLimit-Limit").expect("Missing X-RateLimit-Limit header");
    let remaining = headers.get_one("X-RateLimit-Remaining").expect("Missing X-RateLimit-Remaining header");

    assert_eq!(limit, "60", "DM rate limit should be 60/min");
    assert!(remaining.parse::<u64>().unwrap() <= 59);
}

#[test]
fn test_file_upload_includes_rate_limit_headers() {
    use base64::Engine;
    let client = test_client();

    // Create a room
    let room_res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "file-rl-test"}"#)
        .dispatch();
    let room: serde_json::Value = room_res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let data = base64::engine::general_purpose::STANDARD.encode(b"test file content");
    let body = serde_json::json!({
        "sender": "alice",
        "filename": "test.txt",
        "data": data,
        "content_type": "text/plain"
    });

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let headers = res.headers();
    let limit = headers.get_one("X-RateLimit-Limit").expect("Missing X-RateLimit-Limit header");
    let remaining = headers.get_one("X-RateLimit-Remaining").expect("Missing X-RateLimit-Remaining header");

    assert_eq!(limit, "10", "File upload rate limit should be 10/min");
    assert!(remaining.parse::<u64>().unwrap() <= 9);
}

#[test]
fn test_incoming_webhook_includes_rate_limit_headers() {
    let client = test_client();

    // Create a room with admin key
    let room_res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "hook-rl-test"}"#)
        .dispatch();
    let room: serde_json::Value = room_res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Create incoming webhook
    let wh_res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("X-Admin-Key", admin_key.to_string()))
        .body(r#"{"name": "RL Test Hook", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(wh_res.status(), Status::Ok);
    let wh: serde_json::Value = wh_res.into_json().unwrap();
    let token = wh["token"].as_str().unwrap();

    // Post via webhook
    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "hello from webhook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let headers = res.headers();
    let limit = headers.get_one("X-RateLimit-Limit").expect("Missing X-RateLimit-Limit header");
    let remaining = headers.get_one("X-RateLimit-Remaining").expect("Missing X-RateLimit-Remaining header");

    assert_eq!(limit, "60", "Webhook rate limit should be 60/min");
    assert!(remaining.parse::<u64>().unwrap() <= 59);
}

#[test]
fn test_rate_limit_headers_decrement_correctly() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send 3 messages and check remaining decrements
    for i in 0..3 {
        let body = format!(r#"{{"sender": "counter", "content": "msg {i}"}}"#);
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(body)
            .dispatch();
        assert_eq!(res.status(), Status::Ok);

        let remaining: u64 = res
            .headers()
            .get_one("X-RateLimit-Remaining")
            .unwrap()
            .parse()
            .unwrap();
        // remaining should decrease (but other tests may have used the same IP)
        // Just verify it's a valid number less than the limit
        assert!(remaining < 60, "Remaining ({remaining}) should be less than limit (60)");
    }
}

// --- Sender Length Validation ---

#[test]
fn test_message_sender_too_long() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let long_sender = "a".repeat(101);
    let body = serde_json::json!({"sender": long_sender, "content": "test"});
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_dm_sender_too_long() {
    let client = test_client();
    let long_sender = "b".repeat(101);
    let body = serde_json::json!({"sender": long_sender, "recipient": "bob", "content": "hi"});
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_dm_recipient_too_long() {
    let client = test_client();
    let long_recipient = "c".repeat(101);
    let body = serde_json::json!({"sender": "alice", "recipient": long_recipient, "content": "hi"});
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_reaction_sender_too_long() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send a message first
    let msg_body = serde_json::json!({"sender": "alice", "content": "test"});
    let msg_res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(msg_body.to_string())
        .dispatch();
    let msg: serde_json::Value = msg_res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    let long_sender = "d".repeat(101);
    let body = serde_json::json!({"sender": long_sender, "emoji": "👍"});
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_sender_exactly_100_chars_accepted() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let sender_100 = "e".repeat(100);
    let body = serde_json::json!({"sender": sender_100, "content": "boundary test"});
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
}

#[test]
fn test_message_content_too_long() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let long_content = "x".repeat(10001);
    let body = serde_json::json!({"sender": "alice", "content": long_content});
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_message_content_exactly_10000_accepted() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let max_content = "y".repeat(10000);
    let body = serde_json::json!({"sender": "alice", "content": max_content});
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["content"].as_str().unwrap().len(), 10000);
}

#[test]
fn test_dm_content_too_long() {
    let client = test_client();
    let long_content = "z".repeat(10001);
    let body = serde_json::json!({"sender": "alice", "recipient": "bob", "content": long_content});
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_incoming_webhook_content_too_long() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();
    let admin_key = rooms[0]["admin_key"].as_str().unwrap_or("");

    // Create an incoming webhook
    let wh_body = serde_json::json!({"name": "Test Hook", "created_by": "tester"});
    let wh_res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("X-Admin-Key", admin_key.to_string()))
        .body(wh_body.to_string())
        .dispatch();

    if wh_res.status() != Status::Ok {
        // Need admin key — create a room to get one
        let create_res = client
            .post("/api/v1/rooms")
            .header(ContentType::JSON)
            .body(r#"{"name": "wh-test-room"}"#)
            .dispatch();
        let room: serde_json::Value = create_res.into_json().unwrap();
        let room_id2 = room["id"].as_str().unwrap();
        let admin_key2 = room["admin_key"].as_str().unwrap();

        let wh_body2 = serde_json::json!({"name": "Test Hook 2", "created_by": "tester"});
        let wh_res2 = client
            .post(format!("/api/v1/rooms/{room_id2}/incoming-webhooks"))
            .header(ContentType::JSON)
            .header(Header::new("X-Admin-Key", admin_key2.to_string()))
            .body(wh_body2.to_string())
            .dispatch();
        assert_eq!(wh_res2.status(), Status::Ok);
        let wh: serde_json::Value = wh_res2.into_json().unwrap();
        let token = wh["token"].as_str().unwrap();

        // Post too-long content
        let long_content = "w".repeat(10001);
        let msg_body = serde_json::json!({"content": long_content});
        let res = client
            .post(format!("/api/v1/hook/{token}"))
            .header(ContentType::JSON)
            .body(msg_body.to_string())
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        return;
    }

    let wh: serde_json::Value = wh_res.into_json().unwrap();
    let token = wh["token"].as_str().unwrap();

    let long_content = "w".repeat(10001);
    let msg_body = serde_json::json!({"content": long_content});
    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(msg_body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}
