use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Room Participants ---

#[test]
fn test_participants_empty_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "empty-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/participants"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(participants.len(), 0);
}

#[test]
fn test_participants_nonexistent_room() {
    let client = test_client();
    let res = client
        .get("/api/v1/rooms/nonexistent-uuid/participants")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_participants_basic() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "test-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send messages from different senders
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Alice", "content": "Hello", "sender_type": "human"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Bob", "content": "Hi", "sender_type": "agent"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Alice", "content": "How are you?"}"#)
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/participants"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(participants.len(), 2);

    // Sorted by last_seen DESC — Alice sent the last message so should be first
    assert_eq!(participants[0]["sender"].as_str().unwrap(), "Alice");
    assert_eq!(participants[0]["message_count"].as_i64().unwrap(), 2);
    assert_eq!(participants[0]["sender_type"].as_str().unwrap(), "human");
    assert!(participants[0]["first_seen"].is_string());
    assert!(participants[0]["last_seen"].is_string());

    assert_eq!(participants[1]["sender"].as_str().unwrap(), "Bob");
    assert_eq!(participants[1]["message_count"].as_i64().unwrap(), 1);
    assert_eq!(participants[1]["sender_type"].as_str().unwrap(), "agent");
}

#[test]
fn test_participants_sender_type_uses_latest() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "type-change-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // First message as agent
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Charlie", "content": "I'm an agent", "sender_type": "agent"}"#)
        .dispatch();

    // Second message as human (changed)
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Charlie", "content": "Actually I'm human", "sender_type": "human"}"#)
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/participants"))
        .dispatch();
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(participants.len(), 1);
    // Should use the latest sender_type
    assert_eq!(participants[0]["sender_type"].as_str().unwrap(), "human");
    assert_eq!(participants[0]["message_count"].as_i64().unwrap(), 2);
}

// --- Exclude Sender Filter ---

#[test]
fn test_exclude_sender_single() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Three senders
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Hello from Nanook"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Hello from Forge"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "Hello from Drift"}"#)
        .dispatch();

    // Exclude Forge
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?exclude_sender=Forge"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
    assert!(
        msgs.iter()
            .all(|m| m["sender"].as_str().unwrap() != "Forge")
    );
    assert!(
        msgs.iter()
            .any(|m| m["sender"].as_str().unwrap() == "Nanook")
    );
    assert!(
        msgs.iter()
            .any(|m| m["sender"].as_str().unwrap() == "Drift")
    );
}

#[test]
fn test_exclude_sender_multiple_comma_separated() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-multi"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "From Nanook"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "From Forge"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "From Drift"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Lux", "content": "From Lux"}"#)
        .dispatch();

    // Exclude Forge and Drift (comma-separated)
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?exclude_sender=Forge,Drift"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
    let senders: Vec<&str> = msgs.iter().map(|m| m["sender"].as_str().unwrap()).collect();
    assert!(senders.contains(&"Nanook"));
    assert!(senders.contains(&"Lux"));
    assert!(!senders.contains(&"Forge"));
    assert!(!senders.contains(&"Drift"));
}

#[test]
fn test_exclude_sender_with_after_filter() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-after"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send messages and track seq
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "First"}"#)
        .dispatch();
    let msg1: serde_json::Value = res.into_json().unwrap();
    let seq1 = msg1["seq"].as_i64().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Second"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Third"}"#)
        .dispatch();

    // after=seq1, exclude Forge — should only get Nanook's "Third"
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={seq1}&exclude_sender=Forge"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["sender"].as_str().unwrap(), "Nanook");
    assert_eq!(msgs[0]["content"].as_str().unwrap(), "Third");
}

#[test]
fn test_exclude_sender_activity_feed() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-activity"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Activity from Nanook"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "Activity from Forge"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "Activity from Drift"}"#)
        .dispatch();

    // Exclude Forge from activity feed
    let res = client
        .get(format!(
            "/api/v1/activity?room_id={room_id}&exclude_sender=Forge"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let events = body["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|e| e["sender"].as_str().unwrap() != "Forge")
    );
}

#[test]
fn test_exclude_sender_empty_string_ignored() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "exclude-empty"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Hello"}"#)
        .dispatch();

    // Empty exclude_sender should return all messages
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?exclude_sender="))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
}

// ===== Message Search Tests =====

#[test]
fn test_search_basic() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-basic"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "The weather is cold today"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "I am building something new"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Drift", "content": "The cold never bothered me"}"#)
        .dispatch();

    // Search for "cold" — should find 2 messages
    let res = client.get("/api/v1/search?q=cold").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    assert_eq!(body["query"].as_str().unwrap(), "cold");
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    // Results should include room_name
    assert!(
        results
            .iter()
            .all(|r| r["room_name"].as_str().unwrap() == "search-basic")
    );
}

#[test]
fn test_search_empty_query() {
    let client = test_client();
    let res = client.get("/api/v1/search?q=").dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("empty"));
}

#[test]
fn test_search_no_results() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-empty"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "Hello world"}"#)
        .dispatch();

    let res = client.get("/api/v1/search?q=xyznonexistent").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);
    assert_eq!(body["results"].as_array().unwrap().len(), 0);
}

#[test]
fn test_search_filter_by_room() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-room-a"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-room-b"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "deploy to staging"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_b_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "deploy to production"}"#)
        .dispatch();

    // Search "deploy" scoped to room A
    let res = client
        .get(format!("/api/v1/search?q=deploy&room_id={room_a_id}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(
        body["results"][0]["room_name"].as_str().unwrap(),
        "search-room-a"
    );
}

#[test]
fn test_search_filter_by_sender() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-sender"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "fix the bug"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "found the bug"}"#)
        .dispatch();

    // Search "bug" from Nanook only
    let res = client.get("/api/v1/search?q=bug&sender=Nanook").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(body["results"][0]["sender"].as_str().unwrap(), "Nanook");
}

#[test]
fn test_search_with_limit() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-limit"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    for i in 0..5 {
        client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender": "Nanook", "content": "message number {i}"}}"#
            ))
            .dispatch();
    }

    // Limit to 2 results
    let res = client.get("/api/v1/search?q=message&limit=2").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
}

#[test]
fn test_search_case_insensitive() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-case"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "IMPORTANT UPDATE"}"#)
        .dispatch();

    // SQLite LIKE is case-insensitive for ASCII by default
    let res = client.get("/api/v1/search?q=important").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_cross_room() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-cross-1"}"#)
        .dispatch();
    let room1: serde_json::Value = res.into_json().unwrap();
    let room1_id = room1["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "search-cross-2"}"#)
        .dispatch();
    let room2: serde_json::Value = res.into_json().unwrap();
    let room2_id = room2["id"].as_str().unwrap();

    client
        .post(format!("/api/v1/rooms/{room1_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "testing cross-room search"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room2_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Forge", "content": "also testing search across rooms"}"#)
        .dispatch();

    // Unscoped search should find both
    let res = client.get("/api/v1/search?q=search").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    let room_names: Vec<&str> = body["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["room_name"].as_str().unwrap())
        .collect();
    assert!(room_names.contains(&"search-cross-1"));
    assert!(room_names.contains(&"search-cross-2"));
}
