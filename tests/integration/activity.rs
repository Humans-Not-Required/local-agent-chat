use rocket::http::{ContentType, Status};
use crate::common::test_client;

// Helper: create a room and return room_id
fn create_room(client: &impl std::ops::Deref<Target = rocket::local::blocking::Client>, name: &str) -> String {
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(format!(r#"{{"name": "{name}", "created_by": "tester"}}"#))
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    room["id"].as_str().unwrap().to_string()
}

// Helper: send a message and return message object
fn send_msg(
    client: &impl std::ops::Deref<Target = rocket::local::blocking::Client>,
    room_id: &str,
    sender: &str,
    content: &str,
    sender_type: Option<&str>,
) -> serde_json::Value {
    let st = sender_type.unwrap_or("agent");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "{sender}", "content": "{content}", "sender_type": "{st}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    res.into_json().unwrap()
}

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

// --- Activity Feed: exclude_sender ---

#[test]
fn test_activity_feed_exclude_sender_single() {
    let client = test_client();
    let room_id = create_room(&client, "activity-exclude");

    send_msg(&client, &room_id, "Nanook", "message from nanook", None);
    send_msg(&client, &room_id, "Forge", "message from forge", None);
    send_msg(&client, &room_id, "Drift", "message from drift", None);

    // Exclude Nanook — should get only Forge and Drift
    let res = client
        .get("/api/v1/activity?exclude_sender=Nanook")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);
    let senders: Vec<&str> = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["sender"].as_str().unwrap())
        .collect();
    assert!(!senders.contains(&"Nanook"));
    assert!(senders.contains(&"Forge"));
    assert!(senders.contains(&"Drift"));
}

#[test]
fn test_activity_feed_exclude_sender_multiple() {
    let client = test_client();
    let room_id = create_room(&client, "activity-exclude-multi");

    send_msg(&client, &room_id, "Nanook", "msg from nanook", None);
    send_msg(&client, &room_id, "Forge", "msg from forge", None);
    send_msg(&client, &room_id, "Drift", "msg from drift", None);
    send_msg(&client, &room_id, "Lux", "msg from lux", None);

    // Exclude multiple senders (comma-separated)
    let res = client
        .get("/api/v1/activity?exclude_sender=Nanook,Forge")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);
    let senders: Vec<&str> = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["sender"].as_str().unwrap())
        .collect();
    assert!(!senders.contains(&"Nanook"));
    assert!(!senders.contains(&"Forge"));
    assert!(senders.contains(&"Drift"));
    assert!(senders.contains(&"Lux"));
}

#[test]
fn test_activity_feed_exclude_all_senders() {
    let client = test_client();
    let room_id = create_room(&client, "activity-exclude-all");

    send_msg(&client, &room_id, "Nanook", "only me here", None);

    // Exclude the only sender — should get 0 results
    let res = client
        .get("/api/v1/activity?exclude_sender=Nanook")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 0);
}

// --- Activity Feed: after (seq cursor) ---

#[test]
fn test_activity_feed_after_cursor() {
    let client = test_client();
    let room_id = create_room(&client, "activity-cursor");

    let msg1 = send_msg(&client, &room_id, "Nanook", "first message", None);
    let msg2 = send_msg(&client, &room_id, "Forge", "second message", None);
    let _msg3 = send_msg(&client, &room_id, "Drift", "third message", None);

    let seq1 = msg1["seq"].as_i64().unwrap();
    let seq2 = msg2["seq"].as_i64().unwrap();

    // after=seq1 should return only messages with seq > seq1 (msg2 and msg3)
    let res = client
        .get(format!("/api/v1/activity?after={seq1}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);

    // after=seq2 should return only msg3
    let res = client
        .get(format!("/api/v1/activity?after={seq2}"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["events"][0]["sender"], "Drift");
}

#[test]
fn test_activity_feed_after_cursor_no_new_messages() {
    let client = test_client();
    let room_id = create_room(&client, "activity-cursor-empty");

    let msg = send_msg(&client, &room_id, "Nanook", "only message", None);
    let seq = msg["seq"].as_i64().unwrap();

    // after=last_seq should return 0 results (no messages after this one)
    let res = client
        .get(format!("/api/v1/activity?after={seq}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 0);
}

// --- Activity Feed: sender filter ---

#[test]
fn test_activity_feed_sender_filter() {
    let client = test_client();
    let room_id = create_room(&client, "activity-sender");

    send_msg(&client, &room_id, "Nanook", "nanook msg 1", None);
    send_msg(&client, &room_id, "Forge", "forge msg", None);
    send_msg(&client, &room_id, "Nanook", "nanook msg 2", None);

    // Filter by sender=Nanook
    let res = client
        .get("/api/v1/activity?sender=Nanook")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);
    for event in body["events"].as_array().unwrap() {
        assert_eq!(event["sender"], "Nanook");
    }
}

// --- Activity Feed: combined filters ---

#[test]
fn test_activity_feed_exclude_sender_with_room_filter() {
    let client = test_client();
    let room_a = create_room(&client, "activity-combo-a");
    let room_b = create_room(&client, "activity-combo-b");

    send_msg(&client, &room_a, "Nanook", "nanook in room a", None);
    send_msg(&client, &room_a, "Forge", "forge in room a", None);
    send_msg(&client, &room_b, "Nanook", "nanook in room b", None);
    send_msg(&client, &room_b, "Drift", "drift in room b", None);

    // Room A, excluding Nanook
    let res = client
        .get(format!(
            "/api/v1/activity?room_id={room_a}&exclude_sender=Nanook"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["events"][0]["sender"], "Forge");
    assert_eq!(body["events"][0]["room_name"], "activity-combo-a");
}

#[test]
fn test_activity_feed_seq_fields_present() {
    let client = test_client();
    let room_id = create_room(&client, "activity-seq");

    send_msg(&client, &room_id, "Nanook", "check seq field", None);

    let res = client.get("/api/v1/activity").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);

    let event = &body["events"][0];
    assert!(event["seq"].as_i64().is_some(), "Activity events should include seq for cursor pagination");
    assert!(event["room_id"].as_str().is_some());
    assert!(event["room_name"].as_str().is_some());
    assert!(event["created_at"].as_str().is_some());
    assert_eq!(event["event_type"], "message");
}
