use rocket::http::{ContentType, Header, Status};
use rocket::local::blocking::Client;

fn test_client() -> Client {
    // Use unique temp DB for each test (avoids parallel test contention)
    let db_path = format!(
        "/tmp/chat_test_{}.db",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    let rocket = local_agent_chat::rocket_with_db(&db_path);
    Client::tracked(rocket).expect("valid rocket instance")
}

// --- Health ---

#[test]
fn test_health() {
    let client = test_client();
    let res = client.get("/api/v1/health").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "local-agent-chat");
}

// --- Stats ---

#[test]
fn test_stats() {
    let client = test_client();
    let res = client.get("/api/v1/stats").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["rooms"].as_i64().unwrap() >= 1); // general room
    assert!(body["messages"].as_i64().unwrap() >= 0);
}

// --- Rooms ---

#[test]
fn test_default_general_room() {
    let client = test_client();
    let res = client.get("/api/v1/rooms").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(rooms.iter().any(|r| r["name"] == "general"));
}

#[test]
fn test_create_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "test-room", "description": "A test room", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["name"], "test-room");
    assert_eq!(body["created_by"], "tester");
    assert!(!body["id"].as_str().unwrap().is_empty());
}

#[test]
fn test_create_duplicate_room() {
    let client = test_client();
    client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "dup-room"}"#)
        .dispatch();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "dup-room"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Conflict);
}

#[test]
fn test_create_room_empty_name() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": ""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_get_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "get-test"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let id = body["id"].as_str().unwrap();

    let res = client.get(format!("/api/v1/rooms/{id}")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["name"], "get-test");
    assert_eq!(body["message_count"], 0);
}

#[test]
fn test_get_room_not_found() {
    let client = test_client();
    let res = client.get("/api/v1/rooms/nonexistent-id").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_room_no_auth() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "del-test"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let id = body["id"].as_str().unwrap();

    // Without admin key → should forward (401-like)
    let res = client.delete(format!("/api/v1/rooms/{id}")).dispatch();
    assert_ne!(res.status(), Status::Ok);
}

#[test]
fn test_delete_room_with_admin() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "admin-del-test"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let id = body["id"].as_str().unwrap();
    let admin_key = body["admin_key"].as_str().unwrap();

    let res = client
        .delete(format!("/api/v1/rooms/{id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify deleted
    let res = client.get(format!("/api/v1/rooms/{id}")).dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_room_wrong_admin_key() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "wrong-key-test"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let id = body["id"].as_str().unwrap();

    // Try deleting with wrong key
    let res = client
        .delete(format!("/api/v1/rooms/{id}"))
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);

    // Room should still exist
    let res = client.get(format!("/api/v1/rooms/{id}")).dispatch();
    assert_eq!(res.status(), Status::Ok);
}

// --- Messages ---

#[test]
fn test_send_and_get_messages() {
    let client = test_client();

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "msg-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook", "content": "Hello world!"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["sender"], "nanook");
    assert_eq!(msg["content"], "Hello world!");
    assert_eq!(msg["room_id"], room_id);

    // Get messages
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "Hello world!");
}

#[test]
fn test_send_message_nonexistent_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms/fake-room/messages")
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook", "content": "Hello!"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_send_message_empty_content() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "empty-msg-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook", "content": ""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_send_message_empty_sender() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "empty-sender-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "", "content": "Hello!"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_messages_since_filter() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "since-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send first message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "a", "content": "First"}"#)
        .dispatch();

    // Record timestamp
    let ts = chrono::Utc::now().to_rfc3339();

    // Small delay to ensure ordering
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Send second message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "b", "content": "Second"}"#)
        .dispatch();

    // Get messages since timestamp
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?since={ts}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "Second");
}

#[test]
fn test_messages_sender_filter() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "sender-filter-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "From Alice"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "content": "From Bob"}"#)
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?sender=alice"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["sender"], "alice");
}

#[test]
fn test_messages_limit() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "limit-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    for i in 0..5 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
    }

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?limit=2"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
}

#[test]
fn test_message_with_metadata() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "meta-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook", "content": "Hello!", "metadata": {"type": "greeting", "priority": 1}}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["metadata"]["type"], "greeting");
    assert_eq!(msg["metadata"]["priority"], 1);
}

#[test]
fn test_room_message_count_updates() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "count-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Initially 0
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message_count"], 0);

    // Send 3 messages
    for _ in 0..3 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(r#"{"sender": "bot", "content": "test"}"#)
            .dispatch();
    }

    // Now 3
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message_count"], 3);
}

#[test]
fn test_delete_room_cascades_messages() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "cascade-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Send a message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "test"}"#)
        .dispatch();

    // Delete room with proper admin key
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Messages endpoint should 404
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

// --- llms.txt ---

#[test]
fn test_llms_txt_root() {
    let client = test_client();
    let res = client.get("/llms.txt").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body = res.into_string().unwrap();
    assert!(body.contains("Local Agent Chat"));
}

#[test]
fn test_llms_txt_api() {
    let client = test_client();
    let res = client.get("/api/v1/llms.txt").dispatch();
    assert_eq!(res.status(), Status::Ok);
}

// --- OpenAPI ---

#[test]
fn test_openapi_json() {
    let client = test_client();
    let res = client.get("/api/v1/openapi.json").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["openapi"], "3.0.3");
    assert_eq!(body["info"]["title"], "Local Agent Chat API");
}

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

// --- Messages in nonexistent room ---

#[test]
fn test_get_messages_nonexistent_room() {
    let client = test_client();
    let res = client.get("/api/v1/rooms/nonexistent/messages").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

// --- Message Threading (reply_to) ---

#[test]
fn test_reply_to_message() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send original message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Alice","content":"Hello everyone!"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let original: serde_json::Value = res.into_json().unwrap();
    let original_id = original["id"].as_str().unwrap();
    assert!(original.get("reply_to").is_none() || original["reply_to"].is_null());

    // Send a reply
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender":"Bob","content":"Hey Alice!","reply_to":"{original_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let reply: serde_json::Value = res.into_json().unwrap();
    assert_eq!(reply["reply_to"], original_id);
    assert_eq!(reply["sender"], "Bob");
    assert_eq!(reply["content"], "Hey Alice!");

    // Verify reply_to persists in message list
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    let reply_msg = msgs.iter().find(|m| m["sender"] == "Bob").unwrap();
    assert_eq!(reply_msg["reply_to"], original_id);
}

#[test]
fn test_reply_to_nonexistent_message() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Try to reply to a message that doesn't exist
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Replying to ghost","reply_to":"nonexistent-id"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("reply_to"));
}

