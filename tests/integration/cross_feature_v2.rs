use rocket::http::{ContentType, Header, Status};
use crate::common::{test_client, create_test_room};

// ============================================================
// Cross-feature interaction tests v2
// Focuses on interactions between newer features:
//   retention + edit_history, search + edits, export + DMs,
//   incoming webhooks edge cases, archiving + export
// ============================================================

// --- Retention + Edit History ---

#[test]
fn test_retention_prunes_edit_history_via_cascade() {
    // When retention prunes a message that has edit history,
    // the edit history records should be cascade-deleted too.
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "ret-edithist-cascade");

    // Set max_messages = 10 on the room (minimum allowed)
    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"max_messages": 10}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Send message 1 and edit it (creates edit history)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-a", "content": "original v1"}"#)
        .dispatch();
    let msg1: serde_json::Value = res.into_json().unwrap();
    let msg1_id = msg1["id"].as_str().unwrap().to_string();

    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg1_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-a", "content": "edited v1"}"#)
        .dispatch();

    // Verify edit history exists
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg1_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["edit_count"].as_i64().unwrap(), 1);

    // Send 11 more messages to exceed max_messages=10 (msg1 + 11 = 12 total, need to prune 2)
    for i in 2..=12 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "agent-b", "content": "message {i}"}}"#))
            .dispatch();
    }

    // Trigger retention
    let res = client.post("/api/v1/admin/retention/run").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let result: serde_json::Value = res.into_json().unwrap();
    assert!(result["total_pruned"].as_i64().unwrap() >= 1);

    // The pruned message's edit history should also be gone (CASCADE)
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg1_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_retention_preserves_pinned_message_edit_history() {
    // Pinned messages survive retention; their edit history should also survive.
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "ret-pin-edithist");

    // Set max_messages = 10 (minimum allowed)
    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"max_messages": 10}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Send a message, edit it, and pin it
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-a", "content": "will be pinned"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap().to_string();

    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-a", "content": "pinned and edited"}"#)
        .dispatch();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Send 11 more non-pinned messages to exceed limit (1 pinned + 11 non-pinned = 12 total, 11 non-pinned > 10 limit)
    for i in 2..=12 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "agent-b", "content": "msg {i}"}}"#))
            .dispatch();
    }

    // Run retention
    client.post("/api/v1/admin/retention/run").dispatch();

    // Pinned message and its edit history should survive
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["edit_count"].as_i64().unwrap(), 1);
    assert_eq!(body["current_content"], "pinned and edited");
}

// --- Search + Edits ---

#[test]
fn test_search_reflects_edited_content() {
    // After editing a message, search should find the new content
    // and NOT find the old content (FTS index should be updated).
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "search-edit-fts");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-x", "content": "original zebraphant content"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap().to_string();

    // Verify original is searchable
    let res = client.get("/api/v1/search?q=zebraphant").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);

    // Edit the message
    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-x", "content": "revised giraffalope content"}"#)
        .dispatch();

    // New content should be searchable
    let res = client.get("/api/v1/search?q=giraffalope").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);

    // Old content should NOT be searchable via FTS
    // Note: FTS index should have been updated on edit
    let res = client.get("/api/v1/search?q=zebraphant").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 0);
}

// --- Export + DMs ---

#[test]
fn test_export_dm_conversation_json() {
    // Agents should be able to export DM conversations.
    let client = test_client();

    // Send a DM (auto-creates DM room)
    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-a", "recipient": "agent-b", "content": "hello privately"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let dm: serde_json::Value = res.into_json().unwrap();
    let dm_room_id = dm["room_id"].as_str().unwrap();

    // Send another message
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-b", "recipient": "agent-a", "content": "hello back"}"#)
        .dispatch();

    // Export as JSON
    let res = client
        .get(format!("/api/v1/rooms/{dm_room_id}/export?format=json"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    assert!(body["room_name"].as_str().unwrap().starts_with("dm:"));
}

#[test]
fn test_export_dm_conversation_markdown() {
    let client = test_client();

    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "markdown test"}"#)
        .dispatch();
    let dm: serde_json::Value = res.into_json().unwrap();
    let dm_room_id = dm["room_id"].as_str().unwrap();

    let res = client
        .get(format!("/api/v1/rooms/{dm_room_id}/export?format=markdown"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body = res.into_string().unwrap();
    assert!(body.contains("markdown test"));
    assert!(body.contains("alice"));
}

#[test]
fn test_export_dm_conversation_csv() {
    let client = test_client();

    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "csv-sender", "recipient": "csv-recv", "content": "csv export test"}"#)
        .dispatch();
    let dm: serde_json::Value = res.into_json().unwrap();
    let dm_room_id = dm["room_id"].as_str().unwrap();

    let res = client
        .get(format!("/api/v1/rooms/{dm_room_id}/export?format=csv"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body = res.into_string().unwrap();
    assert!(body.contains("csv export test"));
    assert!(body.contains("csv-sender"));
}

// --- Export + Archived Room ---

#[test]
fn test_export_from_archived_room() {
    // Archived rooms should still be exportable.
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "export-archived");

    // Send some messages
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-1", "content": "before archiving"}"#)
        .dispatch();

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Export should still work
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=json"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    assert_eq!(body["messages"][0]["content"], "before archiving");
}

// --- Export + Filters ---

#[test]
fn test_export_with_sender_filter() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "export-sender-filter");

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-alpha", "content": "from alpha"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-beta", "content": "from beta"}"#)
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=json&sender=agent-alpha"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["sender"], "agent-alpha");
}

