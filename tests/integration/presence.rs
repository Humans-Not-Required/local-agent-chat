use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Presence ---

#[test]
fn test_room_presence_empty() {
    let client = test_client();
    // Get general room ID
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    let res = client.get(format!("/api/v1/rooms/{room_id}/presence")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["room_id"], room_id);
    assert_eq!(body["count"], 0);
    assert!(body["online"].as_array().unwrap().is_empty());
}

#[test]
fn test_room_presence_nonexistent_room() {
    let client = test_client();
    let res = client.get("/api/v1/rooms/nonexistent/presence").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_global_presence_empty() {
    let client = test_client();
    let res = client.get("/api/v1/presence").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["total_online"], 0);
    assert!(body["rooms"].as_object().unwrap().is_empty());
}

#[test]
fn test_presence_tracker_unit() {
    // Unit test for PresenceTracker directly
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    // Join
    let is_new = tracker.join("room1", "alice", Some("agent"));
    assert!(is_new, "First join should be new");

    // Get room presence
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 1);
    assert_eq!(online[0].sender, "alice");
    assert_eq!(online[0].sender_type.as_deref(), Some("agent"));

    // Second join (same sender, same room) should NOT be new
    let is_new = tracker.join("room1", "alice", Some("agent"));
    assert!(!is_new, "Second join from same sender should not be new");

    // Still only 1 unique presence
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 1);

    // Leave once — should NOT be fully left (2 connections)
    let fully_left = tracker.leave("room1", "alice");
    assert!(!fully_left, "First leave with 2 connections should not be fully left");

    // Still present
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 1);

    // Leave again — should be fully left
    let fully_left = tracker.leave("room1", "alice");
    assert!(fully_left, "Second leave should be fully left");

    // Now gone
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 0);
}

#[test]
fn test_presence_tracker_multiple_users() {
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    tracker.join("room1", "alice", Some("agent"));
    tracker.join("room1", "bob", Some("human"));
    tracker.join("room2", "charlie", None);

    // Room 1 has 2 users
    let online = tracker.get_room("room1");
    assert_eq!(online.len(), 2);

    // Room 2 has 1 user
    let online = tracker.get_room("room2");
    assert_eq!(online.len(), 1);
    assert_eq!(online[0].sender, "charlie");

    // Global: 2 rooms, 3 unique users
    let all = tracker.get_all();
    assert_eq!(all.len(), 2);
    let total: usize = all.values().map(|v| v.len()).sum();
    assert_eq!(total, 3);
}

#[test]
fn test_presence_tracker_leave_cleans_empty_rooms() {
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    tracker.join("room1", "alice", None);
    tracker.leave("room1", "alice");

    // Room should be cleaned up from the map
    let all = tracker.get_all();
    assert!(all.is_empty(), "Empty rooms should be cleaned up");
}

#[test]
fn test_presence_tracker_leave_nonexistent() {
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    // Leave from a room that doesn't exist — should not panic
    let result = tracker.leave("room1", "nobody");
    assert!(!result);
}

#[test]
fn test_presence_tracker_sender_type_update() {
    use local_agent_chat::routes::PresenceTracker;

    let tracker = PresenceTracker::default();

    // First join without sender_type
    tracker.join("room1", "alice", None);
    let online = tracker.get_room("room1");
    assert!(online[0].sender_type.is_none());

    // Second join with sender_type should update it
    tracker.join("room1", "alice", Some("agent"));
    let online = tracker.get_room("room1");
    assert_eq!(online[0].sender_type.as_deref(), Some("agent"));
}

#[test]
fn test_stream_with_sender_registers_presence() {
    let client = test_client();
    // Get general room ID
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Connect to SSE stream with sender
    let response = client
        .get(format!("/api/v1/rooms/{room_id}/stream?sender=nanook&sender_type=agent"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);

    // While the stream is alive, presence should show nanook
    let res = client.get(format!("/api/v1/rooms/{room_id}/presence")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);
    let online = body["online"].as_array().unwrap();
    assert_eq!(online[0]["sender"], "nanook");
    assert_eq!(online[0]["sender_type"], "agent");
    assert!(online[0]["connected_at"].as_str().is_some());

    // Drop the response (SSE stream) — presence should be cleaned up
    drop(response);

    let res = client.get(format!("/api/v1/rooms/{room_id}/presence")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 0);
}

#[test]
fn test_stream_without_sender_no_presence() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Connect to SSE stream without sender (backward compat)
    let _response = client
        .get(format!("/api/v1/rooms/{room_id}/stream"))
        .dispatch();

    // No presence should be registered
    let res = client.get(format!("/api/v1/rooms/{room_id}/presence")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 0);
}

#[test]
fn test_global_presence_with_connections() {
    let client = test_client();
    let rooms: Vec<serde_json::Value> = client.get("/api/v1/rooms").dispatch().into_json().unwrap();
    let room_id = rooms[0]["id"].as_str().unwrap();

    // Create a second room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "testing", "created_by": "test"}"#)
        .dispatch();
    let room2: serde_json::Value = res.into_json().unwrap();
    let room2_id = room2["id"].as_str().unwrap();

    // Connect to both rooms
    let _stream1 = client
        .get(format!("/api/v1/rooms/{room_id}/stream?sender=agent-a&sender_type=agent"))
        .dispatch();
    let _stream2 = client
        .get(format!("/api/v1/rooms/{room2_id}/stream?sender=agent-b&sender_type=agent"))
        .dispatch();

    // Global presence should show both
    let res = client.get("/api/v1/presence").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["total_online"], 2);
    assert_eq!(body["rooms"].as_object().unwrap().len(), 2);
}
