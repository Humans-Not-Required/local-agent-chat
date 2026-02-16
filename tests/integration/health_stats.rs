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

    // Verify expected fields exist
    assert!(body["rooms"].is_number());
    assert!(body["messages"].is_number());
    assert!(body["by_sender_type"].is_object());
    assert!(body["active_senders_1h"].is_number());
    assert!(body["active_by_type_1h"].is_object());
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
