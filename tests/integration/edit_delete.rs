use rocket::http::{ContentType, Header, Status};
use crate::common::test_client;

// --- Edit Messages ---

#[test]
fn test_edit_message() {
    let client = test_client();

    // Get general room
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"TestBot","content":"Original content"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();
    assert!(msg.get("edited_at").is_none() || msg["edited_at"].is_null());

    // Edit the message
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"TestBot","content":"Edited content"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let edited: serde_json::Value = res.into_json().unwrap();
    assert_eq!(edited["content"], "Edited content");
    assert!(edited["edited_at"].is_string());
    assert_eq!(edited["sender"], "TestBot");

    // Verify the edit persisted
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    let found = msgs.iter().find(|m| m["id"] == msg_id).unwrap();
    assert_eq!(found["content"], "Edited content");
    assert!(found["edited_at"].is_string());
}

#[test]
fn test_edit_message_wrong_sender() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send a message as "BotA"
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"BotA","content":"Hello"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Try to edit as "BotB" — should fail
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"BotB","content":"Hijacked!"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_edit_message_not_found() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/nonexistent"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Edit ghost"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_edit_message_empty_content() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Hello"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

// --- Delete Messages ---

#[test]
fn test_delete_message_by_sender() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Delete me"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Delete as sender
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}?sender=Bot"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["deleted"], true);

    // Verify it's gone
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(msgs.iter().all(|m| m["id"] != msg_id));
}

#[test]
fn test_delete_message_wrong_sender() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"BotA","content":"My message"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // BotB tries to delete BotA's message
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}?sender=BotB"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_delete_message_no_sender() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Msg"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Delete without sender or admin key — should fail
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_delete_message_admin_override() {
    let client = test_client();

    // Create a room so we get the admin_key
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "admin-msg-del-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Admin will delete me"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Room admin deletes without matching sender
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
}

#[test]
fn test_delete_message_wrong_admin_key() {
    let client = test_client();

    // Create a room so we get the admin_key
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "wrong-admin-msg-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Should not be deleted"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Wrong admin key — should fall back to sender check, which also fails (no sender param)
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_create_room_returns_admin_key() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "key-test"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let key = body["admin_key"].as_str().unwrap();
    assert!(
        key.starts_with("chat_"),
        "admin_key should start with 'chat_'"
    );
    assert!(key.len() > 10, "admin_key should be sufficiently long");
}

#[test]
fn test_admin_key_not_in_room_list() {
    let client = test_client();
    // Create a room
    client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "no-leak-test"}"#)
        .dispatch();

    // List rooms — admin_key should NOT be present
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    for room in &rooms {
        assert!(
            room.get("admin_key").is_none(),
            "admin_key should not be in room list"
        );
    }

    // Get single room — admin_key should NOT be present
    let room_id = rooms.iter().find(|r| r["name"] == "no-leak-test").unwrap()["id"]
        .as_str()
        .unwrap();
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    assert!(
        room.get("admin_key").is_none(),
        "admin_key should not be in room detail"
    );
}

#[test]
fn test_delete_message_not_found() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/nonexistent?sender=Bot"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

// --- Edit preserves reply_to ---

#[test]
fn test_edit_message_preserves_reply_to() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "edit-reply-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send original message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Original"}"#)
        .dispatch();
    let original: serde_json::Value = res.into_json().unwrap();
    let original_id = original["id"].as_str().unwrap();

    // Send reply
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "Reply text", "reply_to": "{original_id}"}}"#
        ))
        .dispatch();
    let reply: serde_json::Value = res.into_json().unwrap();
    let reply_id = reply["id"].as_str().unwrap();
    assert_eq!(reply["reply_to"], original_id);

    // Edit the reply
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{reply_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "content": "Edited reply"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let edited: serde_json::Value = res.into_json().unwrap();
    assert_eq!(edited["content"], "Edited reply");
    assert_eq!(edited["reply_to"], original_id); // reply_to preserved
    assert!(edited["edited_at"].as_str().is_some()); // has edited_at
}
