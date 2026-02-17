use rocket::http::{ContentType, Header, Status};
use crate::common::{test_client, create_test_room};

// ============================================================
// Room Creation with Retention Settings (API CRUD)
// ============================================================

#[test]
fn test_create_room_with_max_messages() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-max-msgs", "created_by": "tester", "max_messages": 100}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["max_messages"], 100);
    assert_eq!(body["name"], "retention-max-msgs");
}

#[test]
fn test_create_room_with_max_age() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-max-age", "created_by": "tester", "max_message_age_hours": 24}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["max_message_age_hours"], 24);
}

#[test]
fn test_create_room_with_both_retention() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-both", "created_by": "tester", "max_messages": 500, "max_message_age_hours": 168}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["max_messages"], 500);
    assert_eq!(body["max_message_age_hours"], 168);
}

#[test]
fn test_create_room_without_retention() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-none", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Fields should not be present when null
    assert!(body.get("max_messages").is_none() || body["max_messages"].is_null());
    assert!(body.get("max_message_age_hours").is_none() || body["max_message_age_hours"].is_null());
}

// --- Validation ---

#[test]
fn test_create_room_max_messages_too_low() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-low", "created_by": "tester", "max_messages": 5}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("max_messages"));
}

#[test]
fn test_create_room_max_messages_too_high() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-high", "created_by": "tester", "max_messages": 2000000}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_create_room_max_age_too_low() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-age-low", "created_by": "tester", "max_message_age_hours": 0}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_create_room_max_age_too_high() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-age-high", "created_by": "tester", "max_message_age_hours": 9000}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

// --- Update Retention Settings ---

#[test]
fn test_update_room_set_max_messages() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "retention-update-1");

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"max_messages": 200}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["max_messages"], 200);
}

#[test]
fn test_update_room_set_max_age() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "retention-update-2");

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"max_message_age_hours": 48}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["max_message_age_hours"], 48);
}

#[test]
fn test_update_room_clear_retention() {
    let client = test_client();
    // Create room with retention
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-clear", "created_by": "tester", "max_messages": 100}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap();
    let admin_key = body["admin_key"].as_str().unwrap();
    assert_eq!(body["max_messages"], 100);

    // Clear retention by setting to null
    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"max_messages": null}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["max_messages"].is_null());
}

#[test]
fn test_update_room_invalid_max_messages() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "retention-update-bad");

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"max_messages": 3}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

// --- Get Room shows retention settings ---

#[test]
fn test_get_room_shows_retention() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-get", "created_by": "tester", "max_messages": 50, "max_message_age_hours": 72}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["max_messages"], 50);
    assert_eq!(body["max_message_age_hours"], 72);
}

// --- List Rooms shows retention settings ---

#[test]
fn test_list_rooms_shows_retention() {
    let client = test_client();
    client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-list", "created_by": "tester", "max_messages": 300}"#)
        .dispatch();

    let res = client
        .get("/api/v1/rooms")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let room = rooms.iter().find(|r| r["name"] == "retention-list").unwrap();
    assert_eq!(room["max_messages"], 300);
}

// --- Boundary values ---

#[test]
fn test_create_room_max_messages_min_boundary() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-min-bound", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["max_messages"], 10);
}

#[test]
fn test_create_room_max_age_max_boundary() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "retention-max-bound", "created_by": "tester", "max_message_age_hours": 8760}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["max_message_age_hours"], 8760);
}

// ============================================================
// Actual Pruning Behavior (via admin/retention/run endpoint)
// ============================================================

/// Helper: send N messages to a room, returning their IDs.
fn send_messages(client: &impl std::ops::Deref<Target = rocket::local::blocking::Client>, room_id: &str, count: usize, sender: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for i in 0..count {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "{sender}", "content": "message {i}"}}"#))
            .dispatch();
        assert_eq!(res.status(), Status::Ok, "Failed to send message {i}");
        let body: serde_json::Value = res.into_json().unwrap();
        ids.push(body["id"].as_str().unwrap().to_string());
    }
    ids
}

