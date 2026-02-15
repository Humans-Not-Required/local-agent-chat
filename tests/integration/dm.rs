use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Direct Messages ---

#[test]
fn test_send_dm_creates_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"Hey Bob!"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["created"], true);
    assert_eq!(body["message"]["sender"], "alice");
    assert_eq!(body["message"]["content"], "Hey Bob!");
    assert!(!body["room_id"].as_str().unwrap().is_empty());
}

#[test]
fn test_send_dm_idempotent_room() {
    let client = test_client();
    // First DM creates the room
    let res1 = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"First message"}"#)
        .dispatch();
    assert_eq!(res1.status(), Status::Ok);
    let body1: serde_json::Value = res1.into_json().unwrap();
    assert_eq!(body1["created"], true);
    let room_id = body1["room_id"].as_str().unwrap().to_string();

    // Second DM reuses the same room
    let res2 = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"Second message"}"#)
        .dispatch();
    assert_eq!(res2.status(), Status::Ok);
    let body2: serde_json::Value = res2.into_json().unwrap();
    assert_eq!(body2["created"], false);
    assert_eq!(body2["room_id"], room_id);
}

#[test]
fn test_send_dm_symmetric() {
    let client = test_client();
    // alice → bob
    let res1 = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"Hello from Alice"}"#)
        .dispatch();
    let body1: serde_json::Value = res1.into_json().unwrap();
    let room_id = body1["room_id"].as_str().unwrap().to_string();

    // bob → alice should use the same room
    let res2 = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"bob","recipient":"alice","content":"Hello from Bob"}"#)
        .dispatch();
    let body2: serde_json::Value = res2.into_json().unwrap();
    assert_eq!(body2["room_id"], room_id);
    assert_eq!(body2["created"], false);
}

#[test]
fn test_dm_not_in_room_list() {
    let client = test_client();
    // Create a DM
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"Secret DM"}"#)
        .dispatch();

    // Regular room list should not include DM rooms
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    for room in &rooms {
        assert!(
            !room["name"].as_str().unwrap().starts_with("dm:"),
            "DM room should not appear in regular room list"
        );
    }
}

#[test]
fn test_list_dm_conversations() {
    let client = test_client();
    // Create DMs with two different people
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"Hi Bob"}"#)
        .dispatch();
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"charlie","content":"Hi Charlie"}"#)
        .dispatch();

    // List alice's conversations
    let res = client.get("/api/v1/dm?sender=alice").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["sender"], "alice");
    assert_eq!(body["count"], 2);

    let convos = body["conversations"].as_array().unwrap();
    let participants: Vec<&str> = convos
        .iter()
        .map(|c| c["other_participant"].as_str().unwrap())
        .collect();
    assert!(participants.contains(&"bob"));
    assert!(participants.contains(&"charlie"));
}

#[test]
fn test_list_dm_conversations_with_unread() {
    let client = test_client();
    // alice sends a DM to bob
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"Message 1"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["room_id"].as_str().unwrap().to_string();

    // Send another message
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"Message 2"}"#)
        .dispatch();

    // bob hasn't read anything — should have 2 unread
    let res = client.get("/api/v1/dm?sender=bob").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let convos = body["conversations"].as_array().unwrap();
    assert_eq!(convos.len(), 1);
    assert_eq!(convos[0]["unread_count"], 2);

    // bob reads up to seq 1
    let mark_body = serde_json::json!({"sender": "bob", "last_read_seq": 1});
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(mark_body.to_string())
        .dispatch();

    // Now bob should have 1 unread
    let res = client.get("/api/v1/dm?sender=bob").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let convos = body["conversations"].as_array().unwrap();
    assert_eq!(convos[0]["unread_count"], 1);
}

#[test]
fn test_dm_self_message_rejected() {
    let client = test_client();
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"alice","content":"Talking to myself"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("yourself"));
}

#[test]
fn test_dm_empty_content_rejected() {
    let client = test_client();
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_dm_empty_sender_rejected() {
    let client = test_client();
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"","recipient":"bob","content":"Hello"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_dm_messages_via_regular_api() {
    let client = test_client();
    // Create a DM
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"DM message"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["room_id"].as_str().unwrap();

    // Read DM messages using the regular messages API
    let res = client
        .get(format!("/api/v1/rooms/{}/messages", room_id))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let messages: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["content"], "DM message");
}

#[test]
fn test_dm_with_sender_type() {
    let client = test_client();
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"nanook","recipient":"forge","content":"Agent DM","sender_type":"agent"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message"]["sender_type"], "agent");
}

#[test]
fn test_dm_with_metadata() {
    let client = test_client();
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"nanook","recipient":"forge","content":"With meta","metadata":{"priority":"high"}}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message"]["metadata"]["priority"], "high");
}

#[test]
fn test_get_dm_conversation() {
    let client = test_client();
    // Create a DM
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"Hey"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["room_id"].as_str().unwrap();

    // Get the DM conversation info
    let res = client.get(format!("/api/v1/dm/{}", room_id)).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["room_type"], "dm");
    assert_eq!(body["message_count"], 1);
}

#[test]
fn test_get_dm_conversation_not_found() {
    let client = test_client();
    let res = client
        .get("/api/v1/dm/nonexistent-room-id")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_dm_last_message_preview() {
    let client = test_client();
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"First"}"#)
        .dispatch();
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"bob","recipient":"alice","content":"Latest reply"}"#)
        .dispatch();

    let res = client.get("/api/v1/dm?sender=alice").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let convos = body["conversations"].as_array().unwrap();
    assert_eq!(convos[0]["last_message_content"], "Latest reply");
    assert_eq!(convos[0]["last_message_sender"], "bob");
    assert_eq!(convos[0]["message_count"], 2);
}

#[test]
fn test_dm_search_included() {
    let client = test_client();
    // Create a DM with searchable content
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"secret agent handshake"}"#)
        .dispatch();

    // Search should find DM messages
    let res = client.get("/api/v1/search?q=handshake").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["count"].as_i64().unwrap() >= 1);
}
