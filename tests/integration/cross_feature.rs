use rocket::http::{ContentType, Header, Status};
use crate::common::{test_client, create_test_room};

// --- Cross-Feature Interaction Tests ---
// These tests verify behaviors across feature boundaries:
// archive + search, archive + activity, archive + messaging,
// bookmarks + DMs, etc.

// --- Archive + Search ---

#[test]
fn test_search_includes_archived_room_messages() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-searchable");

    // Send a message with unique content
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "archivist", "content": "xylophone orchestration"}"#)
        .dispatch();

    // Archive the room
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Search should still find the message
    let res = client.get("/api/v1/search?q=xylophone").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["results"][0]["content"], "xylophone orchestration");
    assert_eq!(body["results"][0]["room_name"], "archived-searchable");
}

#[test]
fn test_search_with_archived_room_filter() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "search-filter-arch");

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent", "content": "filterable kazoo"}"#)
        .dispatch();

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Search with room_id filter should still work
    let res = client
        .get(format!("/api/v1/search?q=kazoo&room_id={room_id}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
}

// --- Archive + Activity Feed ---

#[test]
fn test_activity_feed_includes_archived_room_messages() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-activity");

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "active-agent", "content": "pre-archive activity"}"#)
        .dispatch();

    // Get message seq for verification
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    let msg_seq = msgs[0]["seq"].as_i64().unwrap();

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Activity feed should still include the message
    let res = client.get("/api/v1/activity?limit=10").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let events = body["events"].as_array().unwrap();
    let found = events.iter().any(|e| e["seq"].as_i64().unwrap() == msg_seq);
    assert!(found, "Message from archived room should appear in activity feed");
}

// --- Archive + Messaging ---

#[test]
fn test_can_send_message_to_archived_room() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-writable");

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Should still be able to send messages
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "post-archive", "content": "message after archive"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["content"], "message after archive");
}

// --- Archive + Files ---

#[test]
fn test_can_upload_file_to_archived_room() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-files");

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Should still be able to upload files
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "uploader", "filename": "test.txt", "content_type": "text/plain", "data": "aGVsbG8="}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let file: serde_json::Value = res.into_json().unwrap();
    assert_eq!(file["filename"], "test.txt");
}

// --- Archive + Reactions ---

#[test]
fn test_can_react_to_message_in_archived_room() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-reactions");

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "reactor", "content": "react to this"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Should still be able to react
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "reactor", "emoji": "👍"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify reaction exists
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let reactions = body["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji"], "👍");
}

// --- Archive + Read Positions ---

#[test]
fn test_can_update_read_position_in_archived_room() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-reads");

    // Send a message to get a seq
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "reader", "content": "read this"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let seq = msg["seq"].as_i64().unwrap();

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Should still be able to mark as read
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/read"))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender": "reader", "last_read_seq": {seq}}}"#))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
}

// --- Archive + Mentions ---

#[test]
fn test_mentions_include_archived_room_messages() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-mentions");

    // Send a message that mentions someone
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "mentioner", "content": "hey @targetagent check this"}"#)
        .dispatch();

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Mentions should still find the message
    let res = client
        .get("/api/v1/mentions?target=targetagent")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let mentions = body["mentions"].as_array().unwrap();
    assert!(mentions.len() >= 1, "Mention in archived room should be found");
    assert!(mentions.iter().any(|m| m["content"].as_str().unwrap().contains("@targetagent")));
}

// --- Archive + Bookmarks ---

#[test]
fn test_can_bookmark_archived_room() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-bookmark");

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Should be able to bookmark an archived room
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bookmarker"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify bookmark in list
    let res = client
        .get("/api/v1/bookmarks?sender=bookmarker")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let bookmarks = body["bookmarks"].as_array().unwrap();
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0]["room_name"], "archived-bookmark");
}

// --- Archive + Pins ---

#[test]
fn test_can_pin_message_in_archived_room() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-pins");

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "pinner", "content": "pin this forever"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Should still be able to pin (admin key still works)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify pin
    let res = client.get(format!("/api/v1/rooms/{room_id}/pins")).dispatch();
    let pins: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(pins.len(), 1);
}

// --- Archive + Incoming Webhooks ---

#[test]
fn test_incoming_webhook_to_archived_room() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "archived-webhook");

    // Create an incoming webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "CI Alerts", "created_by": "admin"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let webhook: serde_json::Value = res.into_json().unwrap();
    let token = webhook["token"].as_str().unwrap();

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Should still be able to post via webhook
    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "CI build passed on archived room"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify message exists
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(msgs.iter().any(|m| m["content"] == "CI build passed on archived room"));
}

// --- DM + Bookmark ---

#[test]
fn test_can_bookmark_dm_room() {
    let client = test_client();

    // Create a DM (which auto-creates a room)
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "hello bob"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let dm: serde_json::Value = res.into_json().unwrap();
    let room_id = dm["room_id"].as_str().unwrap();

    // Bookmark the DM room
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify bookmark exists
    let res = client.get("/api/v1/bookmarks?sender=alice").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let bookmarks = body["bookmarks"].as_array().unwrap();
    assert_eq!(bookmarks.len(), 1);
}

// --- DM + Search ---