#[test]
fn test_reply_to_message_in_different_room() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"room-a"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"room-b"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    // Send message in room A
    let res = client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Alice","content":"In room A"}"#)
        .dispatch();
    let msg_a: serde_json::Value = res.into_json().unwrap();
    let msg_a_id = msg_a["id"].as_str().unwrap();

    // Try to reply to room A's message from room B — should fail
    let res = client
        .post(format!("/api/v1/rooms/{room_b_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender":"Bob","content":"Cross-room reply","reply_to":"{msg_a_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_reply_to_null_is_optional() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send without reply_to (backwards compatible)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"No reply"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert!(msg.get("reply_to").is_none() || msg["reply_to"].is_null());

    // Send with explicit null reply_to
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Explicit null","reply_to":null}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert!(msg.get("reply_to").is_none() || msg["reply_to"].is_null());
}

// --- Typing Indicators ---

#[test]
fn test_typing_notification() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/typing"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["ok"], true);
}

#[test]
fn test_typing_nonexistent_room() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms/nonexistent-room-id/typing")
        .header(ContentType::JSON)
        .body(r#"{"sender":"Nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_typing_empty_sender() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/typing"))
        .header(ContentType::JSON)
        .body(r#"{"sender":""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_typing_dedup() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // First call should succeed
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/typing"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"SpamBot"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Second call within 2s should also return ok (deduped silently)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/typing"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"SpamBot"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["ok"], true);
}

#[test]
fn test_sender_type_stored_in_message() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send with top-level sender_type
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"AgentBot","content":"I am an agent","sender_type":"agent"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["sender_type"], "agent");

    // Send with sender_type in metadata (backward compat)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"HumanUser","content":"I am human","metadata":{"sender_type":"human"}}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["sender_type"], "human");

    // Verify sender_type persists in GET
    let messages: Vec<serde_json::Value> = client
        .get(format!("/api/v1/rooms/{room_id}/messages?sender=AgentBot"))
        .dispatch()
        .into_json()
        .unwrap();
    assert!(!messages.is_empty());
    assert_eq!(messages.last().unwrap()["sender_type"], "agent");
}

#[test]
fn test_sender_type_top_level_overrides_metadata() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Top-level sender_type should override metadata.sender_type
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Confused","content":"Mixed signals","sender_type":"agent","metadata":{"sender_type":"human"}}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["sender_type"], "agent");
}

#[test]
fn test_sender_type_optional() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send without sender_type at all
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Anonymous","content":"No type specified"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert!(msg.get("sender_type").is_none() || msg["sender_type"].is_null());
}

// --- sender_type query filter ---

#[test]
fn test_messages_sender_type_filter() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send messages with different sender_types
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"AgentBot","content":"I am an agent","sender_type":"agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"HumanUser","content":"I am a human","sender_type":"human"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Unknown","content":"No type"}"#)
        .dispatch();

    // Filter by sender_type=agent
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?sender_type=agent"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["sender"], "AgentBot");
    assert_eq!(msgs[0]["sender_type"], "agent");

    // Filter by sender_type=human
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?sender_type=human"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["sender"], "HumanUser");
    assert_eq!(msgs[0]["sender_type"], "human");
}

#[test]
fn test_messages_sender_type_combined_with_sender_filter() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Two agents, one human
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot1","content":"Hello from Bot1","sender_type":"agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot2","content":"Hello from Bot2","sender_type":"agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot1","content":"Human message from Bot1","sender_type":"human"}"#)
        .dispatch();

    // Filter by sender=Bot1 AND sender_type=agent
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?sender=Bot1&sender_type=agent"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "Hello from Bot1");
}

// --- Enhanced stats ---

#[test]
fn test_stats_sender_type_breakdown() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send messages with different types
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Agent1","content":"agent msg 1","sender_type":"agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Agent2","content":"agent msg 2","sender_type":"agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Human1","content":"human msg","sender_type":"human"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Anon","content":"no type"}"#)
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    assert_eq!(body["by_sender_type"]["agent"].as_i64().unwrap(), 2);
    assert_eq!(body["by_sender_type"]["human"].as_i64().unwrap(), 1);
    assert_eq!(body["by_sender_type"]["unspecified"].as_i64().unwrap(), 1);
    assert!(body["active_by_type_1h"]["agents"].as_i64().unwrap() >= 2);
    assert!(body["active_by_type_1h"]["humans"].as_i64().unwrap() >= 1);
}

// --- Before filter ---

#[test]
fn test_messages_before_filter() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "before-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send first message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "a", "content": "First"}"#)
        .dispatch();

    // Small delay to ensure ordering
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Record timestamp between messages
    let ts = chrono::Utc::now().to_rfc3339();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // Send second message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "b", "content": "Second"}"#)
        .dispatch();

    // Get messages before timestamp — should only get the first
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?before={ts}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "First");
}

#[test]
fn test_messages_since_and_before_range() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "range-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send three messages with timestamps between them
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "a", "content": "First"}"#)
        .dispatch();
    std::thread::sleep(std::time::Duration::from_millis(10));

    let ts_start = chrono::Utc::now().to_rfc3339();
    std::thread::sleep(std::time::Duration::from_millis(10));

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "b", "content": "Middle"}"#)
        .dispatch();
    std::thread::sleep(std::time::Duration::from_millis(10));

    let ts_end = chrono::Utc::now().to_rfc3339();
    std::thread::sleep(std::time::Duration::from_millis(10));

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "c", "content": "Last"}"#)
        .dispatch();

    // Range query: should only get the middle message
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?since={ts_start}&before={ts_end}"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "Middle");
}

// --- Message ordering ---

#[test]
fn test_messages_returned_in_chronological_order() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "order-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send messages in order
    for i in 1..=5 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "Message {i}"}}"#))
            .dispatch();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 5);
    for i in 0..5 {
        assert_eq!(msgs[i]["content"], format!("Message {}", i + 1));
    }
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

// --- Stats update after deletions ---

#[test]
fn test_stats_update_after_message_deletion() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "stats-delete-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Send 3 messages
    for i in 1..=3 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "Msg {i}"}}"#))
            .dispatch();
    }

    // Verify initial count
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let room_detail: serde_json::Value = res.into_json().unwrap();
    assert_eq!(room_detail["message_count"].as_i64().unwrap(), 3);

    // Get message IDs
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    let msg_id = msgs[0]["id"].as_str().unwrap();

    // Delete one message using admin key
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}?sender=bot"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify count decreased
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
}

