use rocket::http::{ContentType, Status};
use local_agent_chat::message_config::MessageConfig;
use crate::common::{test_client_with_message_config};

// Helper: send N messages to a room
fn send_messages(client: &rocket::local::blocking::Client, room_id: &str, count: usize) {
    for i in 0..count {
        let body = format!(r#"{{"sender": "agent", "content": "msg {i}"}}"#);
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(body)
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
    }
}

// Helper: create a room and return room_id
fn create_room(client: &rocket::local::blocking::Client, name: &str) -> String {
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(format!(r#"{{"name": "{name}"}}"#))
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    room["id"].as_str().unwrap().to_string()
}

/// Default API limit is applied when no ?limit param is given.
#[test]
fn test_default_api_limit_applied() {
    let config = MessageConfig { default_limit_api: 5, default_limit_ui: 200, max_limit: 500 };
    let client = test_client_with_message_config(config);
    let room_id = create_room(&client, "window-default-api");
    send_messages(&client, &room_id, 10);

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 5, "default_limit_api should cap at 5");
}

/// Explicit ?limit=N below the default still works.
#[test]
fn test_explicit_limit_below_default() {
    let config = MessageConfig { default_limit_api: 50, default_limit_ui: 200, max_limit: 500 };
    let client = test_client_with_message_config(config);
    let room_id = create_room(&client, "window-explicit-low");
    send_messages(&client, &room_id, 20);

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?limit=3"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 3);
}

/// ?limit=N above the hard cap is clamped to max_limit.
#[test]
fn test_limit_clamped_to_max() {
    let config = MessageConfig { default_limit_api: 50, default_limit_ui: 200, max_limit: 10 };
    let client = test_client_with_message_config(config);
    let room_id = create_room(&client, "window-max-cap");
    send_messages(&client, &room_id, 20);

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?limit=999"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 10, "hard cap should limit to max_limit=10");
}

/// Full history is preserved — older messages accessible via before_seq pagination.
#[test]
fn test_full_history_preserved() {
    let config = MessageConfig { default_limit_api: 3, default_limit_ui: 200, max_limit: 500 };
    let client = test_client_with_message_config(config);
    let room_id = create_room(&client, "window-history");
    send_messages(&client, &room_id, 6);

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let recent: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(recent.len(), 3);

    let earliest_seq = recent[0]["seq"].as_i64().unwrap();
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?before_seq={earliest_seq}&limit=10"))
        .dispatch();
    let older: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(older.len(), 3, "older messages should still be in DB");
}

/// Default config uses production defaults: 50/200/500.
#[test]
fn test_default_config_values() {
    let config = MessageConfig::default();
    assert_eq!(config.default_limit_api, 50);
    assert_eq!(config.default_limit_ui, 200);
    assert_eq!(config.max_limit, 500);
}

/// resolve() applies default when None supplied.
#[test]
fn test_resolve_applies_default() {
    let config = MessageConfig { default_limit_api: 42, default_limit_ui: 200, max_limit: 500 };
    assert_eq!(config.resolve(None), 42);
}

/// resolve() clamps to max_limit.
#[test]
fn test_resolve_clamps_to_max() {
    let config = MessageConfig { default_limit_api: 50, default_limit_ui: 200, max_limit: 100 };
    assert_eq!(config.resolve(Some(999)), 100);
    assert_eq!(config.resolve(Some(100)), 100);
    assert_eq!(config.resolve(Some(50)), 50);
}

/// resolve() clamps to minimum of 1.
#[test]
fn test_resolve_clamps_to_min() {
    let config = MessageConfig::default();
    assert_eq!(config.resolve(Some(0)), 1);
    assert_eq!(config.resolve(Some(-5)), 1);
}

/// ?latest=N is also bounded by max_limit.
#[test]
fn test_latest_param_respects_max_limit() {
    let config = MessageConfig { default_limit_api: 50, default_limit_ui: 200, max_limit: 5 };
    let client = test_client_with_message_config(config);
    let room_id = create_room(&client, "window-latest");
    send_messages(&client, &room_id, 10);

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?latest=100"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 5, "?latest should be capped by max_limit");
}

/// Activity feed default limit is governed by MessageConfig.
#[test]
fn test_activity_feed_default_limit() {
    let config = MessageConfig { default_limit_api: 4, default_limit_ui: 200, max_limit: 500 };
    let client = test_client_with_message_config(config);
    let room_id = create_room(&client, "window-activity");
    send_messages(&client, &room_id, 8);

    let res = client.get("/api/v1/activity").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let items = body["events"].as_array().unwrap();
    assert_eq!(items.len(), 4, "activity feed should use default_limit_api");
}
