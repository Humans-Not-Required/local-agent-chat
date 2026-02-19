use local_agent_chat::rate_limit::RateLimitConfig;
use rocket::http::{ContentType, Status};
use serde_json::json;

use super::common::{test_client, test_client_with_rate_limits};

fn create_room(client: &rocket::local::blocking::Client, name: &str) -> String {
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(json!({"name": name}).to_string())
        .dispatch();
    let body: serde_json::Value = serde_json::from_str(&res.into_string().unwrap()).unwrap();
    body["id"].as_str().unwrap().to_string()
}

/// Basic broadcast to two rooms: both receive the message.
#[test]
fn broadcast_two_rooms() {
    let client = test_client();
    let r1 = create_room(&client, "broadcast-a");
    let r2 = create_room(&client, "broadcast-b");

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(
            json!({
                "room_ids": [r1, r2],
                "sender": "herald",
                "content": "Hello all rooms!"
            })
            .to_string(),
        )
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&res.into_string().unwrap()).unwrap();
    assert_eq!(body["sent"], 2);
    assert_eq!(body["failed"], 0);
    assert_eq!(body["results"].as_array().unwrap().len(), 2);
    for result in body["results"].as_array().unwrap() {
        assert_eq!(result["success"], true);
        assert!(result["message_id"].as_str().is_some());
    }
}

