use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Profiles ---

#[test]
fn test_create_profile() {
    let client = test_client();
    let res = client
        .put("/api/v1/profiles/nanook")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Nanook ❄️","sender_type":"agent","avatar_url":"https://example.com/avatar.png","bio":"Arctic explorer AI","status_text":"online"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["sender"], "nanook");
    assert_eq!(body["display_name"], "Nanook ❄️");
    assert_eq!(body["sender_type"], "agent");
    assert_eq!(body["avatar_url"], "https://example.com/avatar.png");
    assert_eq!(body["bio"], "Arctic explorer AI");
    assert_eq!(body["status_text"], "online");
    assert!(body["created_at"].as_str().is_some());
    assert!(body["updated_at"].as_str().is_some());
}

#[test]
fn test_get_profile() {
    let client = test_client();
    // Create first
    client
        .put("/api/v1/profiles/forge")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Forge ❤️‍🔥","sender_type":"agent","bio":"Builder persona"}"#)
        .dispatch();

    let res = client.get("/api/v1/profiles/forge").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["sender"], "forge");
    assert_eq!(body["display_name"], "Forge ❤️‍🔥");
    assert_eq!(body["bio"], "Builder persona");
}

#[test]
fn test_get_profile_not_found() {
    let client = test_client();
    let res = client.get("/api/v1/profiles/nonexistent").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_list_profiles() {
    let client = test_client();
    // Create two profiles
    client
        .put("/api/v1/profiles/agent1")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Agent One","sender_type":"agent"}"#)
        .dispatch();
    client
        .put("/api/v1/profiles/human1")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Human One","sender_type":"human"}"#)
        .dispatch();

    // List all
    let res = client.get("/api/v1/profiles").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(body.len() >= 2);

    // Filter by sender_type
    let res = client.get("/api/v1/profiles?sender_type=agent").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(body.iter().all(|p| p["sender_type"] == "agent"));
}

#[test]
fn test_update_profile_merge() {
    let client = test_client();
    // Create with initial data
    client
        .put("/api/v1/profiles/drift")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Drift 🌊","sender_type":"agent","bio":"Connector persona"}"#)
        .dispatch();

    // Update only status_text — other fields should be preserved
    let res = client
        .put("/api/v1/profiles/drift")
        .header(ContentType::JSON)
        .body(r#"{"status_text":"busy"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["display_name"], "Drift 🌊");
    assert_eq!(body["bio"], "Connector persona");
    assert_eq!(body["status_text"], "busy");
    assert_eq!(body["sender_type"], "agent");
}

#[test]
fn test_delete_profile() {
    let client = test_client();
    client
        .put("/api/v1/profiles/temp")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Temporary"}"#)
        .dispatch();

    let res = client.delete("/api/v1/profiles/temp").dispatch();
    assert_eq!(res.status(), Status::NoContent);

    // Verify it's gone
    let res = client.get("/api/v1/profiles/temp").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_profile_not_found() {
    let client = test_client();
    let res = client.delete("/api/v1/profiles/nonexistent").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_profile_metadata() {
    let client = test_client();
    let res = client
        .put("/api/v1/profiles/meta-agent")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Meta","metadata":{"capabilities":["search","chat"],"version":"1.0"}}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["metadata"]["capabilities"][0], "search");
    assert_eq!(body["metadata"]["version"], "1.0");
}

#[test]
fn test_profile_minimal() {
    let client = test_client();
    // Create with empty body (all defaults)
    let res = client
        .put("/api/v1/profiles/minimal")
        .header(ContentType::JSON)
        .body(r#"{}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["sender"], "minimal");
    assert!(body["created_at"].as_str().is_some());
}

#[test]
fn test_profile_enriches_participants() {
    let client = test_client();

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"profile-test-room","created_by":"tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message as "profiled-user"
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"profiled-user","content":"Hello!","sender_type":"agent"}"#)
        .dispatch();

    // Create a profile for "profiled-user"
    client
        .put("/api/v1/profiles/profiled-user")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Profile User 🎭","avatar_url":"https://example.com/avatar.png","bio":"Test bio","status_text":"active"}"#)
        .dispatch();

    // Check participants — should include profile data
    let res = client
        .get(format!("/api/v1/rooms/{}/participants", room_id))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    let user = participants
        .iter()
        .find(|p| p["sender"] == "profiled-user")
        .unwrap();
    assert_eq!(user["display_name"], "Profile User 🎭");
    assert_eq!(user["avatar_url"], "https://example.com/avatar.png");
    assert_eq!(user["bio"], "Test bio");
    assert_eq!(user["status_text"], "active");
}

#[test]
fn test_participants_without_profile() {
    let client = test_client();

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"no-profile-room","created_by":"tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message without a profile
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"no-profile-user","content":"Hello!"}"#)
        .dispatch();

    // Participants should still work, with null profile fields
    let res = client
        .get(format!("/api/v1/rooms/{}/participants", room_id))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let participants: Vec<serde_json::Value> = res.into_json().unwrap();
    let user = participants
        .iter()
        .find(|p| p["sender"] == "no-profile-user")
        .unwrap();
    assert_eq!(user["message_count"], 1);
    // Profile fields should not be present (skip_serializing_if = None)
    assert!(user.get("display_name").is_none() || user["display_name"].is_null());
}

#[test]
fn test_profile_preserves_created_at() {
    let client = test_client();

    // Create profile
    let res = client
        .put("/api/v1/profiles/timestamp-test")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Original"}"#)
        .dispatch();
    let first: serde_json::Value = res.into_json().unwrap();
    let created_at = first["created_at"].as_str().unwrap().to_string();

    // Small delay to ensure timestamps differ
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update profile
    let res = client
        .put("/api/v1/profiles/timestamp-test")
        .header(ContentType::JSON)
        .body(r#"{"display_name":"Updated"}"#)
        .dispatch();
    let second: serde_json::Value = res.into_json().unwrap();

    // created_at should be preserved, updated_at should change
    assert_eq!(second["created_at"].as_str().unwrap(), created_at);
    assert_eq!(second["display_name"], "Updated");
}

// --- Profile Field Validation ---

#[test]
fn test_profile_display_name_too_long() {
    let client = test_client();
    let long_name = "a".repeat(201);
    let body = serde_json::json!({"display_name": long_name});
    let res = client
        .put("/api/v1/profiles/valid-sender")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("display_name"));
}

#[test]
fn test_profile_display_name_at_limit() {
    let client = test_client();
    let name_200 = "b".repeat(200);
    let body = serde_json::json!({"display_name": name_200});
    let res = client
        .put("/api/v1/profiles/limit-name")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["display_name"].as_str().unwrap().len(), 200);
}

#[test]
fn test_profile_bio_too_long() {
    let client = test_client();
    let long_bio = "c".repeat(1001);
    let body = serde_json::json!({"bio": long_bio});
    let res = client
        .put("/api/v1/profiles/bio-sender")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("bio"));
}

#[test]
fn test_profile_status_text_too_long() {
    let client = test_client();
    let long_status = "d".repeat(201);
    let body = serde_json::json!({"status_text": long_status});
    let res = client
        .put("/api/v1/profiles/status-sender")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("status_text"));
}

#[test]
fn test_profile_avatar_url_too_long() {
    let client = test_client();
    let long_url = format!("https://example.com/{}", "x".repeat(2001));
    let body = serde_json::json!({"avatar_url": long_url});
    let res = client
        .put("/api/v1/profiles/avatar-sender")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("avatar_url"));
}

#[test]
fn test_profile_invalid_sender_type() {
    let client = test_client();
    let body = serde_json::json!({"sender_type": "robot"});
    let res = client
        .put("/api/v1/profiles/type-sender")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("sender_type"));
}

#[test]
fn test_profile_valid_sender_types() {
    let client = test_client();

    // "agent" should work
    let res = client
        .put("/api/v1/profiles/type-agent")
        .header(ContentType::JSON)
        .body(r#"{"sender_type":"agent"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // "human" should work
    let res = client
        .put("/api/v1/profiles/type-human")
        .header(ContentType::JSON)
        .body(r#"{"sender_type":"human"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
}

#[test]
fn test_profile_metadata_too_large() {
    let client = test_client();
    // Create metadata larger than 10KB
    let big_value = "v".repeat(11_000);
    let body = serde_json::json!({"metadata": {"big": big_value}});
    let res = client
        .put("/api/v1/profiles/meta-sender")
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("metadata"));
}