// --- Room with description ---

#[test]
fn test_create_room_with_description() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "described-room", "description": "A room for testing descriptions"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    assert_eq!(room["name"], "described-room");
    assert_eq!(room["description"], "A room for testing descriptions");

    // Verify it shows in room detail
    let room_id = room["id"].as_str().unwrap();
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let detail: serde_json::Value = res.into_json().unwrap();
    assert_eq!(detail["description"], "A room for testing descriptions");
}

// --- Room created_by ---

#[test]
fn test_create_room_with_created_by() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "owned-room", "created_by": "nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    assert_eq!(room["created_by"], "nanook");
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

// --- File Attachments ---

fn get_general_room_id(client: &Client) -> String {
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_upload_and_download_file() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let file_data = b"Hello, this is a test file!";
    let b64 = base64::engine::general_purpose::STANDARD.encode(file_data);

    // Upload
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "nanook",
                "filename": "test.txt",
                "content_type": "text/plain",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["filename"], "test.txt");
    assert_eq!(body["content_type"], "text/plain");
    assert_eq!(body["size"], file_data.len() as i64);
    assert_eq!(body["sender"], "nanook");
    assert_eq!(body["room_id"], room_id);
    let file_id = body["id"].as_str().unwrap();
    let url = body["url"].as_str().unwrap();
    assert_eq!(url, format!("/api/v1/files/{file_id}"));

    // Download
    let res = client.get(format!("/api/v1/files/{file_id}")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let bytes = res.into_bytes().unwrap();
    assert_eq!(bytes, file_data);
}

#[test]
fn test_file_info_endpoint() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let b64 = base64::engine::general_purpose::STANDARD.encode(b"info test");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "agent1",
                "filename": "data.json",
                "content_type": "application/json",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    let upload: serde_json::Value = res.into_json().unwrap();
    let file_id = upload["id"].as_str().unwrap();

    // Get file info
    let res = client
        .get(format!("/api/v1/files/{file_id}/info"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let info: serde_json::Value = res.into_json().unwrap();
    assert_eq!(info["filename"], "data.json");
    assert_eq!(info["sender"], "agent1");
    assert_eq!(info["content_type"], "application/json");
    assert_eq!(info["size"], 9); // "info test" = 9 bytes
}

#[test]
fn test_list_files_in_room() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    // Upload 2 files
    for name in &["file1.txt", "file2.txt"] {
        let b64 = base64::engine::general_purpose::STANDARD.encode(name.as_bytes());
        client
            .post(format!("/api/v1/rooms/{room_id}/files"))
            .header(ContentType::JSON)
            .body(
                serde_json::json!({
                    "sender": "uploader",
                    "filename": name,
                    "data": b64
                })
                .to_string(),
            )
            .dispatch();
    }

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/files"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let files: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn test_delete_file_by_sender() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let b64 = base64::engine::general_purpose::STANDARD.encode(b"delete me");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "owner",
                "filename": "temp.txt",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    let upload: serde_json::Value = res.into_json().unwrap();
    let file_id = upload["id"].as_str().unwrap();

    // Delete by correct sender
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/files/{file_id}?sender=owner"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify gone
    let res = client.get(format!("/api/v1/files/{file_id}")).dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_file_wrong_sender_forbidden() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let b64 = base64::engine::general_purpose::STANDARD.encode(b"protected");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "alice",
                "filename": "secret.txt",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    let upload: serde_json::Value = res.into_json().unwrap();
    let file_id = upload["id"].as_str().unwrap();

    // Wrong sender
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/files/{file_id}?sender=bob"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_delete_file_with_admin_key() {
    use base64::Engine;
    let client = test_client();

    // Create a room to get admin key
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "file-test-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Upload a file
    let b64 = base64::engine::general_purpose::STANDARD.encode(b"admin delete");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "someone",
                "filename": "moderated.txt",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    let upload: serde_json::Value = res.into_json().unwrap();
    let file_id = upload["id"].as_str().unwrap();

    // Delete with admin key (different sender)
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/files/{file_id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
}

#[test]
fn test_upload_file_invalid_base64() {
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "nanook",
                "filename": "bad.txt",
                "data": "not-valid-base64!!!"
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("base64"));
}

#[test]
fn test_upload_file_empty_sender() {
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "",
                "filename": "test.txt",
                "data": "aGVsbG8="
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_upload_file_nonexistent_room() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms/nonexistent-room-id/files")
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "nanook",
                "filename": "test.txt",
                "data": "aGVsbG8="
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_upload_file_too_large() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    // Create a 6MB payload (over 5MB limit)
    let big_data = vec![0u8; 6 * 1024 * 1024];
    let b64 = base64::engine::general_purpose::STANDARD.encode(&big_data);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "nanook",
                "filename": "huge.bin",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("too large"));
}

#[test]
fn test_list_files_nonexistent_room() {
    let client = test_client();
    let res = client.get("/api/v1/rooms/fake-room-id/files").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_download_nonexistent_file() {
    let client = test_client();
    let res = client.get("/api/v1/files/nonexistent-file-id").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

// --- Monotonic seq & cursor-based pagination ---

#[test]
fn test_messages_have_seq_field() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "seq-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message and verify seq in response
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "Hello"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert!(
        msg["seq"].is_number(),
        "Message should have numeric seq field"
    );
    assert!(msg["seq"].as_i64().unwrap() >= 1);

    // Verify seq appears in GET messages
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0]["seq"].is_number());
}

#[test]
fn test_seq_monotonically_increasing() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "seq-mono-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 5 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 0..5 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Verify strictly monotonically increasing
    for i in 1..seqs.len() {
        assert!(
            seqs[i] > seqs[i - 1],
            "seq should be strictly increasing: {} > {}",
            seqs[i],
            seqs[i - 1]
        );
    }
}

#[test]
fn test_after_cursor_pagination() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "after-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 3 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=3 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Use after= to get messages after the first one
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={}",
            seqs[0]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0]["content"], "msg 2");
    assert_eq!(msgs[1]["content"], "msg 3");

    // Use after= to get messages after the second one
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={}",
            seqs[1]
        ))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "msg 3");

    // Use after= with last seq — should get nothing
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={}",
            seqs[2]
        ))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 0);
}

