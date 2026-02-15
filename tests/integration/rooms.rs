use rocket::http::{ContentType, Header, Status};
use crate::common::test_client;

// --- Rooms ---

#[test]
fn test_default_general_room() {
    let client = test_client();
    let res = client.get("/api/v1/rooms").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(rooms.iter().any(|r| r["name"] == "general"));
}

#[test]
fn test_create_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "test-room", "description": "A test room", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["name"], "test-room");
    assert_eq!(body["created_by"], "tester");
    assert!(!body["id"].as_str().unwrap().is_empty());
}

#[test]
fn test_create_duplicate_room() {
    let client = test_client();
    client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "dup-room"}"#)
        .dispatch();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "dup-room"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Conflict);
}

#[test]
fn test_create_room_empty_name() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": ""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_get_room() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "get-test"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let id = body["id"].as_str().unwrap();

    let res = client.get(format!("/api/v1/rooms/{id}")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["name"], "get-test");
    assert_eq!(body["message_count"], 0);
}

#[test]
fn test_get_room_not_found() {
    let client = test_client();
    let res = client.get("/api/v1/rooms/nonexistent-id").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_room_no_auth() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "del-test"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let id = body["id"].as_str().unwrap();

    // Without admin key → should forward (401-like)
    let res = client.delete(format!("/api/v1/rooms/{id}")).dispatch();
    assert_ne!(res.status(), Status::Ok);
}

#[test]
fn test_delete_room_with_admin() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "admin-del-test"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let id = body["id"].as_str().unwrap();
    let admin_key = body["admin_key"].as_str().unwrap();

    let res = client
        .delete(format!("/api/v1/rooms/{id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify deleted
    let res = client.get(format!("/api/v1/rooms/{id}")).dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_room_wrong_admin_key() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "wrong-key-test"}"#)
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let id = body["id"].as_str().unwrap();

    // Try deleting with wrong key
    let res = client
        .delete(format!("/api/v1/rooms/{id}"))
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);

    // Room should still exist
    let res = client.get(format!("/api/v1/rooms/{id}")).dispatch();
    assert_eq!(res.status(), Status::Ok);
}

// --- Room with description ---

#[test]
fn test_create_room_with_description() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "described-room", "description": "A room for testing descriptions"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    assert_eq!(room["name"], "described-room");
    assert_eq!(room["description"], "A room for testing descriptions");

    // Verify it shows in room detail
    let room_id = room["id"].as_str().unwrap();
    let res = client.get(format!("/api/v1/rooms/{room_id}")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let detail: serde_json::Value = res.into_json().unwrap();
    assert_eq!(detail["description"], "A room for testing descriptions");
}

// --- Room created_by ---

#[test]
fn test_create_room_with_created_by() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "owned-room", "created_by": "nanook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    assert_eq!(room["created_by"], "nanook");
}

// --- Room Update ---

#[test]
fn test_update_room_name() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "old-name", "description": "original desc"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"name": "new-name"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let updated: serde_json::Value = res.into_json().unwrap();
    assert_eq!(updated["name"], "new-name");
    assert_eq!(updated["description"], "original desc"); // unchanged

    // Verify via GET
    let res = client
        .get(format!("/api/v1/rooms/{room_id}"))
        .dispatch();
    let fetched: serde_json::Value = res.into_json().unwrap();
    assert_eq!(fetched["name"], "new-name");
}

#[test]
fn test_update_room_description() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "desc-test", "description": "old desc"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"description": "new desc"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let updated: serde_json::Value = res.into_json().unwrap();
    assert_eq!(updated["name"], "desc-test"); // unchanged
    assert_eq!(updated["description"], "new desc");
}

#[test]
fn test_update_room_wrong_admin_key() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "admin-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .body(r#"{"name": "hacked"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_update_room_not_found() {
    let client = test_client();
    let res = client
        .put("/api/v1/rooms/nonexistent-id")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer some-key"))
        .body(r#"{"name": "test"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_update_room_empty_name_rejected() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "valid-name"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"name": ""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_update_room_duplicate_name() {
    let client = test_client();
    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "room-alpha"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let admin_key_a = room_a["admin_key"].as_str().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "room-beta"}"#)
        .dispatch();

    // Try to rename room-alpha to room-beta
    let res = client
        .put(format!("/api/v1/rooms/{room_a_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key_a}"),
        ))
        .body(r#"{"name": "room-beta"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Conflict);
}

#[test]
fn test_update_room_updates_timestamp() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "timestamp-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();
    let original_updated = room["updated_at"].as_str().unwrap().to_string();

    // Small delay to ensure timestamp differs
    std::thread::sleep(std::time::Duration::from_millis(10));

    let res = client
        .put(format!("/api/v1/rooms/{room_id}"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"description": "updated"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let updated: serde_json::Value = res.into_json().unwrap();
    assert_ne!(updated["updated_at"].as_str().unwrap(), original_updated);
}
