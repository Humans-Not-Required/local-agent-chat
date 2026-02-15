use rocket::http::{ContentType, Status};
use crate::common::{test_client, create_test_room};

// --- Room Bookmarks ---

#[test]
fn test_add_bookmark() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "bookmark-room-1");

    let res = client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["room_id"], room_id);
    assert_eq!(body["sender"], "nanook");
    assert_eq!(body["bookmarked"], true);
    assert_eq!(body["created"], true);
}

#[test]
fn test_add_bookmark_idempotent() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "bookmark-room-idempotent");

    // First bookmark
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["created"], true);

    // Second bookmark — same sender, same room
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["bookmarked"], true);
    assert_eq!(body["created"], false); // Already existed
}

#[test]
fn test_add_bookmark_nonexistent_room() {
    let client = test_client();

    let res = client
        .put("/api/v1/rooms/nonexistent-room-id/bookmark")
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_add_bookmark_empty_sender() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "bookmark-room-empty-sender");

    let res = client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": ""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_remove_bookmark() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "bookmark-room-remove");

    // Add bookmark first
    client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();

    // Remove it
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/bookmark?sender=nanook"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["bookmarked"], false);
    assert_eq!(body["removed"], true);
}

#[test]
fn test_remove_bookmark_not_bookmarked() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "bookmark-room-not-bookmarked");

    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/bookmark?sender=nanook"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["removed"], false);
}

#[test]
fn test_list_bookmarks() {
    let client = test_client();
    let (room_id1, _) = create_test_room(&client, "bookmark-list-room-1");
    let (room_id2, _) = create_test_room(&client, "bookmark-list-room-2");

    // Bookmark both rooms
    client
        .put(format!("/api/v1/rooms/{room_id1}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();
    client
        .put(format!("/api/v1/rooms/{room_id2}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();

    let res = client.get("/api/v1/bookmarks?sender=nanook").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["sender"], "nanook");
    assert_eq!(body["count"], 2);

    let bookmarks = body["bookmarks"].as_array().unwrap();
    let room_names: Vec<&str> = bookmarks
        .iter()
        .map(|b| b["room_name"].as_str().unwrap())
        .collect();
    assert!(room_names.contains(&"bookmark-list-room-1"));
    assert!(room_names.contains(&"bookmark-list-room-2"));
}

#[test]
fn test_list_bookmarks_empty() {
    let client = test_client();

    let res = client.get("/api/v1/bookmarks?sender=nobody").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 0);
    assert_eq!(body["bookmarks"].as_array().unwrap().len(), 0);
}

#[test]
fn test_list_bookmarks_per_sender() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "bookmark-per-sender-room");

    // Two different senders bookmark the same room
    client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice"}"#)
        .dispatch();
    client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob"}"#)
        .dispatch();

    // Alice should see 1 bookmark
    let res = client.get("/api/v1/bookmarks?sender=alice").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);

    // Bob should see 1 bookmark
    let res = client.get("/api/v1/bookmarks?sender=bob").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);
}

#[test]
fn test_room_list_with_bookmark_status() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "bookmark-status-room");

    // Bookmark the room
    client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();

    // List rooms with sender parameter
    let res = client.get("/api/v1/rooms?sender=nanook").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();

    // Find our bookmarked room
    let bookmarked = rooms.iter().find(|r| r["id"] == room_id).unwrap();
    assert_eq!(bookmarked["bookmarked"], true);

    // General room should NOT be bookmarked
    let general = rooms.iter().find(|r| r["name"] == "general").unwrap();
    assert_eq!(general["bookmarked"], false);
}

#[test]
fn test_room_list_without_sender_no_bookmark_field() {
    let client = test_client();

    // List rooms WITHOUT sender parameter
    let res = client.get("/api/v1/rooms").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();

    // bookmarked field should be absent (None → skipped by serde)
    for room in &rooms {
        assert!(
            room.get("bookmarked").is_none(),
            "bookmarked field should not be present without sender param"
        );
    }
}

#[test]
fn test_bookmarked_rooms_sorted_first() {
    let client = test_client();
    // Create rooms with messages to give them activity timestamps
    let (room_id_a, _) = create_test_room(&client, "aaa-sort-room");
    let (room_id_b, _) = create_test_room(&client, "bbb-sort-room");

    // Send messages to both rooms to give them activity
    client
        .post(format!("/api/v1/rooms/{room_id_a}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "test", "content": "hello a"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id_b}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "test", "content": "hello b"}"#)
        .dispatch();

    // Bookmark room_id_a only (the one that would sort second by name)
    client
        .put(format!("/api/v1/rooms/{room_id_a}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "sorter"}"#)
        .dispatch();

    // List rooms with sender — bookmarked room should appear before non-bookmarked
    let res = client.get("/api/v1/rooms?sender=sorter").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();

    // Find positions
    let pos_a = rooms.iter().position(|r| r["id"] == room_id_a).unwrap();
    let pos_b = rooms.iter().position(|r| r["id"] == room_id_b).unwrap();

    assert!(
        pos_a < pos_b,
        "Bookmarked room (pos {}) should appear before non-bookmarked room (pos {})",
        pos_a, pos_b
    );
}

#[test]
fn test_bookmark_cascade_on_room_delete() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "bookmark-cascade-room");

    // Bookmark the room
    client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();

    // Verify bookmark exists
    let res = client.get("/api/v1/bookmarks?sender=nanook").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);

    // Delete the room
    client
        .delete(format!("/api/v1/rooms/{room_id}"))
        .header(rocket::http::Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();

    // Bookmark should be gone (CASCADE delete)
    let res = client.get("/api/v1/bookmarks?sender=nanook").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 0);
}

#[test]
fn test_bookmark_dm_room() {
    let client = test_client();

    // Create a DM (creates a DM room)
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "Hey!"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let dm_room_id = body["room_id"].as_str().unwrap();

    // Bookmark the DM room
    let res = client
        .put(format!("/api/v1/rooms/{dm_room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["bookmarked"], true);

    // Should appear in bookmarks list
    let res = client.get("/api/v1/bookmarks?sender=alice").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);
}

#[test]
fn test_bookmark_includes_room_stats() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "bookmark-stats-room");

    // Send a message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "test", "content": "hello world"}"#)
        .dispatch();

    // Bookmark it
    client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "nanook"}"#)
        .dispatch();

    // List bookmarks — should include room stats
    let res = client.get("/api/v1/bookmarks?sender=nanook").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let bookmark = &body["bookmarks"][0];
    assert_eq!(bookmark["room_name"], "bookmark-stats-room");
    assert_eq!(bookmark["message_count"], 1);
    assert!(bookmark["last_activity"].as_str().is_some());
    assert!(bookmark["bookmarked_at"].as_str().is_some());
}

#[test]
fn test_add_bookmark_sender_too_long() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "bookmark-long-sender");

    let long_sender = "x".repeat(101);
    let body = serde_json::json!({"sender": long_sender});
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}
