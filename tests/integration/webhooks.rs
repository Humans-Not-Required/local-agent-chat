use rocket::http::{ContentType, Header, Status};
use crate::common::{test_client, create_test_room};

// --- Webhooks ---

#[test]
fn test_create_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-1");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["room_id"], room_id);
    assert_eq!(body["url"], "http://localhost:9999/hook");
    assert_eq!(body["events"], "*");
    assert_eq!(body["active"], true);
    assert!(!body["id"].as_str().unwrap().is_empty());
}

#[test]
fn test_create_webhook_with_event_filter() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-2");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "http://localhost:9999/hook", "events": "message,message_edited", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["events"], "message,message_edited");
}

#[test]
fn test_create_webhook_with_secret() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-3");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "http://localhost:9999/hook", "secret": "my-secret-key", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["has_secret"], true);
}

#[test]
fn test_create_webhook_invalid_url() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-4");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "not-a-url", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_create_webhook_invalid_event() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-room-5");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .body(r#"{"url": "http://localhost:9999/hook", "events": "message,bogus_event", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("bogus_event"));
}

#[test]
fn test_create_webhook_wrong_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "webhook-room-6");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_create_webhook_no_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "webhook-room-7");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    // Without admin key, Rocket forwards (no AdminKey extracted) → 404 or Unauthorized
    assert!(res.status() == Status::Unauthorized || res.status() == Status::NotFound);
}

#[test]
fn test_create_webhook_nonexistent_room() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms/nonexistent-room-id/webhooks")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer some-key"))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_list_webhooks() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-list-room");

    // Create two webhooks
    client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook1", "created_by": "tester"}"#)
        .dispatch();

    client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook2", "events": "message", "created_by": "tester"}"#)
        .dispatch();

    // List
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let webhooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(webhooks.len(), 2);
    // Newest first
    assert_eq!(webhooks[0]["url"], "http://localhost:9999/hook2");
    assert_eq!(webhooks[1]["url"], "http://localhost:9999/hook1");
}

#[test]
fn test_list_webhooks_empty() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-empty-room");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let webhooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(webhooks.len(), 0);
}

#[test]
fn test_list_webhooks_wrong_admin_key() {
    let client = test_client();
    let (room_id, _admin_key) = create_test_room(&client, "webhook-list-auth");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_delete_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-del-room");

    // Create webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    let webhook: serde_json::Value = res.into_json().unwrap();
    let webhook_id = webhook["id"].as_str().unwrap();

    // Delete it
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/webhooks/{webhook_id}"
        ))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["deleted"], true);

    // Verify it's gone
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    let webhooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(webhooks.len(), 0);
}

#[test]
fn test_delete_webhook_not_found() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-del-nf");

    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/webhooks/nonexistent-id"
        ))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_webhook_wrong_admin_key() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-del-auth");

    // Create webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    let webhook: serde_json::Value = res.into_json().unwrap();
    let webhook_id = webhook["id"].as_str().unwrap();

    // Try to delete with wrong key
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/webhooks/{webhook_id}"
        ))
        .header(Header::new("Authorization", "Bearer wrong-key"))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_update_webhook() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-update-room");

    // Create webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    let webhook: serde_json::Value = res.into_json().unwrap();
    let webhook_id = webhook["id"].as_str().unwrap();

    // Update URL and deactivate
    let res = client
        .put(format!(
            "/api/v1/rooms/{room_id}/webhooks/{webhook_id}"
        ))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/new-hook", "active": false}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["updated"], true);

    // Verify via list
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    let webhooks: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(webhooks.len(), 1);
    assert_eq!(webhooks[0]["url"], "http://localhost:9999/new-hook");
    assert_eq!(webhooks[0]["active"], false);
}

#[test]
fn test_update_webhook_not_found() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-update-nf");

    let res = client
        .put(format!(
            "/api/v1/rooms/{room_id}/webhooks/nonexistent-id"
        ))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/new-hook"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_update_webhook_no_fields() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-update-empty");

    // Create webhook
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();
    let webhook: serde_json::Value = res.into_json().unwrap();
    let webhook_id = webhook["id"].as_str().unwrap();

    // Update with empty body
    let res = client
        .put(format!(
            "/api/v1/rooms/{room_id}/webhooks/{webhook_id}"
        ))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_webhooks_cascade_on_room_delete() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "webhook-cascade-room");

    // Create a webhook
    client
        .post(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .body(r#"{"url": "http://localhost:9999/hook", "created_by": "tester"}"#)
        .dispatch();

    // Delete the room
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Webhooks should be gone (room is gone, so 404)
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/webhooks"))
        .header(Header::new(
            "Authorization",
            format!("Bearer {admin_key}"),
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}
