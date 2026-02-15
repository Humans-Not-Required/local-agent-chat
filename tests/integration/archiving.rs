use rocket::http::{ContentType, Header, Status};
use crate::common::test_client;

// --- Room Archiving ---

#[test]
fn test_archive_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "archive-me", "description": "test room"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Archive the room
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let archived: serde_json::Value = res.into_json().unwrap();
    assert!(archived["archived_at"].is_string());
    assert_eq!(archived["name"], "archive-me");
}

#[test]
fn test_archive_room_wrong_key() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "no-archive"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new("Authorization", "Bearer wrong_key"))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_archive_room_not_found() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms/nonexistent-id/archive")
        .header(Header::new("Authorization", "Bearer some_key"))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_archive_already_archived() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "double-archive"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Archive once
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Archive again — should conflict
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Conflict);
}

#[test]
fn test_unarchive_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "unarchive-me"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Archive
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();

    // Unarchive
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/unarchive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    assert!(room["archived_at"].is_null());
}

#[test]
fn test_unarchive_not_archived() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "not-archived"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Try to unarchive a room that's not archived
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/unarchive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Conflict);
}

#[test]
fn test_archived_rooms_hidden_from_list() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "visible-room"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "hidden-room"}"#)
        .dispatch();
    let hidden: serde_json::Value = res.into_json().unwrap();
    let hidden_id = hidden["id"].as_str().unwrap();
    let admin_key = hidden["admin_key"].as_str().unwrap();

    // Archive the hidden room
    client
        .post(format!("/api/v1/rooms/{hidden_id}/archive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();

    // List rooms (default — no archived)
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let names: Vec<&str> = rooms.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"visible-room"));
    assert!(!names.contains(&"hidden-room"));

    // List rooms with include_archived=true
    let res = client.get("/api/v1/rooms?include_archived=true").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let names: Vec<&str> = rooms.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"visible-room"));
    assert!(names.contains(&"hidden-room"));
}

#[test]
fn test_get_archived_room_shows_archived_at() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "get-archived"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Before archiving — no archived_at
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body.get("archived_at").is_none() || body["archived_at"].is_null());

    // Archive
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();

    // After archiving — has archived_at
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["archived_at"].is_string());
}

#[test]
fn test_archive_unarchive_roundtrip() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "roundtrip-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();
    let auth = Header::new("Authorization", format!("Bearer {admin_key}"));

    // Archive
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(auth.clone())
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify hidden
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(!rooms.iter().any(|r| r["name"] == "roundtrip-room"));

    // Unarchive
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/unarchive"))
        .header(auth.clone())
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify visible again
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(rooms.iter().any(|r| r["name"] == "roundtrip-room"));
}

#[test]
fn test_archived_room_messages_still_accessible() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "msg-after-archive"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Send a message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "tester", "content": "pre-archive message"}"#)
        .dispatch();

    // Archive
    client
        .post(format!("/api/v1/rooms/{room_id}/archive"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();

    // Messages should still be accessible
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "pre-archive message");
}
