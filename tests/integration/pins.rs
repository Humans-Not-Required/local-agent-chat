use rocket::local::blocking::Client;
use rocket::http::{ContentType, Header, Status};
use crate::common::test_client;

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