/// Helper: get all message IDs in a room.
fn get_message_ids(client: &impl std::ops::Deref<Target = rocket::local::blocking::Client>, room_id: &str) -> Vec<String> {
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?limit=1000"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    msgs.iter().map(|m| m["id"].as_str().unwrap().to_string()).collect()
}

/// Helper: trigger retention and return result.
fn trigger_retention(client: &impl std::ops::Deref<Target = rocket::local::blocking::Client>) -> serde_json::Value {
    let res = client
        .post("/api/v1/admin/retention/run")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    res.into_json().unwrap()
}

// --- Count-based pruning ---

#[test]
fn test_retention_prunes_oldest_by_count() {
    let client = test_client();

    // Create room with max_messages=10
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-count", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();

    // Send 15 messages
    let msg_ids = send_messages(&client, &room_id, 15, "alice");
    assert_eq!(msg_ids.len(), 15);

    // Trigger retention
    let result = trigger_retention(&client);
    assert_eq!(result["total_pruned"], 5);
    assert_eq!(result["rooms_checked"], 1);
    let detail = &result["details"][0];
    assert_eq!(detail["pruned_by_count"], 5);
    assert_eq!(detail["pruned_by_age"], 0);

    // Verify: 10 messages remain (the newest 10)
    let remaining = get_message_ids(&client, &room_id);
    assert_eq!(remaining.len(), 10);

    // The oldest 5 should be gone, newest 10 should remain
    for old_id in &msg_ids[..5] {
        assert!(!remaining.contains(old_id), "Old message {old_id} should have been pruned");
    }
    for new_id in &msg_ids[5..] {
        assert!(remaining.contains(new_id), "New message {new_id} should still exist");
    }
}

#[test]
fn test_retention_noop_when_under_limit() {
    let client = test_client();

    // Create room with max_messages=20
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-noop", "created_by": "tester", "max_messages": 20}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();

    // Send only 5 messages (under limit)
    send_messages(&client, &room_id, 5, "bob");

    // Trigger retention
    let result = trigger_retention(&client);
    assert_eq!(result["total_pruned"], 0);

    // All 5 should still exist
    let remaining = get_message_ids(&client, &room_id);
    assert_eq!(remaining.len(), 5);
}

#[test]
fn test_retention_noop_no_retention_rooms() {
    let client = test_client();

    // Create room WITHOUT retention settings
    let (room_id, _) = create_test_room(&client, "no-retention");
    send_messages(&client, &room_id, 10, "carol");

    // Trigger retention — should check 0 rooms
    let result = trigger_retention(&client);
    assert_eq!(result["rooms_checked"], 0);
    assert_eq!(result["total_pruned"], 0);

    // All 10 messages remain
    let remaining = get_message_ids(&client, &room_id);
    assert_eq!(remaining.len(), 10);
}

// --- Pinned messages survive pruning ---

#[test]
fn test_retention_preserves_pinned_messages() {
    let client = test_client();

    // Create room with max_messages=10
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-pins", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();
    let admin_key = body["admin_key"].as_str().unwrap().to_string();

    // Send 15 messages
    let msg_ids = send_messages(&client, &room_id, 15, "alice");

    // Pin message #2 (one of the oldest that would normally be pruned)
    let pin_res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{}/pin", msg_ids[1]))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(pin_res.status(), Status::Ok);

    // Trigger retention
    let result = trigger_retention(&client);
    // Should prune 4 (not 5) — the pinned message is exempt
    // 15 total, 1 pinned (exempt), 14 non-pinned, keep 10 non-pinned = prune 4
    assert_eq!(result["total_pruned"], 4);

    // Verify: 11 messages remain (10 non-pinned + 1 pinned)
    let remaining = get_message_ids(&client, &room_id);
    assert_eq!(remaining.len(), 11);

    // The pinned message (#2, index 1) must still exist
    assert!(remaining.contains(&msg_ids[1]), "Pinned message should survive retention");

    // Verify it's still pinned via pins endpoint
    let pins_res = client
        .get(format!("/api/v1/rooms/{room_id}/pins"))
        .dispatch();
    assert_eq!(pins_res.status(), Status::Ok);
    let pins: Vec<serde_json::Value> = pins_res.into_json().unwrap();
    assert_eq!(pins.len(), 1);
    assert_eq!(pins[0]["id"].as_str().unwrap(), msg_ids[1]);
}