#[test]
fn test_after_cursor_with_limit() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "after-limit-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 5 messages
    let mut first_seq: i64 = 0;
    for i in 1..=5 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        if i == 1 {
            first_seq = msg["seq"].as_i64().unwrap();
        }
    }

    // Get 2 messages after the first
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={first_seq}&limit=2"
        ))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0]["content"], "msg 2");
    assert_eq!(msgs[1]["content"], "msg 3");
}

#[test]
fn test_since_still_works_backward_compat() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "since-compat-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send first message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "a", "content": "Old"}"#)
        .dispatch();

    let ts = chrono::Utc::now().to_rfc3339();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Send second message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "b", "content": "New"}"#)
        .dispatch();

    // since= should still work and return seq in results
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?since={ts}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "New");
    assert!(msgs[0]["seq"].is_number());
}

#[test]
fn test_activity_feed_has_seq() {
    let client = test_client();

    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send a message
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "Activity seq test"}"#)
        .dispatch();

    // Activity feed events should have seq
    let res = client.get("/api/v1/activity").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["count"].as_i64().unwrap() >= 1);
    let events = body["events"].as_array().unwrap();
    assert!(
        events[0]["seq"].is_number(),
        "Activity events should have seq field"
    );
}

#[test]
fn test_activity_feed_after_cursor() {
    let client = test_client();

    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send 3 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=3 {
        let res = client
            .post(format!("/api/v1/rooms/{general_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender": "bot", "content": "activity msg {i}"}}"#
            ))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Use after= on activity feed — should only get messages after that seq
    let res = client
        .get(format!("/api/v1/activity?after={}", seqs[0]))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);
    let events = body["events"].as_array().unwrap();
    // Activity is newest-first, so msg 3 then msg 2
    assert_eq!(events[0]["content"], "activity msg 3");
    assert_eq!(events[1]["content"], "activity msg 2");
}

#[test]
fn test_seq_global_across_rooms() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "seq-room-a"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "seq-room-b"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    // Send messages alternating between rooms
    let res = client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "A1"}"#)
        .dispatch();
    let msg_a1: serde_json::Value = res.into_json().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_b_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "B1"}"#)
        .dispatch();
    let msg_b1: serde_json::Value = res.into_json().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "A2"}"#)
        .dispatch();
    let msg_a2: serde_json::Value = res.into_json().unwrap();

    // Seqs should be globally monotonic across rooms
    let seq_a1 = msg_a1["seq"].as_i64().unwrap();
    let seq_b1 = msg_b1["seq"].as_i64().unwrap();
    let seq_a2 = msg_a2["seq"].as_i64().unwrap();
    assert!(seq_b1 > seq_a1, "B1 seq should be > A1 seq");
    assert!(seq_a2 > seq_b1, "A2 seq should be > B1 seq");
}

#[test]
fn test_edit_preserves_seq() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "edit-seq-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "Original"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();
    let original_seq = msg["seq"].as_i64().unwrap();

    // Edit the message
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "Edited"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let edited: serde_json::Value = res.into_json().unwrap();

    // Seq should be preserved (not changed)
    assert_eq!(edited["seq"].as_i64().unwrap(), original_seq);
}

// --- Room Participants ---

#[test]
fn test_participants_empty_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "empty-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/participants"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(participants.len(), 0);
}

#[test]
fn test_participants_nonexistent_room() {
    let client = test_client();
    let res = client
        .get("/api/v1/rooms/nonexistent-uuid/participants")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_participants_basic() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "test-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send messages from different senders
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Alice", "content": "Hello", "sender_type": "human"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Bob", "content": "Hi", "sender_type": "agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Alice", "content": "How are you?"}"#)
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/participants"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(participants.len(), 2);

    // Sorted by last_seen DESC — Alice sent the last message so should be first
    assert_eq!(participants[0]["sender"].as_str().unwrap(), "Alice");
    assert_eq!(participants[0]["message_count"].as_i64().unwrap(), 2);
    assert_eq!(participants[0]["sender_type"].as_str().unwrap(), "human");
    assert!(participants[0]["first_seen"].is_string());
    assert!(participants[0]["last_seen"].is_string());

    assert_eq!(participants[1]["sender"].as_str().unwrap(), "Bob");
    assert_eq!(participants[1]["message_count"].as_i64().unwrap(), 1);
    assert_eq!(participants[1]["sender_type"].as_str().unwrap(), "agent");
}

#[test]
fn test_participants_sender_type_uses_latest() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "type-change-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // First message as agent
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Charlie", "content": "I'm an agent", "sender_type": "agent"}"#)
        .dispatch();

    // Second message as human (changed)
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Charlie", "content": "Actually I'm human", "sender_type": "human"}"#)
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/participants"))
        .dispatch();
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(participants.len(), 1);
    // Should use the latest sender_type
    assert_eq!(participants[0]["sender_type"].as_str().unwrap(), "human");
    assert_eq!(participants[0]["message_count"].as_i64().unwrap(), 2);
}

// --- Exclude Sender Filter ---

#[test]
fn test_exclude_sender_single() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Three senders
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Hello from Nanook"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Hello from Forge"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "Hello from Drift"}"#)
        .dispatch();

    // Exclude Forge
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?exclude_sender=Forge"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
    assert!(
        msgs.iter()
            .all(|m| m["sender"].as_str().unwrap() != "Forge")
    );
    assert!(
        msgs.iter()
            .any(|m| m["sender"].as_str().unwrap() == "Nanook")
    );
    assert!(
        msgs.iter()
            .any(|m| m["sender"].as_str().unwrap() == "Drift")
    );
}

#[test]
fn test_exclude_sender_multiple_comma_separated() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-multi"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "From Nanook"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "From Forge"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "From Drift"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Lux", "content": "From Lux"}"#)
        .dispatch();

    // Exclude Forge and Drift (comma-separated)
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?exclude_sender=Forge,Drift"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
    let senders: Vec<&str> = msgs.iter().map(|m| m["sender"].as_str().unwrap()).collect();
    assert!(senders.contains(&"Nanook"));
    assert!(senders.contains(&"Lux"));
    assert!(!senders.contains(&"Forge"));
    assert!(!senders.contains(&"Drift"));
}

#[test]
fn test_exclude_sender_with_after_filter() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-after"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send messages and track seq
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "First"}"#)
        .dispatch();
    let msg1: serde_json::Value = res.into_json().unwrap();
    let seq1 = msg1["seq"].as_i64().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Second"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Third"}"#)
        .dispatch();

    // after=seq1, exclude Forge — should only get Nanook's "Third"
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={seq1}&exclude_sender=Forge"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["sender"].as_str().unwrap(), "Nanook");
    assert_eq!(msgs[0]["content"].as_str().unwrap(), "Third");
}