/// Broadcast delivers retrievable messages to each room.
#[test]
fn broadcast_messages_retrievable() {
    let client = test_client();
    let r1 = create_room(&client, "bc-retrieve-1");
    let r2 = create_room(&client, "bc-retrieve-2");

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(
            json!({
                "room_ids": [r1.clone(), r2.clone()],
                "sender": "herald",
                "content": "Broadcast content"
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify message appears in room 1
    let msgs1: serde_json::Value = serde_json::from_str(
        &client
            .get(format!("/api/v1/rooms/{r1}/messages"))
            .dispatch()
            .into_string()
            .unwrap(),
    )
    .unwrap();
    let msgs1 = msgs1.as_array().unwrap();
    assert_eq!(msgs1.len(), 1);
    assert_eq!(msgs1[0]["content"], "Broadcast content");
    assert_eq!(msgs1[0]["sender"], "herald");

    // Verify message appears in room 2
    let msgs2: serde_json::Value = serde_json::from_str(
        &client
            .get(format!("/api/v1/rooms/{r2}/messages"))
            .dispatch()
            .into_string()
            .unwrap(),
    )
    .unwrap();
    let msgs2 = msgs2.as_array().unwrap();
    assert_eq!(msgs2.len(), 1);
    assert_eq!(msgs2[0]["content"], "Broadcast content");
}

/// Broadcast to a non-existent room returns partial failure.
#[test]
fn broadcast_invalid_room_partial_failure() {
    let client = test_client();
    let r1 = create_room(&client, "bc-partial");

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(
            json!({
                "room_ids": [r1.clone(), "nonexistent-uuid-1234"],
                "sender": "herald",
                "content": "Partial broadcast"
            })
            .to_string(),
        )
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&res.into_string().unwrap()).unwrap();
    assert_eq!(body["sent"], 1);
    assert_eq!(body["failed"], 1);

    let results = body["results"].as_array().unwrap();
    let success = results.iter().find(|r| r["room_id"] == r1).unwrap();
    let failure = results.iter().find(|r| r["room_id"] == "nonexistent-uuid-1234").unwrap();

    assert_eq!(success["success"], true);
    assert_eq!(failure["success"], false);
    assert!(failure["error"].as_str().is_some());
}

/// Broadcast to all invalid rooms returns all failed.
#[test]
fn broadcast_all_invalid_rooms() {
    let client = test_client();

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(
            json!({
                "room_ids": ["bad-uuid-1", "bad-uuid-2"],
                "sender": "herald",
                "content": "Nobody home"
            })
            .to_string(),
        )
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&res.into_string().unwrap()).unwrap();
    assert_eq!(body["sent"], 0);
    assert_eq!(body["failed"], 2);
}

/// Empty room_ids is rejected with 400.
#[test]
fn broadcast_empty_room_ids_rejected() {
    let client = test_client();

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(json!({"room_ids": [], "sender": "herald", "content": "Hi"}).to_string())
        .dispatch();

    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = serde_json::from_str(&res.into_string().unwrap()).unwrap();
    assert!(body["error"].as_str().unwrap().contains("room_ids"));
}

/// More than 20 rooms is rejected with 400.
#[test]
fn broadcast_too_many_rooms_rejected() {
    let client = test_client();

    let room_ids: Vec<String> = (0..21).map(|i| format!("fake-room-{i}")).collect();
    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(json!({"room_ids": room_ids, "sender": "herald", "content": "Too many"}).to_string())
        .dispatch();

    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = serde_json::from_str(&res.into_string().unwrap()).unwrap();
    assert!(body["error"].as_str().unwrap().contains("20"));
}

/// Empty content is rejected with 400.
#[test]
fn broadcast_empty_content_rejected() {
    let client = test_client();
    let r = create_room(&client, "bc-empty-content");

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(json!({"room_ids": [r], "sender": "herald", "content": ""}).to_string())
        .dispatch();

    assert_eq!(res.status(), Status::BadRequest);
}

/// Empty sender is rejected with 400.
#[test]
fn broadcast_empty_sender_rejected() {
    let client = test_client();
    let r = create_room(&client, "bc-empty-sender");

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(json!({"room_ids": [r], "sender": "", "content": "Hi"}).to_string())
        .dispatch();

    assert_eq!(res.status(), Status::BadRequest);
}

/// sender_type is preserved on broadcast messages.
#[test]
fn broadcast_sender_type_preserved() {
    let client = test_client();
    let r = create_room(&client, "bc-sender-type");

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(
            json!({
                "room_ids": [r.clone()],
                "sender": "herald",
                "sender_type": "agent",
                "content": "Agent broadcast"
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let msgs: serde_json::Value = serde_json::from_str(
        &client
            .get(format!("/api/v1/rooms/{r}/messages"))
            .dispatch()
            .into_string()
            .unwrap(),
    )
    .unwrap();
    let msgs = msgs.as_array().unwrap();
    assert_eq!(msgs[0]["sender_type"], "agent");
}

/// Broadcast messages are FTS-searchable.
#[test]
fn broadcast_messages_searchable() {
    let client = test_client();
    let r1 = create_room(&client, "bc-search-1");
    let r2 = create_room(&client, "bc-search-2");

    // Regular message in r2 with different content
    client
        .post(format!("/api/v1/rooms/{r2}/messages"))
        .header(ContentType::JSON)
        .body(json!({"sender": "other", "content": "unrelated message"}).to_string())
        .dispatch();

    // Broadcast a unique phrase
    client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(
            json!({
                "room_ids": [r1.clone(), r2.clone()],
                "sender": "herald",
                "content": "Syswidealert deployment notice"
            })
            .to_string(),
        )
        .dispatch();

    // Search should find the broadcast in both rooms
    let res: serde_json::Value = serde_json::from_str(
        &client
            .get("/api/v1/search?q=Syswidealert")
            .dispatch()
            .into_string()
            .unwrap(),
    )
    .unwrap();
    let results = res["results"].as_array().unwrap();
    assert_eq!(results.len(), 2, "broadcast message should appear in both rooms");
    let room_ids_found: Vec<&str> = results.iter().map(|r| r["room_id"].as_str().unwrap()).collect();
    assert!(room_ids_found.contains(&r1.as_str()));
    assert!(room_ids_found.contains(&r2.as_str()));
}

/// Broadcast to a single room works correctly.
#[test]
fn broadcast_single_room() {
    let client = test_client();
    let r = create_room(&client, "bc-single");

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(json!({"room_ids": [r.clone()], "sender": "herald", "content": "Single room"}).to_string())
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&res.into_string().unwrap()).unwrap();
    assert_eq!(body["sent"], 1);
    assert_eq!(body["failed"], 0);
    assert_eq!(body["results"][0]["room_id"], r);
    assert_eq!(body["results"][0]["success"], true);
    assert!(body["results"][0]["message_id"].as_str().is_some());
}

/// Broadcast to 20 rooms (max limit) succeeds.
#[test]
fn broadcast_twenty_rooms_succeeds() {
    // Use a raised room creation limit (default 10/hr would block creating 20 rooms)
    let config = RateLimitConfig { rooms_max: 50, ..Default::default() };
    let client = test_client_with_rate_limits(config);
    let room_ids: Vec<String> = (0..20)
        .map(|i| create_room(&client, &format!("bc-max-{i}")))
        .collect();

    let res = client
        .post("/api/v1/broadcast")
        .header(ContentType::JSON)
        .body(json!({"room_ids": room_ids, "sender": "herald", "content": "Max rooms test"}).to_string())
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&res.into_string().unwrap()).unwrap();
    assert_eq!(body["sent"], 20);
    assert_eq!(body["failed"], 0);
}
