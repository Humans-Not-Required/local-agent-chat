use rocket::http::{ContentType, Status};
use crate::common::test_client;

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