#[test]
fn test_exclude_sender_activity_feed() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-activity"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Activity from Nanook"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Activity from Forge"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "Activity from Drift"}"#)
        .dispatch();

    // Exclude Forge from activity feed
    let res = client
        .get(format!(
            "/api/v1/activity?room_id={room_id}&exclude_sender=Forge"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let events = body["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|e| e["sender"].as_str().unwrap() != "Forge")
    );
}

#[test]
fn test_exclude_sender_empty_string_ignored() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-empty"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Hello"}"#)
        .dispatch();

    // Empty exclude_sender should return all messages
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?exclude_sender="))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
}

// ===== Message Search Tests =====

#[test]
fn test_search_basic() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-basic"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "The weather is cold today"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "I am building something new"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "The cold never bothered me"}"#)
        .dispatch();

    // Search for "cold" — should find 2 messages
    let res = client.get("/api/v1/search?q=cold").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    assert_eq!(body["query"].as_str().unwrap(), "cold");
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    // Results should include room_name
    assert!(
        results
            .iter()
            .all(|r| r["room_name"].as_str().unwrap() == "search-basic")
    );
}

#[test]
fn test_search_empty_query() {
    let client = test_client();
    let res = client.get("/api/v1/search?q=").dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("empty"));
}

#[test]
fn test_search_no_results() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-empty"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Hello world"}"#)
        .dispatch();

    let res = client.get("/api/v1/search?q=xyznonexistent").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);
    assert_eq!(body["results"].as_array().unwrap().len(), 0);
}

#[test]
fn test_search_filter_by_room() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-room-a"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-room-b"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "deploy to staging"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_b_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "deploy to production"}"#)
        .dispatch();

    // Search "deploy" scoped to room A
    let res = client
        .get(format!("/api/v1/search?q=deploy&room_id={room_a_id}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(
        body["results"][0]["room_name"].as_str().unwrap(),
        "search-room-a"
    );
}

#[test]
fn test_search_filter_by_sender() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-sender"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "fix the bug"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "found the bug"}"#)
        .dispatch();

    // Search "bug" from Nanook only
    let res = client.get("/api/v1/search?q=bug&sender=Nanook").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(body["results"][0]["sender"].as_str().unwrap(), "Nanook");
}

#[test]
fn test_search_with_limit() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-limit"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    for i in 0..5 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender": "Nanook", "content": "message number {i}"}}"#
            ))
            .dispatch();
    }

    // Limit to 2 results
    let res = client.get("/api/v1/search?q=message&limit=2").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
}

#[test]
fn test_search_case_insensitive() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-case"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "IMPORTANT UPDATE"}"#)
        .dispatch();

    // SQLite LIKE is case-insensitive for ASCII by default
    let res = client.get("/api/v1/search?q=important").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_cross_room() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-cross-1"}"#)
        .dispatch();
    let room1: serde_json::Value = res.into_json().unwrap();
    let room1_id = room1["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-cross-2"}"#)
        .dispatch();
    let room2: serde_json::Value = res.into_json().unwrap();
    let room2_id = room2["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room1_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "testing cross-room search"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room2_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "also testing search across rooms"}"#)
        .dispatch();

    // Unscoped search should find both
    let res = client.get("/api/v1/search?q=search").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    let room_names: Vec<&str> = body["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["room_name"].as_str().unwrap())
        .collect();
    assert!(room_names.contains(&"search-cross-1"));
    assert!(room_names.contains(&"search-cross-2"));
}

// --- FTS5 Search ---

#[test]
fn test_search_fts5_word_matching() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "fts-word-match"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "The frobulation process completed successfully"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Starting frobulation on all servers now"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "The weather is nice today"}"#)
        .dispatch();

    // FTS5 word matching: "frobulation" should find exactly the 2 messages containing it
    let res = client.get("/api/v1/search?q=frobulation").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);

    // "weather" should find only 1 message
    let res = client.get("/api/v1/search?q=weather").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_fts5_multi_word() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "fts-multi-word"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "The API test results look good"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Running API integration tests now"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "The weather API is down"}"#)
        .dispatch();

    // Multi-word search: "API test" should match messages with both terms
    let res = client.get("/api/v1/search?q=API%20test").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Both first two messages contain "API" and "test" (via stemming)
    assert_eq!(body["count"].as_u64().unwrap(), 2);
}

#[test]
fn test_search_fts5_edited_message() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "fts-edit"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "original content here"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Verify "original" is searchable
    let res = client.get("/api/v1/search?q=original").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);

    // Edit the message
    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "updated content instead"}"#)
        .dispatch();

    // Old content should no longer be searchable
    let res = client.get("/api/v1/search?q=original").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);

    // New content should be searchable
    let res = client.get("/api/v1/search?q=updated").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_fts5_deleted_message() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "fts-delete"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "ephemeral message to delete"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Verify it's searchable
    let res = client.get("/api/v1/search?q=ephemeral").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);

    // Delete the message
    client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}?sender=Nanook"
        ))
        .dispatch();

    // Should no longer be searchable
    let res = client.get("/api/v1/search?q=ephemeral").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);
}

#[test]
fn test_search_fts5_sender_search() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "fts-sender-search"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "hello from nanook"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "hello from forge"}"#)
        .dispatch();

    // FTS5 indexes sender too — searching for a sender name matches content or sender
    let res = client.get("/api/v1/search?q=nanook").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Should find the message from Nanook (matches sender in FTS)
    assert!(body["count"].as_u64().unwrap() >= 1);
}

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

// --- Room Update ---

#[test]
fn test_update_room_name() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "old-name", "description": "original desc"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"name": "new-name"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let updated: serde_json::Value = res.into_json().unwrap();
    assert_eq!(updated["name"], "new-name");
    assert_eq!(updated["description"], "original desc"); // unchanged

    // Verify via GET
    let res = client
        .get(format!("/api/v1/rooms/{room_id}"))
        .dispatch();
    let fetched: serde_json::Value = res.into_json().unwrap();
    assert_eq!(fetched["name"], "new-name");
}

#[test]
fn test_update_room_description() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "desc-test", "description": "old desc"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"description": "new desc"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let updated: serde_json::Value = res.into_json().unwrap();
    assert_eq!(updated["name"], "desc-test"); // unchanged
    assert_eq!(updated["description"], "new desc");
}

