use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Reactions ---

#[test]
fn test_add_reaction() {
    let client = test_client();
    let room: serde_json::Value = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "react-room-1"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let room_id = room["id"].as_str().unwrap();

    let msg: serde_json::Value = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "React to this!"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Add a reaction
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "emoji": "👍"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let reaction: serde_json::Value = res.into_json().unwrap();
    assert_eq!(reaction["sender"].as_str().unwrap(), "Forge");
    assert_eq!(reaction["emoji"].as_str().unwrap(), "👍");
    assert_eq!(reaction["message_id"].as_str().unwrap(), msg_id);
    assert_eq!(reaction["room_id"].as_str().unwrap(), room_id);
    assert!(!reaction["created_at"].as_str().unwrap().is_empty());
}

#[test]
fn test_get_reactions_grouped() {
    let client = test_client();
    let room: serde_json::Value = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "react-room-2"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let room_id = room["id"].as_str().unwrap();

    let msg: serde_json::Value = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Multiple reactions"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Add multiple reactions
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "emoji": "👍"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "emoji": "👍"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Lux", "emoji": "❤️"}"#)
        .dispatch();

    // Get reactions
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message_id"].as_str().unwrap(), msg_id);
    let reactions = body["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 2);

    // 👍 should have 2 senders
    let thumbs = reactions.iter().find(|r| r["emoji"] == "👍").unwrap();
    assert_eq!(thumbs["count"].as_i64().unwrap(), 2);
    let senders: Vec<&str> = thumbs["senders"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
    assert!(senders.contains(&"Forge"));
    assert!(senders.contains(&"Drift"));

    // ❤️ should have 1 sender
    let heart = reactions.iter().find(|r| r["emoji"] == "❤️").unwrap();
    assert_eq!(heart["count"].as_i64().unwrap(), 1);
}

#[test]
fn test_reaction_toggle() {
    let client = test_client();
    let room: serde_json::Value = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "react-room-3"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let room_id = room["id"].as_str().unwrap();

    let msg: serde_json::Value = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Toggle test"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Add reaction
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "emoji": "🎉"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let r1: serde_json::Value = res.into_json().unwrap();
    assert!(!r1["created_at"].as_str().unwrap().is_empty()); // Has timestamp = was added

    // Toggle same reaction (should remove)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "emoji": "🎉"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let r2: serde_json::Value = res.into_json().unwrap();
    assert!(r2["created_at"].as_str().unwrap().is_empty()); // Empty timestamp = was removed

    // Verify no reactions remain
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["reactions"].as_array().unwrap().len(), 0);
}

#[test]
fn test_remove_reaction_via_delete() {
    let client = test_client();
    let room: serde_json::Value = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "react-room-4"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let room_id = room["id"].as_str().unwrap();

    let msg: serde_json::Value = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Delete reaction test"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Add reaction
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "emoji": "🔥"}"#)
        .dispatch();

    // Delete via DELETE endpoint
    let encoded_emoji = urlencoding::encode("🔥");
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions?sender=Drift&emoji={encoded_emoji}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["status"].as_str().unwrap(), "removed");

    // Verify gone
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["reactions"].as_array().unwrap().len(), 0);
}

#[test]
fn test_reaction_nonexistent_message() {
    let client = test_client();
    let room: serde_json::Value = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "react-room-5"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/fake-id/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "emoji": "👍"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_reaction_empty_sender() {
    let client = test_client();
    let room: serde_json::Value = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "react-room-6"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let room_id = room["id"].as_str().unwrap();

    let msg: serde_json::Value = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Test"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "", "emoji": "👍"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_reactions_cascade_on_message_delete() {
    let client = test_client();
    let room: serde_json::Value = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "react-room-7"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let room_id = room["id"].as_str().unwrap();

    let msg: serde_json::Value = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Will be deleted"}"#)
        .dispatch()
        .into_json()
        .unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Add reactions
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "emoji": "👍"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "emoji": "❤️"}"#)
        .dispatch();

    // Delete the message
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}?sender=Nanook"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Reactions should be gone (CASCADE)
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound); // Message doesn't exist anymore
}

