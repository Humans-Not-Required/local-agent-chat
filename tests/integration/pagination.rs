use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Monotonic seq & cursor-based pagination ---

#[test]
fn test_messages_have_seq_field() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "seq-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message and verify seq in response
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "Hello"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msg: serde_json::Value = res.into_json().unwrap();
    assert!(
        msg["seq"].is_number(),
        "Message should have numeric seq field"
    );
    assert!(msg["seq"].as_i64().unwrap() >= 1);

    // Verify seq appears in GET messages
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0]["seq"].is_number());
}

#[test]
fn test_seq_monotonically_increasing() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "seq-mono-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 5 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 0..5 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Verify strictly monotonically increasing
    for i in 1..seqs.len() {
        assert!(
            seqs[i] > seqs[i - 1],
            "seq should be strictly increasing: {} > {}",
            seqs[i],
            seqs[i - 1]
        );
    }
}

#[test]
fn test_after_cursor_pagination() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "after-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 3 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=3 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Use after= to get messages after the first one
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={}",
            seqs[0]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0]["content"], "msg 2");
    assert_eq!(msgs[1]["content"], "msg 3");

    // Use after= to get messages after the second one
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={}",
            seqs[1]
        ))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "msg 3");

    // Use after= with last seq — should get nothing
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={}",
            seqs[2]
        ))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 0);
}

#[test]
fn test_after_cursor_with_limit() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "after-limit-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 5 messages
    let mut first_seq: i64 = 0;
    for i in 1..=5 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        if i == 1 {
            first_seq = msg["seq"].as_i64().unwrap();
        }
    }

    // Get 2 messages after the first
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?after={first_seq}&limit=2"
        ))
        .dispatch();
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0]["content"], "msg 2");
    assert_eq!(msgs[1]["content"], "msg 3");
}

#[test]
fn test_since_still_works_backward_compat() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "since-compat-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send first message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "a", "content": "Old"}"#)
        .dispatch();

    let ts = chrono::Utc::now().to_rfc3339();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Send second message
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "b", "content": "New"}"#)
        .dispatch();

    // since= should still work and return seq in results
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages?since={ts}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["content"], "New");
    assert!(msgs[0]["seq"].is_number());
}

#[test]
fn test_activity_feed_has_seq() {
    let client = test_client();

    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send a message
    client
        .post(format!("/api/v1/rooms/{general_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "Activity seq test"}"#)
        .dispatch();

    // Activity feed events should have seq
    let res = client.get("/api/v1/activity").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["count"].as_i64().unwrap() >= 1);
    let events = body["events"].as_array().unwrap();
    assert!(
        events[0]["seq"].is_number(),
        "Activity events should have seq field"
    );
}

#[test]
fn test_activity_feed_after_cursor() {
    let client = test_client();

    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    let general_id = rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Send 3 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=3 {
        let res = client
            .post(format!("/api/v1/rooms/{general_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender": "bot", "content": "activity msg {i}"}}"#
            ))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Use after= on activity feed — should only get messages after that seq
    let res = client
        .get(format!("/api/v1/activity?after={}", seqs[0]))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_i64().unwrap(), 2);
    let events = body["events"].as_array().unwrap();
    // Activity is newest-first, so msg 3 then msg 2
    assert_eq!(events[0]["content"], "activity msg 3");
    assert_eq!(events[1]["content"], "activity msg 2");
}

#[test]
fn test_seq_global_across_rooms() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "seq-room-a"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "seq-room-b"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    // Send messages alternating between rooms
    let res = client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "A1"}"#)
        .dispatch();
    let msg_a1: serde_json::Value = res.into_json().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_b_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "B1"}"#)
        .dispatch();
    let msg_b1: serde_json::Value = res.into_json().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_a_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "A2"}"#)
        .dispatch();
    let msg_a2: serde_json::Value = res.into_json().unwrap();

    // Seqs should be globally monotonic across rooms
    let seq_a1 = msg_a1["seq"].as_i64().unwrap();
    let seq_b1 = msg_b1["seq"].as_i64().unwrap();
    let seq_a2 = msg_a2["seq"].as_i64().unwrap();
    assert!(seq_b1 > seq_a1, "B1 seq should be > A1 seq");
    assert!(seq_a2 > seq_b1, "A2 seq should be > B1 seq");
}

#[test]
fn test_edit_preserves_seq() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "edit-seq-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "Original"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();
    let original_seq = msg["seq"].as_i64().unwrap();

    // Edit the message
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot", "content": "Edited"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let edited: serde_json::Value = res.into_json().unwrap();

    // Seq should be preserved (not changed)
    assert_eq!(edited["seq"].as_i64().unwrap(), original_seq);
}

// --- before_seq backward pagination tests ---

#[test]
fn test_before_seq_returns_older_messages() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "before-seq-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 5 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=5 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Get messages before seq of msg 4 — should return msg 1, 2, 3
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?before_seq={}",
            seqs[3]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 3);
    // Results should be in chronological order (ASC)
    assert_eq!(msgs[0]["content"], "msg 1");
    assert_eq!(msgs[1]["content"], "msg 2");
    assert_eq!(msgs[2]["content"], "msg 3");
}

#[test]
fn test_before_seq_with_limit() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "before-seq-limit-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 10 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=10 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Get 3 messages before seq of msg 8 — should return msg 5, 6, 7 (most recent 3 before seq 8)
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?before_seq={}&limit=3",
            seqs[7]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0]["content"], "msg 5");
    assert_eq!(msgs[1]["content"], "msg 6");
    assert_eq!(msgs[2]["content"], "msg 7");
}

#[test]
fn test_before_seq_first_message() {
    let client = test_client();
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "before-seq-first-test"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 3 messages
    let mut seqs: Vec<i64> = vec![];
    for i in 1..=3 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(r#"{{"sender": "bot", "content": "msg {i}"}}"#))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Get messages before first message's seq — should return empty
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages?before_seq={}",
            seqs[0]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let msgs: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(msgs.len(), 0);
}

#[test]
fn test_before_seq_nonexistent_room() {
    let client = test_client();
    let res = client
        .get("/api/v1/rooms/00000000-0000-0000-0000-000000000000/messages?before_seq=5")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}
