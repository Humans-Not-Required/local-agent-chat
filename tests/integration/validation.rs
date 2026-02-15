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
