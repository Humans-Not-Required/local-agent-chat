use rocket::http::{ContentType, Status};
use crate::common::test_client;

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