#[test]
fn test_update_room_wrong_admin_key() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "admin-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .body(r#"{"name": "hacked"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_update_room_not_found() {
    let client = test_client();
    let res = client
        .put("/api/v1/rooms/nonexistent-id")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer some-key"))
        .body(r#"{"name": "test"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_update_room_empty_name_rejected() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "valid-name"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"name": ""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_update_room_duplicate_name() {
    let client = test_client();
    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "room-alpha"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let admin_key_a = room_a["admin_key"].as_str().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "room-beta"}"#)
        .dispatch();

    // Try to rename room-alpha to room-beta
    let res = client
        .put(format!("/api/v1/rooms/{room_a_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key_a}"),
        ))
        .body(r#"{"name": "room-beta"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Conflict);
}

#[test]
fn test_update_room_updates_timestamp() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "timestamp-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();
    let original_updated = room["updated_at"].as_str().unwrap().to_string();

    // Small delay to ensure timestamp differs
    std::thread::sleep(std::time::Duration::from_millis(10));

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"description": "updated"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let updated: serde_json::Value = res.into_json().unwrap();
    assert_ne!(updated["updated_at"].as_str().unwrap(), original_updated);
}

// --- before_seq backward pagination tests ---

#[test]
fn test_before_seq_returns_older_messages() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "before-seq-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 5 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=5 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Get messages before seq of msg 4 — should return msg 1, 2, 3
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?before_seq={}",
            seqs[3]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 3);
    // Results should be in chronological order (ASC)
    assert_eq!(msgs[0]["content"], "msg 1");
    assert_eq!(msgs[1]["content"], "msg 2");
    assert_eq!(msgs[2]["content"], "msg 3");
}

#[test]
fn test_before_seq_with_limit() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "before-seq-limit-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 10 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=10 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Get 3 messages before seq of msg 8 — should return msg 5, 6, 7 (most recent 3 before seq 8)
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?before_seq={}&limit=3",
            seqs[7]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0]["content"], "msg 5");
    assert_eq!(msgs[1]["content"], "msg 6");
    assert_eq!(msgs[2]["content"], "msg 7");
}

#[test]
fn test_before_seq_first_message() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "before-seq-first-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 3 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=3 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Get messages before first message's seq — should return empty
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?before_seq={}",
            seqs[0]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 0);
}

#[test]
fn test_before_seq_nonexistent_room() {
    let client = test_client();
    let res = client
        .get("/api/v1/rooms/00000000-0000-0000-0000-000000000000/messages?before_seq=5")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

// --- Message Pinning ---

/// Helper: create a room and return (room_id, admin_key)
fn create_room_with_key(client: &Client, name: &str) -> (String, String) {
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(format!(r#"{{"name": "{name}", "created_by": "tester"}}"#))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();
    let admin_key = body["admin_key"].as_str().unwrap().to_string();
    (room_id, admin_key)
}

/// Helper: send a message and return message_id
fn send_msg(client: &Client, room_id: &str, sender: &str, content: &str) -> String {
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "{sender}", "content": "{content}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    body["id"].as_str().unwrap().to_string()
}

#[test]
fn test_pin_message() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "pin-test");
    let msg_id = send_msg(&client, &room_id, "alice", "Important announcement!");

    let res = client
        .post(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/pin"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["id"], msg_id);
    assert_eq!(body["content"], "Important announcement!");
    assert!(body["pinned_at"].as_str().is_some());
    assert_eq!(body["pinned_by"], "admin");
}

#[test]
fn test_pin_message_already_pinned() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "pin-dup-test");
    let msg_id = send_msg(&client, &room_id, "alice", "Pin me once");

    // Pin it
    let res = client
        .post(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/pin"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Try to pin again
    let res = client
        .post(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/pin"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Conflict);
}

#[test]
fn test_pin_message_wrong_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_room_with_key(&client, "pin-auth-test");
    let msg_id = send_msg(&client, &room_id, "alice", "Secret message");

    let res = client
        .post(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/pin"
        ))
        .header(Header::new("Authorization", "Bearer wrong_key"))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_pin_message_not_found() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "pin-404-test");

    let res = client
        .post(format!(
            "/api/v1/rooms/{room_id}/messages/nonexistent-id/pin"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_unpin_message() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "unpin-test");
    let msg_id = send_msg(&client, &room_id, "alice", "Pinned then unpinned");

    // Pin
    let res = client
        .post(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/pin"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Unpin
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/pin"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["status"], "unpinned");
    assert_eq!(body["message_id"], msg_id);
}

#[test]
fn test_unpin_message_not_pinned() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "unpin-notpinned-test");
    let msg_id = send_msg(&client, &room_id, "alice", "Never pinned");

    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/pin"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_list_pins() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "list-pins-test");
    let msg1 = send_msg(&client, &room_id, "alice", "First pinned");
    let _msg2 = send_msg(&client, &room_id, "bob", "Not pinned");
    let msg3 = send_msg(&client, &room_id, "alice", "Second pinned");

    // Pin two messages
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg1}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg3}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // List pins
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/pins"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let pins: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(pins.len(), 2);
    // Newest-pinned first
    assert_eq!(pins[0]["id"], msg3);
    assert_eq!(pins[1]["id"], msg1);
}

#[test]
fn test_list_pins_empty() {
    let client = test_client();
    let (room_id, _admin_key) = create_room_with_key(&client, "list-pins-empty-test");
    send_msg(&client, &room_id, "alice", "No pins here");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/pins"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let pins: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(pins.len(), 0);
}

#[test]
fn test_list_pins_nonexistent_room() {
    let client = test_client();
    let res = client
        .get("/api/v1/rooms/00000000-0000-0000-0000-000000000000/pins")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_pinned_message_in_get_messages() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "pin-in-messages-test");
    let msg_id = send_msg(&client, &room_id, "alice", "Pin visible in list");

    // Pin it
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Get messages - should include pinned_at/pinned_by
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let messages: Vec<serde_json::Value> = res.into_json().unwrap();
    let pinned_msg = messages.iter().find(|m| m["id"] == msg_id).unwrap();
    assert!(pinned_msg["pinned_at"].as_str().is_some());
    assert_eq!(pinned_msg["pinned_by"], "admin");
}

