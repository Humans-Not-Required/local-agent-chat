use rocket::http::{ContentType, Header, Status};
use local_agent_chat::rate_limit::RateLimitConfig;
use crate::common::test_client_with_rate_limits;

// --- Configurable Rate Limits ---

#[test]
fn test_custom_message_rate_limit() {
    let mut config = RateLimitConfig::default();
    config.messages_max = 3; // Very low limit for testing
    let client = test_client_with_rate_limits(config);

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "custom-rl-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 3 messages (should all succeed)
    for i in 0..3 {
        let body = format!(r#"{{"sender": "agent", "content": "msg {i}"}}"#);
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(body)
            .dispatch();
        assert_eq!(res.status(), Status::Ok, "Message {i} should succeed");
    }

    // The 4th should be rate limited (custom limit of 3)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent", "content": "too many"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::TooManyRequests);

    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("3"));
    assert_eq!(body["limit"], 3);
}

#[test]
fn test_custom_message_rate_limit_headers() {
    let mut config = RateLimitConfig::default();
    config.messages_max = 5;
    let client = test_client_with_rate_limits(config);

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "custom-rl-headers"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send first message and check headers reflect custom limit
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent", "content": "hello"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let limit = res.headers().get_one("X-RateLimit-Limit").unwrap();
    let remaining = res.headers().get_one("X-RateLimit-Remaining").unwrap();
    assert_eq!(limit, "5", "Custom message limit should be 5");
    assert_eq!(remaining, "4", "Should have 4 remaining after first message");
}

#[test]
fn test_custom_room_rate_limit() {
    let mut config = RateLimitConfig::default();
    config.rooms_max = 2;
    let client = test_client_with_rate_limits(config);

    // Create 2 rooms (should succeed)
    for i in 0..2 {
        let body = format!(r#"{{"name": "room-{i}"}}"#);
        let res = client
            .post("/api/v1/rooms")
            .header(ContentType::JSON)
            .body(body)
            .dispatch();
        assert_eq!(res.status(), Status::Ok, "Room {i} should succeed");
    }

    // The 3rd should be rate limited
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "too-many-rooms"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::TooManyRequests);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["limit"], 2);
}

#[test]
fn test_custom_file_rate_limit() {
    use base64::Engine;
    let mut config = RateLimitConfig::default();
    config.files_max = 2;
    let client = test_client_with_rate_limits(config);

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "file-rl-custom"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let data = base64::engine::general_purpose::STANDARD.encode(b"test");

    // Upload 2 files (should succeed)
    for i in 0..2 {
        let body = serde_json::json!({
            "sender": "agent",
            "filename": format!("file-{i}.txt"),
            "data": data,
            "content_type": "text/plain"
        });
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/files"))
            .header(ContentType::JSON)
            .body(body.to_string())
            .dispatch();
        assert_eq!(res.status(), Status::Ok, "File {i} should succeed");

        let limit = res.headers().get_one("X-RateLimit-Limit").unwrap();
        assert_eq!(limit, "2", "Custom file limit should be 2");
    }

    // The 3rd should be rate limited
    let body = serde_json::json!({
        "sender": "agent",
        "filename": "too-many.txt",
        "data": data,
        "content_type": "text/plain"
    });
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::TooManyRequests);
}

#[test]
fn test_custom_dm_rate_limit() {
    let mut config = RateLimitConfig::default();
    config.dms_max = 2;
    let client = test_client_with_rate_limits(config);

    // Send 2 DMs (should succeed)
    for i in 0..2 {
        let body = serde_json::json!({
            "sender": "alice",
            "recipient": "bob",
            "content": format!("dm {i}")
        });
        let res = client
            .post("/api/v1/dm")
            .header(ContentType::JSON)
            .body(body.to_string())
            .dispatch();
        assert_eq!(res.status(), Status::Ok, "DM {i} should succeed");

        let limit = res.headers().get_one("X-RateLimit-Limit").unwrap();
        assert_eq!(limit, "2", "Custom DM limit should be 2");
    }

    // The 3rd should be rate limited
    let body = serde_json::json!({
        "sender": "alice",
        "recipient": "bob",
        "content": "too many"
    });
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::TooManyRequests);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["limit"], 2);
}

#[test]
fn test_custom_webhook_rate_limit() {
    let mut config = RateLimitConfig::default();
    config.webhooks_max = 2;
    let client = test_client_with_rate_limits(config);

    // Create a room and incoming webhook
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "wh-rl-custom"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let wh_res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("X-Admin-Key", admin_key.to_string()))
        .body(r#"{"name": "Custom RL Hook", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(wh_res.status(), Status::Ok);
    let wh: serde_json::Value = wh_res.into_json().unwrap();
    let token = wh["token"].as_str().unwrap();

    // Post 2 messages via webhook (should succeed)
    for i in 0..2 {
        let body = serde_json::json!({"content": format!("hook msg {i}")});
        let res = client
            .post(format!("/api/v1/hook/{token}"))
            .header(ContentType::JSON)
            .body(body.to_string())
            .dispatch();
        assert_eq!(res.status(), Status::Ok, "Hook message {i} should succeed");

        let limit = res.headers().get_one("X-RateLimit-Limit").unwrap();
        assert_eq!(limit, "2", "Custom webhook limit should be 2");
    }

    // The 3rd should be rate limited
    let body = serde_json::json!({"content": "too many"});
    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::TooManyRequests);
}

#[test]
fn test_default_rate_limits_unchanged() {
    // Verify that default RateLimitConfig matches the original hardcoded values
    let config = RateLimitConfig::default();
    assert_eq!(config.messages_max, 60);
    assert_eq!(config.messages_window_secs, 60);
    assert_eq!(config.rooms_max, 10);
    assert_eq!(config.rooms_window_secs, 3600);
    assert_eq!(config.files_max, 10);
    assert_eq!(config.files_window_secs, 60);
    assert_eq!(config.dms_max, 60);
    assert_eq!(config.dms_window_secs, 60);
    assert_eq!(config.webhooks_max, 60);
    assert_eq!(config.webhooks_window_secs, 60);
}