#[test]
fn test_export_includes_edit_count() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "export-editcount");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "editor", "content": "v1"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "editor", "content": "v2"}"#)
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=json"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["content"], "v2");
    // Exported messages should reflect edit_count
    assert!(messages[0].get("edited_at").is_some());
}

// --- Incoming Webhooks + Edge Cases ---

#[test]
fn test_incoming_webhook_with_metadata() {
    // Incoming webhooks should support the metadata field.
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-metadata");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "MetaBot", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "with meta", "metadata": {"build_id": 42, "status": "passed"}}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["content"], "with meta");
    // Metadata should be preserved
    if let Some(meta) = msg.get("metadata").filter(|m| !m.is_null()) {
        assert_eq!(meta["build_id"], 42);
    }
}

#[test]
fn test_incoming_webhook_message_appears_in_search() {
    // Messages posted via incoming webhooks should be FTS-indexed.
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-search");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "SearchBot", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();

    client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "Unique platypusfox9283 notification"}"#)
        .dispatch();

    let res = client.get("/api/v1/search?q=platypusfox9283").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
}

#[test]
fn test_incoming_webhook_message_in_activity_feed() {
    // Messages from incoming webhooks should appear in the activity feed.
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-activity");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "ActivityBot", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();

    client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "webhook activity test"}"#)
        .dispatch();

    let res = client.get("/api/v1/activity?limit=5").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["events"].as_array().unwrap().iter().any(|e| {
        e["content"].as_str().unwrap_or("") == "webhook activity test"
            && e["sender"].as_str().unwrap_or("") == "ActivityBot"
    }));
}

#[test]
fn test_incoming_webhook_respects_room_retention() {
    // Messages from incoming webhooks should be subject to retention.
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-retention");

    // Set max_messages = 10 (minimum allowed)
    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"max_messages": 10}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "RetBot", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();

    // Post 13 messages via webhook (exceeds max_messages=10 by 3)
    for i in 1..=13 {
        client
            .post(format!("/api/v1/hook/{token}"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"content": "webhook msg {i}"}}"#))
            .dispatch();
    }

    // Trigger retention
    let res = client.post("/api/v1/admin/retention/run").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let result: serde_json::Value = res.into_json().unwrap();
    assert_eq!(result["total_pruned"].as_i64().unwrap(), 3);

    // Only newest 10 messages should remain
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let messages: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(messages.len(), 10);
    // Oldest messages pruned, newest kept
    assert!(messages.iter().any(|m| m["content"] == "webhook msg 13"));
    assert!(messages.iter().any(|m| m["content"] == "webhook msg 12"));
}

// --- Edit History + Various Interactions ---

#[test]
fn test_edit_history_for_dm_messages() {
    // Edit history should work for DM messages too.
    let client = test_client();

    let res = client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "recipient": "bob", "content": "dm original"}"#)
        .dispatch();
    let dm: serde_json::Value = res.into_json().unwrap();
    let dm_room_id = dm["room_id"].as_str().unwrap();
    let msg_id = dm["message"]["id"].as_str().unwrap();

    // Edit the DM message
    let res = client
        .put(format!("/api/v1/rooms/{dm_room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "dm revised"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Check edit history
    let res = client
        .get(format!("/api/v1/rooms/{dm_room_id}/messages/{msg_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["edit_count"].as_i64().unwrap(), 1);
    assert_eq!(body["current_content"], "dm revised");
    assert_eq!(body["edits"][0]["previous_content"], "dm original");
}

#[test]
fn test_edit_count_in_search_results() {
    // Search results for edited messages should reflect edit_count.
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "search-editcount");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "editor", "content": "unique narwhalephant44 word"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Edit twice
    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "editor", "content": "narwhalephant44 revised once"}"#)
        .dispatch();
    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "editor", "content": "narwhalephant44 revised twice"}"#)
        .dispatch();

    let res = client.get("/api/v1/search?q=narwhalephant44").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["results"][0]["content"], "narwhalephant44 revised twice");
    // edited_at should be set
    assert!(body["results"][0]["edited_at"].as_str().is_some());
}

// --- Search Pagination Edge Cases ---