#[test]
fn test_pin_then_unpin_clears_fields() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "pin-unpin-clear-test");
    let msg_id = send_msg(&client, &room_id, "alice", "Pin and unpin me");

    // Pin
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Unpin
    client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Get messages - pinned_at should be null
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let messages: Vec<serde_json::Value> = res.into_json().unwrap();
    let msg = messages.iter().find(|m| m["id"] == msg_id).unwrap();
    assert!(msg.get("pinned_at").is_none() || msg["pinned_at"].is_null());
    assert!(msg.get("pinned_by").is_none() || msg["pinned_by"].is_null());
}

#[test]
fn test_pin_repin_after_unpin() {
    let client = test_client();
    let (room_id, admin_key) = create_room_with_key(&client, "pin-repin-test");
    let msg_id = send_msg(&client, &room_id, "alice", "Pin me again");

    // Pin
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Unpin
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Re-pin
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["pinned_at"].as_str().is_some());
}

// --- Presence ---

#[test]
fn test_room_presence_empty() {
    let client = test_client();
    // Get general room ID
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client.get(format!("/api/v1/rooms/{room_id}/presence")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["room_id"], room_id);
    assert_eq!(body["count"], 0);
    assert!(body["online"].as_array().unwrap().is_empty());
}

#[test]
fn test_room_presence_nonexistent_room() {
    let client = test_client();
    let res = client.get("/api/v1/rooms/nonexistent/presence").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_global_presence_empty() {
    let client = test_client();
    let res = client.get("/api/v1/presence").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["total_online"], 0);
    assert!(body["rooms"].as_object().unwrap().is_empty());
}

#[test]
fn test_presence_tracker_unit() {
    // Unit test for PresenceTracker directly
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    // Join
    let is_new = tracker.join("room1", "alice", Some("agent"));
    assert!(is_new, "First join should be new");

    // Get room presence
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 1);
    assert_eq!(online[0].sender, "alice");
    assert_eq!(online[0].sender_type.as_deref(), Some("agent"));

    // Second join (same sender, same room) should NOT be new
    let is_new = tracker.join("room1", "alice", Some("agent"));
    assert!(!is_new, "Second join from same sender should not be new");

    // Still only 1 unique presence
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 1);

    // Leave once — should NOT be fully left (2 connections)
    let fully_left = tracker.leave("room1", "alice");
    assert!(!fully_left, "First leave with 2 connections should not be fully left");

    // Still present
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 1);

    // Leave again — should be fully left
    let fully_left = tracker.leave("room1", "alice");
    assert!(fully_left, "Second leave should be fully left");

    // Now gone
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 0);
}

#[test]
fn test_presence_tracker_multiple_users() {
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    tracker.join("room1", "alice", Some("agent"));
    tracker.join("room1", "bob", Some("human"));
    tracker.join("room2", "charlie", None);

    // Room 1 has 2 users
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 2);

    // Room 2 has 1 user
    let online = tracker.get_room("room2");
    assert_eq!(online.len(), 1);
    assert_eq!(online[0].sender, "charlie");

    // Global: 2 rooms, 3 unique users
    let all = tracker.get_all();
    assert_eq!(all.len(), 2);
    let total: usize = all.values().map(|v| v.len()).sum();
    assert_eq!(total, 3);
}

#[test]
fn test_presence_tracker_leave_cleans_empty_rooms() {
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    tracker.join("room1", "alice", None);
    tracker.leave("room1", "alice");

    // Room should be cleaned up from the map
    let all = tracker.get_all();
    assert!(all.is_empty(), "Empty rooms should be cleaned up");
}

#[test]
fn test_presence_tracker_leave_nonexistent() {
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    // Leave from a room that doesn't exist — should not panic
    let result = tracker.leave("room1", "nobody");
    assert!(!result);
}

#[test]
fn test_presence_tracker_sender_type_update() {
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    // First join without sender_type
    tracker.join("room1", "alice", None);
    let online = tracker.get_room("room1");
    assert!(online[0].sender_type.is_none());

    // Second join with sender_type should update it
    tracker.join("room1", "alice", Some("agent"));
    let online = tracker.get_room("room1");
    assert_eq!(online[0].sender_type.as_deref(), Some("agent"));
}

#[test]
fn test_stream_with_sender_registers_presence() {
    let client = test_client();
    // Get general room ID
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Connect to SSE stream with sender
    let response = client
        .get(format!("/api/v1/rooms/{room_id}/stream?sender=nanook&sender_type=agent"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);

    // While the stream is alive, presence should show nanook
    let res = client.get(format!("/api/v1/rooms/{room_id}/presence")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);
    let online = body["online"].as_array().unwrap();
    assert_eq!(online[0]["sender"], "nanook");
    assert_eq!(online[0]["sender_type"], "agent");
    assert!(online[0]["connected_at"].as_str().is_some());

    // Drop the response (SSE stream) — presence should be cleaned up
    drop(response);

    let res = client.get(format!("/api/v1/rooms/{room_id}/presence")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 0);
}

#[test]
fn test_stream_without_sender_no_presence() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Connect to SSE stream without sender (backward compat)
    let _response = client
        .get(format!("/api/v1/rooms/{room_id}/stream"))
        .dispatch();

    // No presence should be registered
    let res = client.get(format!("/api/v1/rooms/{room_id}/presence")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 0);
}

#[test]
fn test_global_presence_with_connections() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Create a second room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "testing", "created_by": "test"}"#)
        .dispatch();
    let room2: serde_json::Value = res.into_json().unwrap();
    let room2_id = room2["id"].as_str().unwrap();

    // Connect to both rooms
    let _stream1 = client
        .get(format!("/api/v1/rooms/{room_id}/stream?sender=agent-a&sender_type=agent"))
        .dispatch();
    let _stream2 = client
        .get(format!("/api/v1/rooms/{room2_id}/stream?sender=agent-b&sender_type=agent"))
        .dispatch();

    // Global presence should show both
    let res = client.get("/api/v1/presence").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["total_online"], 2);
    assert_eq!(body["rooms"].as_object().unwrap().len(), 2);
}

// --- Webhooks ---

/// Helper: create a room and return (room_id, admin_key)
fn create_test_room(client: &Client, name: &str) -> (String, String) {
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(format!(r#"{{"name": "{name}", "created_by": "tester"}}"#))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    (
        body["id"].as_str().unwrap().to_string(),
        body["admin_key"].as_str().unwrap().to_string(),
    )
}

#[test]
fn test_create_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-1");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["room_id"], room_id);
    assert_eq!(body["url"], "http://localhost:9999/hook");
    assert_eq!(body["events"], "*");
    assert_eq!(body["active"], true);
    assert!(!body["id"].as_str().unwrap().is_empty());
}