#[test]
fn test_get_room_reactions_bulk() {
    let client = test_client();

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "bulk-reactions-test", "created_by": "Nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Create two messages
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Message 1"}"#)
        .dispatch();
    let msg1: serde_json::Value = res.into_json().unwrap();
    let msg1_id = msg1["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Message 2"}"#)
        .dispatch();
    let msg2: serde_json::Value = res.into_json().unwrap();
    let msg2_id = msg2["id"].as_str().unwrap();

    // Add reactions to msg1
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg1_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "emoji": "👍"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg1_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "emoji": "👍"}"#)
        .dispatch();

    // Add reactions to msg2
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg2_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "emoji": "❤️"}"#)
        .dispatch();

    // Fetch bulk room reactions
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/reactions"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    assert_eq!(body["room_id"].as_str().unwrap(), room_id);
    let reactions = body["reactions"].as_object().unwrap();

    // msg1 should have 👍 with 2 senders
    let msg1_reactions = reactions[msg1_id].as_array().unwrap();
    assert_eq!(msg1_reactions.len(), 1);
    assert_eq!(msg1_reactions[0]["emoji"], "👍");
    assert_eq!(msg1_reactions[0]["count"], 2);
    let senders: Vec<&str> = msg1_reactions[0]["senders"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
    assert!(senders.contains(&"Forge"));
    assert!(senders.contains(&"Drift"));

    // msg2 should have ❤️ with 1 sender
    let msg2_reactions = reactions[msg2_id].as_array().unwrap();
    assert_eq!(msg2_reactions.len(), 1);
    assert_eq!(msg2_reactions[0]["emoji"], "❤️");
    assert_eq!(msg2_reactions[0]["count"], 1);
    assert_eq!(msg2_reactions[0]["senders"][0], "Nanook");
}

#[test]
fn test_get_room_reactions_empty() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "empty-reactions-test", "created_by": "Nanook"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/reactions"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["reactions"].as_object().unwrap().len(), 0);
}

#[test]
fn test_get_room_reactions_nonexistent_room() {
    let client = test_client();

    let res = client
        .get("/api/v1/rooms/nonexistent-room-id/reactions")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_room_last_message_preview() {
    let client = test_client();

    // Create room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "preview-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Empty room — no preview fields
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body.get("last_message_sender").is_none() || body["last_message_sender"].is_null());
    assert!(body.get("last_message_preview").is_none() || body["last_message_preview"].is_null());

    // Send a message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Alice", "content": "Hello world"}"#)
        .dispatch();

    // Send another message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Bob", "content": "Goodbye world"}"#)
        .dispatch();

    // Room detail should show last message from Bob
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["last_message_sender"], "Bob");
    assert_eq!(body["last_message_preview"], "Goodbye world");

    // Room list should also include preview
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let room = rooms.iter().find(|r| r["id"].as_str() == Some(room_id)).unwrap();
    assert_eq!(room["last_message_sender"], "Bob");
    assert_eq!(room["last_message_preview"], "Goodbye world");
}

#[test]
fn test_room_last_message_preview_truncation() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "truncate-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a very long message (200 chars)
    let long_content = "A".repeat(200);
    let body_json = serde_json::json!({"sender": "Verbose", "content": long_content});
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(body_json.to_string())
        .dispatch();

    // Preview should be truncated to 100 chars
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let preview = body["last_message_preview"].as_str().unwrap();
    assert_eq!(preview.len(), 100);
    assert_eq!(body["last_message_sender"], "Verbose");
}

#[test]
fn test_rooms_sorted_by_last_activity() {
    let client = test_client();

    // Create three rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "alpha-room"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "beta-room"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "gamma-room"}"#)
        .dispatch();
    let room_c: serde_json::Value = res.into_json().unwrap();
    let room_c_id = room_c["id"].as_str().unwrap();

    // Send messages in order: alpha first, then gamma, then beta (most recent)
    client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "test", "content": "first"}"#)
        .dispatch();

    // Small delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(50));

    client
        .post(format!("/api/v1/rooms/{room_c_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "test", "content": "second"}"#)
        .dispatch();

    std::thread::sleep(std::time::Duration::from_millis(50));

    client
        .post(format!("/api/v1/rooms/{room_b_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "test", "content": "third"}"#)
        .dispatch();

    // Fetch room list — should be ordered by last activity DESC
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();

    // beta (most recent) first, then gamma, then alpha, then #general (no messages) last
    assert!(rooms.len() >= 4); // 3 created + #general
    assert_eq!(rooms[0]["name"], "beta-room");
    assert_eq!(rooms[1]["name"], "gamma-room");
    assert_eq!(rooms[2]["name"], "alpha-room");
    // #general has no messages, should be last
    assert_eq!(rooms[3]["name"], "general");
}