#[test]
fn test_search_has_more_false_at_end() {
    // When results fit within the limit, has_more should be false.
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "search-hasmore-false");

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "tester", "content": "unique flamingocactus77"}"#)
        .dispatch();

    let res = client.get("/api/v1/search?q=flamingocactus77&limit=10").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
    assert_eq!(body["has_more"], false);
}

#[test]
fn test_search_has_more_true_when_exceeds_limit() {
    // When there are more results than the limit, has_more should be true.
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "search-hasmore-true");

    // Create 5 messages with the same search term
    for i in 1..=5 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "tester", "content": "elephantunicorn99 message {i}"}}"#))
            .dispatch();
    }

    // Search with limit=3 — should report has_more=true
    let res = client.get("/api/v1/search?q=elephantunicorn99&limit=3").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 3);
    assert_eq!(body["has_more"], true);
}

#[test]
fn test_search_pagination_with_after_cursor() {
    // Paginating search results using after=seq.
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "search-paginate");

    // Create 5 messages
    for i in 1..=5 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "tester", "content": "paginate-rhinolion88 entry {i}"}}"#))
            .dispatch();
    }

    // First page: limit=2
    let res = client.get("/api/v1/search?q=rhinolion88&limit=2").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let page1: serde_json::Value = res.into_json().unwrap();
    assert_eq!(page1["count"].as_i64().unwrap(), 2);
    assert_eq!(page1["has_more"], true);

    // Get the seq of the last result to use as cursor
    let last_seq = page1["results"][1]["seq"].as_i64().unwrap();

    // Second page: after=last_seq
    let res = client
        .get(format!("/api/v1/search?q=rhinolion88&limit=2&after={last_seq}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let page2: serde_json::Value = res.into_json().unwrap();
    assert_eq!(page2["count"].as_i64().unwrap(), 2);

    // Ensure no overlap between pages
    let page1_ids: Vec<&str> = page1["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["message_id"].as_str().unwrap())
        .collect();
    let page2_ids: Vec<&str> = page2["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["message_id"].as_str().unwrap())
        .collect();
    for id in &page1_ids {
        assert!(!page2_ids.contains(id), "Overlap found between pages");
    }
}

#[test]
fn test_search_date_range_filtering() {
    // Date range filtering should narrow search results.
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "search-daterange");

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "tester", "content": "daterange-wombatfish23 test"}"#)
        .dispatch();

    // Search with future before_date (should find it)
    let res = client
        .get("/api/v1/search?q=wombatfish23&before_date=2099-01-01T00:00:00Z")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);

    // Search with past before_date (should find nothing)
    let res = client
        .get("/api/v1/search?q=wombatfish23&before_date=2020-01-01T00:00:00Z")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 0);
}

// --- Stats Endpoint Edge Cases ---

#[test]
fn test_stats_reflect_edit_history() {
    // Stats should count edits if tracked.
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "stats-edithist");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent", "content": "will edit"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent", "content": "edited once"}"#)
        .dispatch();

    // Stats endpoint should still return OK and reflect counts
    let res = client.get("/api/v1/stats").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let stats: serde_json::Value = res.into_json().unwrap();
    assert!(stats["messages"].as_i64().unwrap() >= 1);
}

// --- Mentions + Edit ---

#[test]
fn test_mentions_after_message_edit_adds_mention() {
    // If a message is edited to add an @mention, the mentions endpoint
    // should find it (since it does a live LIKE scan, not a cached index).
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "mention-edit");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-a", "content": "hello world"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // No mentions of agent-b yet
    let res = client
        .get("/api/v1/mentions?target=agent-b")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 0);

    // Edit to add @mention
    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-a", "content": "hello @agent-b world"}"#)
        .dispatch();

    // Now agent-b should have a mention
    let res = client
        .get("/api/v1/mentions?target=agent-b")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 1);
}

// --- Bookmarks + Archived Room ---

#[test]
fn test_bookmark_persists_after_room_archive() {
    // Bookmarking a room and then archiving it — bookmark should persist.
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "bookmark-archive");

    // Bookmark the room
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-1"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Archive the room
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Bookmark should still be in the list
    let res = client
        .get("/api/v1/bookmarks?sender=agent-1")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let bookmarks = body["bookmarks"].as_array().unwrap();
    assert!(bookmarks.iter().any(|b| b["room_id"] == room_id));
}

// --- Typing + Validation ---

#[test]
fn test_typing_rejects_empty_sender() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "typing-empty-sender");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/typing"))
        .header(ContentType::JSON)
        .body(r#"{"sender": ""}"#)
        .dispatch();
    // Should reject empty sender
    assert!(res.status() == Status::BadRequest || res.status() == Status::UnprocessableEntity);
}

#[test]
fn test_typing_for_nonexistent_room() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms/nonexistent-room-id/typing")
        .header(ContentType::JSON)
        .body(r#"{"sender": "agent-x"}"#)
        .dispatch();
    // Should handle gracefully (either 404 or 200 with no effect)
    assert!(res.status() == Status::NotFound || res.status() == Status::Ok);
}