// --- Reactions cascade with pruned messages ---

#[test]
fn test_retention_cascades_reactions() {
    let client = test_client();

    // Create room with max_messages=10
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-reactions", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();

    // Send 15 messages
    let msg_ids = send_messages(&client, &room_id, 15, "alice");

    // Add reactions to message #0 (will be pruned) and #14 (will survive)
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{}/reactions", msg_ids[0]))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "emoji": "👍"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{}/reactions", msg_ids[14]))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "emoji": "❤️"}"#)
        .dispatch();

    // Trigger retention
    trigger_retention(&client);

    // Verify: reaction on pruned message is gone (404 for the message)
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{}/reactions", msg_ids[0]))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);

    // Verify: reaction on surviving message is intact
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{}/reactions", msg_ids[14]))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let reactions = body["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji"], "❤️");
}

// --- FTS index cleanup ---

#[test]
fn test_retention_cleans_fts_index() {
    let client = test_client();

    // Create room with max_messages=10
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-fts", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();

    // Send messages with distinctive content (standalone word for FTS5 word matching)
    for i in 0..5 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "alice", "content": "xylophone unique old content number {i}"}}"#))
            .dispatch();
    }
    // Send 10 more (non-xylophone) to push older ones past retention
    for i in 0..10 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "alice", "content": "zeppelin recent content number {i}"}}"#))
            .dispatch();
    }

    // Before pruning: search finds xylophone messages
    let res = client
        .get(format!("/api/v1/search?q=xylophone&room_id={room_id}"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 5, "Should find 5 xylophone messages before pruning");

    // Trigger retention
    trigger_retention(&client);

    // After pruning: xylophone messages should not be searchable
    let res = client
        .get(format!("/api/v1/search?q=xylophone&room_id={room_id}"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 0, "Pruned messages should not appear in FTS search");

    // Verify newer messages still searchable
    let res = client
        .get(format!("/api/v1/search?q=zeppelin&room_id={room_id}"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 10, "Newer messages should still be searchable");
}

// --- Thread integrity after pruning ---

#[test]
fn test_retention_thread_reply_survives_parent_prune() {
    let client = test_client();

    // Create room with max_messages=10
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-threads", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();

    // Send a "parent" message (will be pruned)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "parent message"}"#)
        .dispatch();
    let parent_id = res.into_json::<serde_json::Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send 9 filler messages
    send_messages(&client, &room_id, 9, "bob");

    // Send a reply to the parent (this will be among the newest 10)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender": "carol", "content": "reply to parent", "reply_to": "{parent_id}"}}"#))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let reply_id = res.into_json::<serde_json::Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send 1 more to push parent past limit (12 total, keep 10)
    send_messages(&client, &room_id, 1, "dave");

    // Before pruning: 12 messages
    let before = get_message_ids(&client, &room_id);
    assert_eq!(before.len(), 12);

    // Trigger retention — should prune 2 oldest (including parent)
    let result = trigger_retention(&client);
    assert_eq!(result["total_pruned"], 2);

    // After pruning: 10 messages remain
    let after = get_message_ids(&client, &room_id);
    assert_eq!(after.len(), 10);

    // Parent should be gone
    assert!(!after.contains(&parent_id), "Parent should be pruned");

    // Reply should still exist with its reply_to field intact
    assert!(after.contains(&reply_id), "Reply should survive");
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?limit=100"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    let reply = msgs.iter().find(|m| m["id"].as_str().unwrap() == reply_id).unwrap();
    // reply_to still points to the deleted parent (soft reference)
    assert_eq!(reply["reply_to"].as_str().unwrap(), parent_id);
}

// --- Multiple rooms with different settings ---

#[test]
fn test_retention_multiple_rooms_independent() {
    let client = test_client();

    // Room A: max_messages=10
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-multi-a", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    let body_a: serde_json::Value = res.into_json().unwrap();
    let room_a = body_a["id"].as_str().unwrap().to_string();

    // Room B: max_messages=15
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-multi-b", "created_by": "tester", "max_messages": 15}"#)
        .dispatch();
    let body_b: serde_json::Value = res.into_json().unwrap();
    let room_b = body_b["id"].as_str().unwrap().to_string();

    // Send 20 to room A, 20 to room B
    send_messages(&client, &room_a, 20, "alice");
    send_messages(&client, &room_b, 20, "bob");

    // Trigger retention
    let result = trigger_retention(&client);
    assert_eq!(result["rooms_checked"], 2);
    // Room A: 20 - 10 = 10 pruned. Room B: 20 - 15 = 5 pruned.
    assert_eq!(result["total_pruned"], 15);

    let remaining_a = get_message_ids(&client, &room_a);
    assert_eq!(remaining_a.len(), 10);

    let remaining_b = get_message_ids(&client, &room_b);
    assert_eq!(remaining_b.len(), 15);
}

// --- Idempotent: running twice with no new messages prunes nothing ---

#[test]
fn test_retention_idempotent_second_run() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-idem", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();

    send_messages(&client, &room_id, 15, "alice");

    // First run: prunes 5
    let result1 = trigger_retention(&client);
    assert_eq!(result1["total_pruned"], 5);

    // Second run: nothing to prune
    let result2 = trigger_retention(&client);
    assert_eq!(result2["total_pruned"], 0);
    assert_eq!(result2["rooms_checked"], 1);

    // Still 10 messages
    let remaining = get_message_ids(&client, &room_id);
    assert_eq!(remaining.len(), 10);
}

// --- Admin endpoint response shape ---

#[test]
fn test_retention_run_response_shape() {
    let client = test_client();

    let result = trigger_retention(&client);
    assert!(result.get("rooms_checked").is_some());
    assert!(result.get("total_pruned").is_some());
    assert!(result.get("details").is_some());
    assert!(result["details"].is_array());
}

#[test]
fn test_retention_run_detail_fields() {
    let client = test_client();

    // Create room with retention to ensure details are populated
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-detail", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();
    send_messages(&client, &room_id, 12, "alice");

    let result = trigger_retention(&client);
    let detail = &result["details"][0];
    assert_eq!(detail["room_id"].as_str().unwrap(), room_id);
    assert!(detail.get("pruned_by_count").is_some());
    assert!(detail.get("pruned_by_age").is_some());
    assert!(detail.get("total").is_some());
    assert_eq!(detail["total"], 2);
}

// --- Retention + read positions ---

#[test]
fn test_retention_read_positions_survive() {
    let client = test_client();

    // Create room with max_messages=10
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "prune-readpos", "created_by": "tester", "max_messages": 10}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let room_id = body["id"].as_str().unwrap().to_string();

    // Send 15 messages
    send_messages(&client, &room_id, 15, "alice");

    // Mark read position at seq 5 (will be pruned) and seq 12 (will survive)
    client
        .put(format!("/api/v1/rooms/{room_id}/read"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "last_read_seq": 5}"#)
        .dispatch();
    client
        .put(format!("/api/v1/rooms/{room_id}/read"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "carol", "last_read_seq": 12}"#)
        .dispatch();

    // Trigger retention
    trigger_retention(&client);

    // Read positions should survive (they reference seq numbers, not message IDs)
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/read"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let positions: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(positions.len(), 2);

    let bob_pos = positions.iter().find(|p| p["sender"] == "bob").unwrap();
    assert_eq!(bob_pos["last_read_seq"], 5);
    let carol_pos = positions.iter().find(|p| p["sender"] == "carol").unwrap();
    assert_eq!(carol_pos["last_read_seq"], 12);
}
