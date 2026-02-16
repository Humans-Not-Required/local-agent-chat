use rocket::http::{ContentType, Status};
use crate::common::{test_client, create_test_room};

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

#[test]
fn test_reply_to_deleted_message_fails() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "reply-delete-test");

    // Send original message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"Will be deleted"}"#)
        .dispatch();
    let original: serde_json::Value = res.into_json().unwrap();
    let original_id = original["id"].as_str().unwrap();

    // Delete the message (using admin key)
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{original_id}"))
        .header(rocket::http::Header::new("Authorization", format!("Bearer {}", admin_key)))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Try to reply to the deleted message — should fail
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender":"bob","content":"Reply to ghost","reply_to":"{original_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_reply_to_empty_string_ignored() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "reply-empty-test");

    // Send with reply_to as empty string — should be treated as no reply
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"bot","content":"Empty reply_to","reply_to":""}"#)
        .dispatch();
    // Should either succeed (empty string treated as null) or fail validation
    // Either behavior is acceptable, but it shouldn't crash
    assert!(
        res.status() == Status::Ok || res.status() == Status::BadRequest,
        "Should either accept (as null) or reject (as invalid), got {}",
        res.status()
    );
}

#[test]
fn test_reply_chain_preserved_after_edit() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "reply-edit-test");

    // Send original → reply
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"Original text"}"#)
        .dispatch();
    let original: serde_json::Value = res.into_json().unwrap();
    let original_id = original["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender":"bob","content":"I agree!","reply_to":"{original_id}"}}"#
        ))
        .dispatch();
    let reply: serde_json::Value = res.into_json().unwrap();
    let reply_id = reply["id"].as_str().unwrap();

    // Edit the original message
    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{original_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"Edited original text"}"#)
        .dispatch();

    // Reply should still point to the original
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    let reply_msg = msgs.iter().find(|m| m["id"] == reply_id).unwrap();
    assert_eq!(reply_msg["reply_to"], original_id);
}

#[test]
fn test_multiple_replies_to_same_message() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "multi-reply-test");

    // Send root message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"What do you think?"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    // Multiple people reply to the same message
    for name in ["bob", "charlie", "dave", "eve"] {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender":"{name}","content":"Reply from {name}","reply_to":"{root_id}"}}"#
            ))
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let reply: serde_json::Value = res.into_json().unwrap();
        assert_eq!(reply["reply_to"], root_id);
    }

    // Verify all replies are in the message list
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    let replies: Vec<&serde_json::Value> = msgs
        .iter()
        .filter(|m| m["reply_to"].as_str() == Some(root_id))
        .collect();
    assert_eq!(replies.len(), 4);
}