#[test]
fn test_create_webhook_with_event_filter() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-2");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "http://localhost:9999/hook", "events": "message,message_edited", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["events"], "message,message_edited");
}

#[test]
fn test_create_webhook_with_secret() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-3");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "http://localhost:9999/hook", "secret": "my-secret-key", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["has_secret"], true);
}

#[test]
fn test_create_webhook_invalid_url() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-4");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "not-a-url", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_create_webhook_invalid_event() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-5");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "http://localhost:9999/hook", "events": "message,bogus_event", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("bogus_event"));
}

#[test]
fn test_create_webhook_wrong_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "webhook-room-6");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_create_webhook_no_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "webhook-room-7");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    // Without admin key, Rocket forwards (no AdminKey extracted) → 404 or Unauthorized
    assert!(res.status() == Status::Unauthorized || res.status() == Status::NotFound);
}

#[test]
fn test_create_webhook_nonexistent_room() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms/nonexistent-room-id/webhooks")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer some-key"))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_list_webhooks() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-list-room");

    // Create two webhooks
    client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook1", "created_by": "tester"}"#)
        .dispatch();

    client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook2", "events": "message", "created_by": "tester"}"#)
        .dispatch();

    // List
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let webhooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(webhooks.len(), 2);
    // Newest first
    assert_eq!(webhooks[0]["url"], "http://localhost:9999/hook2");
    assert_eq!(webhooks[1]["url"], "http://localhost:9999/hook1");
}

#[test]
fn test_list_webhooks_empty() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-empty-room");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let webhooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(webhooks.len(), 0);
}

#[test]
fn test_list_webhooks_wrong_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "webhook-list-auth");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_delete_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-del-room");

    // Create webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    let webhook: serde_json::Value = res.into_json().unwrap();
    let webhook_id = webhook["id"].as_str().unwrap();

    // Delete it
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/webhooks/{webhook_id}"
        ))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["deleted"], true);

    // Verify it's gone
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    let webhooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(webhooks.len(), 0);
}

#[test]
fn test_delete_webhook_not_found() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-del-nf");

    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/webhooks/nonexistent-id"
        ))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_webhook_wrong_admin_key() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-del-auth");

    // Create webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    let webhook: serde_json::Value = res.into_json().unwrap();
    let webhook_id = webhook["id"].as_str().unwrap();

    // Try to delete with wrong key
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/webhooks/{webhook_id}"
        ))
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_update_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-update-room");

    // Create webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    let webhook: serde_json::Value = res.into_json().unwrap();
    let webhook_id = webhook["id"].as_str().unwrap();

    // Update URL and deactivate
    let res = client
        .put(format!(
            "/api/v1/rooms/{room_id}/webhooks/{webhook_id}"
        ))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/new-hook", "active": false}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["updated"], true);

    // Verify via list
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    let webhooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(webhooks.len(), 1);
    assert_eq!(webhooks[0]["url"], "http://localhost:9999/new-hook");
    assert_eq!(webhooks[0]["active"], false);
}

#[test]
fn test_update_webhook_not_found() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-update-nf");

    let res = client
        .put(format!(
            "/api/v1/rooms/{room_id}/webhooks/nonexistent-id"
        ))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/new-hook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_update_webhook_no_fields() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-update-empty");

    // Create webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    let webhook: serde_json::Value = res.into_json().unwrap();
    let webhook_id = webhook["id"].as_str().unwrap();

    // Update with empty body
    let res = client
        .put(format!(
            "/api/v1/rooms/{room_id}/webhooks/{webhook_id}"
        ))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_webhooks_cascade_on_room_delete() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-cascade-room");

    // Create a webhook
    client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();

    // Delete the room
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Webhooks should be gone (room is gone, so 404)
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

// --- Thread View ---

#[test]
fn test_thread_simple() {
    let client = test_client();

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-test", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send root message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Root message"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    // Send reply to root
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "Reply 1", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let reply1: serde_json::Value = res.into_json().unwrap();
    let reply1_id = reply1["id"].as_str().unwrap();

    // Send reply to the reply (nested)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "charlie", "content": "Nested reply", "reply_to": "{reply1_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Get thread from root
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["root"]["id"], root_id);
    assert_eq!(thread["root"]["content"], "Root message");
    assert_eq!(thread["total_replies"], 2);

    let replies = thread["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["content"], "Reply 1");
    assert_eq!(replies[0]["depth"], 1);
    assert_eq!(replies[1]["content"], "Nested reply");
    assert_eq!(replies[1]["depth"], 2);
}

#[test]
fn test_thread_from_child_walks_to_root() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-walk", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Root -> Reply1 -> Reply2
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Thread root"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "Mid reply", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    let mid: serde_json::Value = res.into_json().unwrap();
    let mid_id = mid["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "charlie", "content": "Leaf reply", "reply_to": "{mid_id}"}}"#
        ))
        .dispatch();
    let leaf: serde_json::Value = res.into_json().unwrap();
    let leaf_id = leaf["id"].as_str().unwrap();

    // Request thread from the LEAF message — should walk up to root
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{leaf_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    // Root should be the original root message
    assert_eq!(thread["root"]["id"], root_id);
    assert_eq!(thread["root"]["content"], "Thread root");
    assert_eq!(thread["total_replies"], 2);
}

#[test]
fn test_thread_single_message_no_replies() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-solo", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Standalone"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["root"]["id"], msg_id);
    assert_eq!(thread["total_replies"], 0);
    assert!(thread["replies"].as_array().unwrap().is_empty());
}

#[test]
fn test_thread_not_found() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-404", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Non-existent message
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/nonexistent-id/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_thread_wrong_room() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "room-a", "created_by": "tester"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "room-b", "created_by": "tester"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    // Create message in room A
    let res = client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "In room A"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Try to get thread in room B — should 404
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_b_id}/messages/{msg_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_thread_multiple_branches() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-branches", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Root message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Root"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    // Two direct replies to root (branching)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "Branch A", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "charlie", "content": "Branch B", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Get thread — should have root + 2 branches
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["total_replies"], 2);
    let replies = thread["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 2);
    // Both should be depth 1
    assert!(replies.iter().all(|r| r["depth"] == 1));
}

#[test]
fn test_thread_nonexistent_room() {
    let client = test_client();

    let res = client
        .get("/api/v1/rooms/nonexistent-room/messages/some-id/thread")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["error"], "Room not found");
}
