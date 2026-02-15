use rocket::http::{ContentType, Header, Status};
use crate::common::{test_client, create_test_room};

// --- Incoming Webhooks ---

#[test]
fn test_create_incoming_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-1");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "CI Alerts", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["room_id"], room_id);
    assert_eq!(body["name"], "CI Alerts");
    assert_eq!(body["active"], true);
    assert!(body["token"].as_str().unwrap().starts_with("whk_"));
    assert!(body["url"].as_str().unwrap().starts_with("/api/v1/hook/whk_"));
}

#[test]
fn test_create_incoming_webhook_no_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "inhook-room-noauth");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .body(r#"{"name": "test", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
}

#[test]
fn test_create_incoming_webhook_wrong_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "inhook-room-wrongkey");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer wrong_key"))
        .body(r#"{"name": "test", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_create_incoming_webhook_empty_name() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-emptyname");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_list_incoming_webhooks() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-list");

    // Create two webhooks
    client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "Hook A", "created_by": "tester"}"#)
        .dispatch();
    client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "Hook B", "created_by": "tester"}"#)
        .dispatch();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let hooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(hooks.len(), 2);
}

#[test]
fn test_update_incoming_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-update");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "Old Name", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let hook_id = hook["id"].as_str().unwrap();

    // Update name
    let res = client
        .put(format!(
            "/api/v1/rooms/{room_id}/incoming-webhooks/{hook_id}"
        ))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "New Name"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["updated"], true);

    // Disable
    let res = client
        .put(format!(
            "/api/v1/rooms/{room_id}/incoming-webhooks/{hook_id}"
        ))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"active": false}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    let hooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(hooks[0]["name"], "New Name");
    assert_eq!(hooks[0]["active"], false);
}

#[test]
fn test_delete_incoming_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-delete");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "To Delete", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let hook_id = hook["id"].as_str().unwrap();

    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/incoming-webhooks/{hook_id}"
        ))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["deleted"], true);

    // Verify it's gone
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    let hooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(hooks.len(), 0);
}

#[test]
fn test_post_via_incoming_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-post");

    // Create incoming webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "CI Bot", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();

    // Post a message via the hook (no auth header needed)
    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "Build #42 passed ✅"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["content"], "Build #42 passed ✅");
    assert_eq!(msg["sender"], "CI Bot"); // Falls back to webhook name
    assert_eq!(msg["sender_type"], "agent"); // Default
    assert_eq!(msg["room_id"], room_id);
    assert!(msg["seq"].as_i64().unwrap() > 0);

    // Verify message appears in room
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert!(msgs.iter().any(|m| m["content"] == "Build #42 passed ✅"));
}

#[test]
fn test_post_via_incoming_webhook_custom_sender() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-sender");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "Default Name", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();

    // Post with custom sender
    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "Hello!", "sender": "CustomBot", "sender_type": "human"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert_eq!(msg["sender"], "CustomBot");
    assert_eq!(msg["sender_type"], "human");
}

#[test]
fn test_post_via_incoming_webhook_invalid_token() {
    let client = test_client();

    let res = client
        .post("/api/v1/hook/whk_nonexistent_token")
        .header(ContentType::JSON)
        .body(r#"{"content": "test"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_post_via_incoming_webhook_disabled() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-disabled");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "Disabled Hook", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();
    let hook_id = hook["id"].as_str().unwrap();

    // Disable the webhook
    client
        .put(format!(
            "/api/v1/rooms/{room_id}/incoming-webhooks/{hook_id}"
        ))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"active": false}"#)
        .dispatch();

    // Try to post — should be rejected
    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "Should fail"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_post_via_incoming_webhook_empty_content() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-empty");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "Test Hook", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": ""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_incoming_webhooks_cascade_on_room_delete() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-cascade");

    // Create a webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "Cascade Test", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap().to_string();

    // Delete the room
    client
        .delete(format!("/api/v1/rooms/{room_id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();

    // Token should no longer work
    let res = client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "Should fail"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_post_via_incoming_webhook_searchable() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "inhook-room-search");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/incoming-webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"name": "SearchBot", "created_by": "tester"}"#)
        .dispatch();
    let hook: serde_json::Value = res.into_json().unwrap();
    let token = hook["token"].as_str().unwrap();

    // Post a message with unique content
    client
        .post(format!("/api/v1/hook/{token}"))
        .header(ContentType::JSON)
        .body(r#"{"content": "Deployment xylophone92 succeeded"}"#)
        .dispatch();

    // Verify it's searchable via FTS
    let res = client
        .get("/api/v1/search?q=xylophone92")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);
    assert_eq!(body["results"][0]["sender"], "SearchBot");
}
