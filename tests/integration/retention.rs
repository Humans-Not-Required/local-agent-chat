use rocket::http::{ContentType, Header, Status};
use crate::common::{test_client, create_test_room};

// --- Room Creation with Retention Settings ---

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
