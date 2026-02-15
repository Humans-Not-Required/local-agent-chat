use rocket::local::blocking::Client;
use rocket::http::{ContentType, Header, Status};
use crate::common::{test_client, create_test_room};

// --- Read Positions ---

/// Helper: send a message and return the seq
fn send_test_message(client: &Client, room_id: &str, sender: &str, content: &str) -> i64 {
    let res = client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "{}", "content": "{}"}}"#,
            sender, content
        ))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    body["seq"].as_i64().unwrap()
}

#[test]
fn test_update_read_position() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "read-test-1");

    let seq = send_test_message(&client, &room_id, "alice", "hello");

    let res = client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "alice", "last_read_seq": {}}}"#,
            seq
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["sender"], "alice");
    assert_eq!(body["last_read_seq"], seq);
    assert_eq!(body["room_id"], room_id);
    assert!(body["updated_at"].as_str().is_some());
}

#[test]
fn test_read_position_upsert_only_increases() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "read-test-upsert");

    let seq1 = send_test_message(&client, &room_id, "alice", "msg1");
    let seq2 = send_test_message(&client, &room_id, "alice", "msg2");

    // Set to seq2
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "alice", "last_read_seq": {}}}"#,
            seq2
        ))
        .dispatch();

    // Try to set back to seq1 (should be ignored)
    let res = client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "alice", "last_read_seq": {}}}"#,
            seq1
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Should still be seq2
    assert_eq!(body["last_read_seq"], seq2);
}

#[test]
fn test_read_position_nonexistent_room() {
    let client = test_client();
    let res = client
        .put("/api/v1/rooms/nonexistent-room/read")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "last_read_seq": 1}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_read_position_empty_sender() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "read-test-empty-sender");

    let res = client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender": "  ", "last_read_seq": 1}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_get_read_positions() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "read-test-get");

    let seq1 = send_test_message(&client, &room_id, "alice", "hello");
    let seq2 = send_test_message(&client, &room_id, "bob", "world");

    // Alice reads up to seq1
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "alice", "last_read_seq": {}}}"#,
            seq1
        ))
        .dispatch();

    // Bob reads up to seq2
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "last_read_seq": {}}}"#,
            seq2
        ))
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{}/read", room_id))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(body.len(), 2);

    // Sorted by updated_at DESC, so bob (latest update) should be first
    let senders: Vec<&str> = body.iter().map(|p| p["sender"].as_str().unwrap()).collect();
    assert!(senders.contains(&"alice"));
    assert!(senders.contains(&"bob"));
}

#[test]
fn test_get_read_positions_nonexistent_room() {
    let client = test_client();
    let res = client
        .get("/api/v1/rooms/nonexistent-room/read")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_get_read_positions_empty() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "read-test-empty");

    let res = client
        .get(format!("/api/v1/rooms/{}/read", room_id))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(body.len(), 0);
}

#[test]
fn test_unread_counts() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "unread-test-1");

    let seq1 = send_test_message(&client, &room_id, "alice", "msg1");
    send_test_message(&client, &room_id, "alice", "msg2");
    let seq3 = send_test_message(&client, &room_id, "bob", "msg3");

    // Bob reads up to seq1 (so 2 unread: msg2, msg3)
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "last_read_seq": {}}}"#,
            seq1
        ))
        .dispatch();

    let res = client.get("/api/v1/unread?sender=bob").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["sender"], "bob");

    // Find our room in the response
    let rooms = body["rooms"].as_array().unwrap();
    let our_room = rooms
        .iter()
        .find(|r| r["room_id"].as_str().unwrap() == room_id)
        .unwrap();
    assert_eq!(our_room["unread_count"], 2);
    assert_eq!(our_room["last_read_seq"], seq1);
    assert_eq!(our_room["latest_seq"], seq3);

    assert!(body["total_unread"].as_i64().unwrap() >= 2);
}

#[test]
fn test_unread_counts_no_read_position() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "unread-test-no-pos");

    send_test_message(&client, &room_id, "alice", "msg1");
    send_test_message(&client, &room_id, "alice", "msg2");

    // Bob has never set a read position, so all messages are unread
    let res = client.get("/api/v1/unread?sender=bob").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    let rooms = body["rooms"].as_array().unwrap();
    let our_room = rooms
        .iter()
        .find(|r| r["room_id"].as_str().unwrap() == room_id)
        .unwrap();
    // With no read position, last_read_seq=0, so all messages are unread
    assert_eq!(our_room["unread_count"], 2);
    assert_eq!(our_room["last_read_seq"], 0);
}

#[test]
fn test_unread_counts_empty_sender() {
    let client = test_client();
    let res = client.get("/api/v1/unread?sender=%20").dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_unread_counts_all_read() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "unread-test-all-read");

    let seq1 = send_test_message(&client, &room_id, "alice", "msg1");

    // Alice reads up to the latest
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "alice", "last_read_seq": {}}}"#,
            seq1
        ))
        .dispatch();

    let res = client.get("/api/v1/unread?sender=alice").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    let rooms = body["rooms"].as_array().unwrap();
    let our_room = rooms
        .iter()
        .find(|r| r["room_id"].as_str().unwrap() == room_id)
        .unwrap();
    assert_eq!(our_room["unread_count"], 0);
}

#[test]
fn test_read_positions_cascade_on_room_delete() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "read-test-cascade");

    let seq = send_test_message(&client, &room_id, "alice", "msg");

    // Set a read position
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "alice", "last_read_seq": {}}}"#,
            seq
        ))
        .dispatch();

    // Delete the room
    client
        .delete(format!("/api/v1/rooms/{}", room_id))
        .header(Header::new("Authorization", format!("Bearer {}", admin_key)))
        .dispatch();

    // Unread should not include deleted room
    let res = client.get("/api/v1/unread?sender=alice").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let rooms = body["rooms"].as_array().unwrap();
    assert!(!rooms
        .iter()
        .any(|r| r["room_id"].as_str().unwrap() == room_id));
}

#[test]
fn test_unread_multiple_rooms() {
    let client = test_client();
    let (room_a, _) = create_test_room(&client, "unread-multi-a");
    let (room_b, _) = create_test_room(&client, "unread-multi-b");

    // 3 messages in room A, 2 in room B
    send_test_message(&client, &room_a, "alice", "a1");
    let seq_a2 = send_test_message(&client, &room_a, "alice", "a2");
    send_test_message(&client, &room_a, "alice", "a3");

    send_test_message(&client, &room_b, "bob", "b1");
    send_test_message(&client, &room_b, "bob", "b2");

    // Charlie reads up to a2 in room A (1 unread), nothing in room B (2 unread)
    client
        .put(format!("/api/v1/rooms/{}/read", room_a))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "charlie", "last_read_seq": {}}}"#,
            seq_a2
        ))
        .dispatch();

    let res = client.get("/api/v1/unread?sender=charlie").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    let rooms = body["rooms"].as_array().unwrap();
    let a_room = rooms
        .iter()
        .find(|r| r["room_id"].as_str().unwrap() == room_a)
        .unwrap();
    let b_room = rooms
        .iter()
        .find(|r| r["room_id"].as_str().unwrap() == room_b)
        .unwrap();

    assert_eq!(a_room["unread_count"], 1);
    assert_eq!(b_room["unread_count"], 2);
    assert!(body["total_unread"].as_i64().unwrap() >= 3);
}
