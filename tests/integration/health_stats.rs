use rocket::http::{ContentType, Header, Status};
use crate::common::test_client;

// --- Health ---

#[test]
fn test_health() {
    let client = test_client();
    let res = client.get("/api/v1/health").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "local-agent-chat");
}

// --- Stats ---

#[test]
fn test_stats() {
    let client = test_client();
    let res = client.get("/api/v1/stats").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["rooms"].as_i64().unwrap() >= 1); // general room
    assert!(body["messages"].as_i64().unwrap() >= 0);
}

// --- Enhanced stats ---

#[test]
fn test_stats_sender_type_breakdown() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send messages with different types
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Agent1","content":"agent msg 1","sender_type":"agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Agent2","content":"agent msg 2","sender_type":"agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Human1","content":"human msg","sender_type":"human"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"Anon","content":"no type"}"#)
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    assert_eq!(body["by_sender_type"]["agent"].as_i64().unwrap(), 2);
    assert_eq!(body["by_sender_type"]["human"].as_i64().unwrap(), 1);
    assert_eq!(body["by_sender_type"]["unspecified"].as_i64().unwrap(), 1);
    assert!(body["active_by_type_1h"]["agents"].as_i64().unwrap() >= 2);
    assert!(body["active_by_type_1h"]["humans"].as_i64().unwrap() >= 1);
}

// --- Stats update after deletions ---

#[test]
fn test_stats_update_after_message_deletion() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "stats-delete-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Send 3 messages
    for i in 1..=3 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "Msg {i}"}}"#))
            .dispatch();
    }

    // Verify initial count
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let room_detail: serde_json::Value = res.into_json().unwrap();
    assert_eq!(room_detail["message_count"].as_i64().unwrap(), 3);

    // Get message IDs
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    let msg_id = msgs[0]["id"].as_str().unwrap();

    // Delete one message using admin key
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}?sender=bot"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify count decreased
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
}

// --- Health response fields ---

#[test]
fn test_health_response_fields() {
    let client = test_client();
    let res = client.get("/api/v1/health").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "local-agent-chat");
    assert!(body["version"].is_string());
}

// --- Stats reflect DM messages ---

#[test]
fn test_stats_include_dm_messages() {
    let client = test_client();

    // Initial stats
    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let msgs_before = before["messages"].as_i64().unwrap();

    // Send a DM
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","recipient":"bob","content":"DM counted in stats"}"#)
        .dispatch();

    // Stats should increase
    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["messages"].as_i64().unwrap(), msgs_before + 1);
}

// --- Stats response structure ---

#[test]
fn test_stats_response_structure() {
    let client = test_client();

    let res = client.get("/api/v1/stats").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    // Core fields
    assert!(body["rooms"].is_number());
    assert!(body["rooms_archived"].is_number());
    assert!(body["messages"].is_number());
    assert!(body["by_sender_type"].is_object());
    assert!(body["active_senders_1h"].is_number());
    assert!(body["active_by_type_1h"].is_object());

    // DM stats
    assert!(body["dms"].is_object());
    assert!(body["dms"]["conversations"].is_number());
    assert!(body["dms"]["messages"].is_number());

    // File stats
    assert!(body["files"].is_object());
    assert!(body["files"]["count"].is_number());
    assert!(body["files"]["total_bytes"].is_number());

    // Feature counts
    assert!(body["profiles"].is_number());
    assert!(body["reactions"].is_number());
    assert!(body["pins"].is_number());
    assert!(body["threads"].is_number());
    assert!(body["bookmarks"].is_number());

    // Webhook stats
    assert!(body["webhooks"].is_object());
    assert!(body["webhooks"]["outgoing"].is_number());
    assert!(body["webhooks"]["outgoing_active"].is_number());
    assert!(body["webhooks"]["incoming"].is_number());
    assert!(body["webhooks"]["deliveries_24h"].is_number());
    assert!(body["webhooks"]["delivery_successes_24h"].is_number());
    assert!(body["webhooks"]["delivery_failures_24h"].is_number());
}

// --- Stats DM breakdown ---

#[test]
fn test_stats_dm_breakdown() {
    let client = test_client();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let dm_convos_before = before["dms"]["conversations"].as_i64().unwrap();

    // Create a DM conversation
    client
        .post("/api/v1/dm")
        .header(ContentType::JSON)
        .body(r#"{"sender":"dmstat-alice","recipient":"dmstat-bob","content":"dm stats test"}"#)
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["dms"]["conversations"].as_i64().unwrap(), dm_convos_before + 1);
    assert!(after["dms"]["messages"].as_i64().unwrap() >= 1);
}

// --- Stats file tracking ---