#[test]
fn test_search_finds_dm_messages() {
    let client = test_client();

    // Send a DM with unique content
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "quasar nebula discussion"}"#)
        .dispatch();

    // Search should find the DM content
    let res = client.get("/api/v1/search?q=quasar").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["results"][0]["content"], "quasar nebula discussion");
}

// --- DM + Reactions ---

#[test]
fn test_reactions_on_dm_messages() {
    let client = test_client();

    // Send a DM
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "great idea"}"#)
        .dispatch();
    let dm: serde_json::Value = res.into_json().unwrap();
    let room_id = dm["room_id"].as_str().unwrap();
    let msg_id = dm["message"]["id"].as_str().unwrap();

    // Add a reaction
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "emoji": "❤️"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify reaction
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let reactions = body["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji"], "❤️");
    assert!(reactions[0]["senders"].as_array().unwrap().iter().any(|s| s == "bob"));
}

// --- DM + Threading ---

#[test]
fn test_threading_in_dm_rooms() {
    let client = test_client();

    // Send a DM
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "original message"}"#)
        .dispatch();
    let dm: serde_json::Value = res.into_json().unwrap();
    let room_id = dm["room_id"].as_str().unwrap();
    let msg_id = dm["message"]["id"].as_str().unwrap();

    // Reply to the DM message via regular messages API
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender": "bob", "content": "reply to you", "reply_to": "{msg_id}"}}"#))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Get thread
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/thread"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();
    assert_eq!(thread["root"]["content"], "original message");
    assert_eq!(thread["total_replies"].as_i64().unwrap(), 1);
    assert_eq!(thread["replies"][0]["content"], "reply to you");
}

// --- Bookmark + Room Deletion (CASCADE) ---

#[test]
fn test_bookmark_cascade_on_room_delete() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "deletable-bookmarked");

    // Bookmark the room
    client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bookmarker"}"#)
        .dispatch();

    // Verify bookmark exists
    let res = client.get("/api/v1/bookmarks?sender=bookmarker").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["bookmarks"].as_array().unwrap().len(), 1);

    // Delete the room
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Bookmark should be gone (CASCADE)
    let res = client.get("/api/v1/bookmarks?sender=bookmarker").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["bookmarks"].as_array().unwrap().len(), 0);
}

// --- Reaction + Message Edit ---

#[test]
fn test_reactions_persist_through_message_edit() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "react-edit");

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "writer", "content": "original text"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Add a reaction
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "reader", "emoji": "👀"}"#)
        .dispatch();

    // Edit the message
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "writer", "content": "edited text"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Reaction should still exist
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let reactions = body["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji"], "👀");
}

// --- Pin + Message Edit ---

#[test]
fn test_pin_persists_through_message_edit() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "pin-edit");

    // Send and pin a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "author", "content": "pinnable content"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Edit the pinned message
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "author", "content": "edited pinned content"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Pin should still exist and show updated content
    let res = client.get(format!("/api/v1/rooms/{room_id}/pins")).dispatch();
    let pins: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(pins.len(), 1);
    assert_eq!(pins[0]["content"], "edited pinned content");
    assert!(pins[0]["pinned_at"].is_string());
}

// --- Unread + DM ---

#[test]
fn test_unread_counts_include_dm_rooms() {
    let client = test_client();

    // Send a DM
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "msg 1"}"#)
        .dispatch();
    let dm: serde_json::Value = res.into_json().unwrap();
    let room_id = dm["room_id"].as_str().unwrap();

    // Mark bob's read position to 0 (hasn't read anything)
    client
        .put(format!("/api/v1/rooms/{room_id}/read"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "last_read_seq": 0}"#)
        .dispatch();

    // Send another DM
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "msg 2"}"#)
        .dispatch();

    // Check bob's unread - should have 2 messages
    let res = client.get("/api/v1/unread?sender=bob").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["total_unread"].as_i64().unwrap() >= 2);
}

// --- Participants + Profile Enrichment ---

#[test]
fn test_participants_enriched_with_profile_data() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "profile-participants");

    // Create a profile
    client
        .put("/api/v1/profiles/enrich-bot")
        .header(ContentType::JSON)
        .body(r#"{"display_name": "Enrichment Bot", "bio": "I enrich data", "sender_type": "agent"}"#)
        .dispatch();

    // Send a message from that user
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "enrich-bot", "content": "hello from enriched sender"}"#)
        .dispatch();

    // Get participants — should include profile data
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/participants"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0]["sender"], "enrich-bot");
    assert_eq!(participants[0]["display_name"], "Enrichment Bot");
    assert_eq!(participants[0]["bio"], "I enrich data");
}

// --- Read Position + Room Deletion ---

#[test]
fn test_read_positions_cascade_on_room_delete() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "read-pos-cascade");

    // Send a message and mark as read
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "reader", "content": "will be deleted"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let seq = msg["seq"].as_i64().unwrap();

    client
        .put(format!("/api/v1/rooms/{room_id}/read"))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender": "reader", "last_read_seq": {seq}}}"#))
        .dispatch();

    // Delete the room
    client
        .delete(format!("/api/v1/rooms/{room_id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Unread counts should not include deleted room
    let res = client.get("/api/v1/unread?sender=reader").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let rooms = body["rooms"].as_array().unwrap();
    assert!(!rooms.iter().any(|r| r["room_id"] == room_id));
}
