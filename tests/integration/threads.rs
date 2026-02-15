use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Thread View ---

#[test]
fn test_thread_simple() {
    let client = test_client();

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-test", "created_by": "tester"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send root message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Root message"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    // Send reply to root
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "Reply 1", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let reply1: serde_json::Value = res.into_json().unwrap();
    let reply1_id = reply1["id"].as_str().unwrap();

    // Send reply to the reply (nested)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "charlie", "content": "Nested reply", "reply_to": "{reply1_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Get thread from root
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["root"]["id"], root_id);
    assert_eq!(thread["root"]["content"], "Root message");
    assert_eq!(thread["total_replies"], 2);

    let replies = thread["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["content"], "Reply 1");
    assert_eq!(replies[0]["depth"], 1);
    assert_eq!(replies[1]["content"], "Nested reply");
    assert_eq!(replies[1]["depth"], 2);
}

#[test]
fn test_thread_from_child_walks_to_root() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-walk", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Root -> Reply1 -> Reply2
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Thread root"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "Mid reply", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    let mid: serde_json::Value = res.into_json().unwrap();
    let mid_id = mid["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "charlie", "content": "Leaf reply", "reply_to": "{mid_id}"}}"#
        ))
        .dispatch();
    let leaf: serde_json::Value = res.into_json().unwrap();
    let leaf_id = leaf["id"].as_str().unwrap();

    // Request thread from the LEAF message — should walk up to root
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{leaf_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    // Root should be the original root message
    assert_eq!(thread["root"]["id"], root_id);
    assert_eq!(thread["root"]["content"], "Thread root");
    assert_eq!(thread["total_replies"], 2);
}

#[test]
fn test_thread_single_message_no_replies() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-solo", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Standalone"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["root"]["id"], msg_id);
    assert_eq!(thread["total_replies"], 0);
    assert!(thread["replies"].as_array().unwrap().is_empty());
}

#[test]
fn test_thread_not_found() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-404", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Non-existent message
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/nonexistent-id/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_thread_wrong_room() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "room-a", "created_by": "tester"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "room-b", "created_by": "tester"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    // Create message in room A
    let res = client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "In room A"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Try to get thread in room B — should 404
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_b_id}/messages/{msg_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_thread_multiple_branches() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-branches", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Root message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Root"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    // Two direct replies to root (branching)
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "Branch A", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "charlie", "content": "Branch B", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Get thread — should have root + 2 branches
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["total_replies"], 2);
    let replies = thread["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 2);
    // Both should be depth 1
    assert!(replies.iter().all(|r| r["depth"] == 1));
}

#[test]
fn test_thread_nonexistent_room() {
    let client = test_client();

    let res = client
        .get("/api/v1/rooms/nonexistent-room/messages/some-id/thread")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["error"], "Room not found");
}
