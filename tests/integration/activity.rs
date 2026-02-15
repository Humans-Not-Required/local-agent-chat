use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Activity Feed ---

#[test]
fn test_activity_feed_basic() {
    let client = test_client();

    // Get the general room ID
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send messages in general
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Hello from general", "sender_type": "agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "content": "Hi from general too", "sender_type": "human"}"#)
        .dispatch();

    // Activity feed should show both (newest first)
    let res = client.get("/api/v1/activity").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);
    let events = body["events"].as_array().unwrap();
    // Newest first
    assert_eq!(events[0]["sender"], "bob");
    assert_eq!(events[0]["room_name"], "general");
    assert_eq!(events[0]["event_type"], "message");
    assert_eq!(events[1]["sender"], "alice");
}

#[test]
fn test_activity_feed_cross_room() {
    let client = test_client();

    // Get general room ID
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a second room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "dev"}"#)
        .dispatch();
    let dev_room: serde_json::Value = res.into_json().unwrap();
    let dev_id = dev_room["id"].as_str().unwrap();

    // Send message in each room
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Hello in general"}"#)
        .dispatch();

    // Tiny delay to ensure timestamp ordering
    std::thread::sleep(std::time::Duration::from_millis(10));

    client
        .post(format!("/api/v1/rooms/{dev_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "content": "Hello in dev"}"#)
        .dispatch();

    // Activity feed shows both rooms
    let res = client.get("/api/v1/activity").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);
    let events = body["events"].as_array().unwrap();
    // Newest first: dev room, then general
    assert_eq!(events[0]["room_name"], "dev");
    assert_eq!(events[1]["room_name"], "general");
}

#[test]
fn test_activity_feed_since_filter() {
    let client = test_client();

    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send first message
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Old message"}"#)
        .dispatch();

    // Capture timestamp after first message
    std::thread::sleep(std::time::Duration::from_millis(50));
    let since = chrono::Utc::now().to_rfc3339();
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Send second message
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "content": "New message"}"#)
        .dispatch();

    // Activity with since should only show the new message
    // URL-encode the + in RFC3339 timestamps
    let encoded_since = since.replace('+', "%2B");
    let res = client
        .get(format!("/api/v1/activity?since={encoded_since}"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["events"][0]["sender"], "bob");
    assert!(body["since"].is_string());
}

#[test]
fn test_activity_feed_room_filter() {
    let client = test_client();

    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create second room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "secret"}"#)
        .dispatch();
    let secret_room: serde_json::Value = res.into_json().unwrap();
    let secret_id = secret_room["id"].as_str().unwrap();

    // Send messages in both rooms
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "In general"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{secret_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "content": "In secret"}"#)
        .dispatch();

    // Filter to secret room only
    let res = client
        .get(format!("/api/v1/activity?room_id={secret_id}"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["events"][0]["sender"], "bob");
    assert_eq!(body["events"][0]["room_name"], "secret");
}

#[test]
fn test_activity_feed_sender_type_filter() {
    let client = test_client();

    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send agent and human messages
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook", "content": "Agent here", "sender_type": "agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "jordan", "content": "Human here", "sender_type": "human"}"#)
        .dispatch();

    // Filter to agents only
    let res = client.get("/api/v1/activity?sender_type=agent").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["events"][0]["sender"], "nanook");
    assert_eq!(body["events"][0]["sender_type"], "agent");
}

#[test]
fn test_activity_feed_limit() {
    let client = test_client();

    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send 5 messages
    for i in 0..5 {
        client
            .post(format!("/api/v1/rooms/{general_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
    }

    // Limit to 2
    let res = client.get("/api/v1/activity?limit=2").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);
}

#[test]
fn test_activity_feed_empty() {
    let client = test_client();

    // No messages sent — should get empty activity
    let res = client.get("/api/v1/activity").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 0);
    assert!(body["events"].as_array().unwrap().is_empty());
}