#[test]
fn test_stats_file_tracking() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let files_before = before["files"]["count"].as_i64().unwrap();
    let bytes_before = before["files"]["total_bytes"].as_i64().unwrap();

    // Upload a small file (base64 of "hello world" = "aGVsbG8gd29ybGQ=", 11 bytes)
    client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"filebot","filename":"test.txt","content_type":"text/plain","data":"aGVsbG8gd29ybGQ="}"#)
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["files"]["count"].as_i64().unwrap(), files_before + 1);
    assert!(after["files"]["total_bytes"].as_i64().unwrap() > bytes_before);
}

// --- Stats reaction count ---

#[test]
fn test_stats_reaction_count() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send a message to react to
    let msg_res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"reactor","content":"react to me"}"#)
        .dispatch();
    let msg: serde_json::Value = msg_res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let reactions_before = before["reactions"].as_i64().unwrap();

    // Add a reaction
    client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/reactions"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"reactor","emoji":"👍"}"#)
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["reactions"].as_i64().unwrap(), reactions_before + 1);
}

// --- Stats thread count ---

#[test]
fn test_stats_thread_count() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Send a parent message
    let msg_res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"threadbot","content":"thread root"}"#)
        .dispatch();
    let msg: serde_json::Value = msg_res.into_json().unwrap();
    let parent_id = msg["id"].as_str().unwrap();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let threads_before = before["threads"].as_i64().unwrap();

    // Send a reply
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender":"threadbot","content":"reply","reply_to":"{parent_id}"}}"#))
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["threads"].as_i64().unwrap(), threads_before + 1);
}

// --- Stats bookmark count ---

#[test]
fn test_stats_bookmark_count() {
    let client = test_client();

    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let bookmarks_before = before["bookmarks"].as_i64().unwrap();

    // Add a bookmark
    client
        .put(format!("/api/v1/rooms/{room_id}/bookmark"))
        .header(ContentType::JSON)
        .body(r#"{"sender":"bookmarkbot"}"#)
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["bookmarks"].as_i64().unwrap(), bookmarks_before + 1);
}

// --- Stats archived rooms ---

#[test]
fn test_stats_archived_rooms() {
    let client = test_client();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let archived_before = before["rooms_archived"].as_i64().unwrap();

    // Create and archive a room
    let room_res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"archive-stats-test"}"#)
        .dispatch();
    let room: serde_json::Value = room_res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["rooms_archived"].as_i64().unwrap(), archived_before + 1);
}

// --- Stats webhook counts ---

#[test]
fn test_stats_webhook_counts() {
    let client = test_client();

    // Create a room with webhooks
    let room_res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"webhook-stats-test"}"#)
        .dispatch();
    let room: serde_json::Value = room_res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let outgoing_before = before["webhooks"]["outgoing"].as_i64().unwrap();
    let incoming_before = before["webhooks"]["incoming"].as_i64().unwrap();

    // Create an outgoing webhook
    client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url":"http://example.com/hook","events":"*","created_by":"statsbot"}"#)
        .dispatch();

    // Create an incoming webhook
    client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name":"Stats Hook","created_by":"statsbot"}"#)
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["webhooks"]["outgoing"].as_i64().unwrap(), outgoing_before + 1);
    assert_eq!(after["webhooks"]["outgoing_active"].as_i64().unwrap(), outgoing_before + 1);
    assert_eq!(after["webhooks"]["incoming"].as_i64().unwrap(), incoming_before + 1);
}

// --- Stats profile count ---

#[test]
fn test_stats_profile_count() {
    let client = test_client();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let profiles_before = before["profiles"].as_i64().unwrap();

    // Create a profile
    client
        .put("/api/v1/profiles/stats-test-agent")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Stats Test","sender_type":"agent","bio":"testing stats"}"#)
        .dispatch();

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["profiles"].as_i64().unwrap(), profiles_before + 1);
}

// --- Stats with multiple rooms ---

#[test]
fn test_stats_count_multiple_rooms() {
    let client = test_client();

    let res = client.get("/api/v1/stats").dispatch();
    let before: serde_json::Value = res.into_json().unwrap();
    let rooms_before = before["rooms"].as_i64().unwrap();

    // Create 3 new rooms
    for i in 1..=3 {
        client
            .post("/api/v1/rooms")
            .header(ContentType::JSON)
            .body(format!(r#"{{"name":"stats-room-{i}"}}"#))
            .dispatch();
    }

    let res = client.get("/api/v1/stats").dispatch();
    let after: serde_json::Value = res.into_json().unwrap();
    assert_eq!(after["rooms"].as_i64().unwrap(), rooms_before + 3);
}
