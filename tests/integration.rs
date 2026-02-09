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
    let res = client
        .get("/api/v1/rooms/nonexistent-id")
        .dispatch();
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

    let res = client
        .delete(format!("/api/v1/rooms/{id}"))
        .header(Header::new("Authorization", "Bearer test-admin-key"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify deleted
    let res = client.get(format!("/api/v1/rooms/{id}")).dispatch();
    assert_eq!(res.status(), Status::NotFound);
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

    // Send a message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "test"}"#)
        .dispatch();

    // Delete room
    client
        .delete(format!("/api/v1/rooms/{room_id}"))
        .header(Header::new("Authorization", "Bearer admin"))
        .dispatch();

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
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}?sender=Bot"))
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
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}?sender=BotB"))
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

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Bot","content":"Admin will delete me"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Admin deletes without matching sender
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(Header::new("Authorization", "Bearer some-admin-key"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
}

#[test]
fn test_delete_message_not_found() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/nonexistent?sender=Bot"))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

// --- Messages in nonexistent room ---

#[test]
fn test_get_messages_nonexistent_room() {
    let client = test_client();
    let res = client
        .get("/api/v1/rooms/nonexistent/messages")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}
