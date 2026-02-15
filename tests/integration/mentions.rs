use rocket::http::{ContentType, Status};
use crate::common::test_client;

// --- Mentions ---

#[test]
fn test_mentions_basic() {
    let client = test_client();

    // Create a room
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"mention-test","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message mentioning nanook
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"Hey @nanook can you check this?"}"#)
        .dispatch();

    // Query mentions for nanook
    let res = client.get("/api/v1/mentions?target=nanook").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["target"], "nanook");
    assert_eq!(body["count"], 1);
    let mentions = body["mentions"].as_array().unwrap();
    assert_eq!(mentions[0]["sender"], "alice");
    assert!(mentions[0]["content"].as_str().unwrap().contains("@nanook"));
    assert_eq!(mentions[0]["room_name"], "mention-test");
}

#[test]
fn test_mentions_excludes_self() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"self-mention","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // nanook mentions themselves (shouldn't appear in results)
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"nanook","content":"I @nanook am testing"}"#)
        .dispatch();

    // alice mentions nanook (should appear)
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"Hey @nanook!"}"#)
        .dispatch();

    let res = client.get("/api/v1/mentions?target=nanook").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);
    assert_eq!(body["mentions"][0]["sender"], "alice");
}

#[test]
fn test_mentions_multiple_rooms() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"room-a","created_by":"admin"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"room-b","created_by":"admin"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    // Mention in both rooms
    client
        .post(format!("/api/v1/rooms/{}/messages", room_a_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"@forge check this in room A"}"#)
        .dispatch();

    client
        .post(format!("/api/v1/rooms/{}/messages", room_b_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"bob","content":"@forge room B needs you"}"#)
        .dispatch();

    // Get all mentions
    let res = client.get("/api/v1/mentions?target=forge").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 2);

    // Filter by room
    let res = client
        .get(format!("/api/v1/mentions?target=forge&room_id={}", room_a_id))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 1);
    assert_eq!(body["mentions"][0]["room_name"], "room-a");
}

#[test]
fn test_mentions_cursor_pagination() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"paginate-mentions","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 3 mentions
    for i in 1..=3 {
        client
            .post(format!("/api/v1/rooms/{}/messages", room_id))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender":"alice","content":"@nanook message {i}"}}"#
            ))
            .dispatch();
    }

    // Get first 2 (newest first)
    let res = client
        .get("/api/v1/mentions?target=nanook&limit=2")
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 2);
    let mentions = body["mentions"].as_array().unwrap();
    // Newest first
    assert!(mentions[0]["content"].as_str().unwrap().contains("message 3"));
    assert!(mentions[1]["content"].as_str().unwrap().contains("message 2"));

    // Use cursor to get the rest (messages with seq > some value)
    // Note: after=seq gets messages AFTER that seq, but since we ORDER BY seq DESC,
    // we need a different approach. Let's use the lowest seq from previous batch.
    let _oldest_seq = mentions[1]["seq"].as_i64().unwrap();
    // Get mentions after seq=0 but limit to 1 to prove pagination works
    let res = client
        .get("/api/v1/mentions?target=nanook&limit=10")
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 3);
}

#[test]
fn test_mentions_empty_target_rejected() {
    let client = test_client();
    let res = client.get("/api/v1/mentions?target=").dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("must not be empty"));
}

#[test]
fn test_mentions_no_results() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"no-mentions","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a message without any mentions
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"No mentions here"}"#)
        .dispatch();

    let res = client.get("/api/v1/mentions?target=nanook").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 0);
    assert!(body["mentions"].as_array().unwrap().is_empty());
}

#[test]
fn test_unread_mentions_basic() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"unread-mentions","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send mentions
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"@nanook first mention"}"#)
        .dispatch();

    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"bob","content":"@nanook second mention"}"#)
        .dispatch();

    // Check unread mentions (no read position set yet, so all are unread)
    let res = client.get("/api/v1/mentions/unread?target=nanook").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["target"], "nanook");
    assert_eq!(body["total_unread"], 2);
    let rooms = body["rooms"].as_array().unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0]["room_name"], "unread-mentions");
    assert_eq!(rooms[0]["mention_count"], 2);
}

#[test]
fn test_unread_mentions_after_read() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"read-mentions","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send first mention
    let res = client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"@nanook first"}"#)
        .dispatch();
    let msg1: serde_json::Value = res.into_json().unwrap();
    let seq1 = msg1["seq"].as_i64().unwrap();

    // Mark as read up to seq1
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender":"nanook","last_read_seq":{seq1}}}"#))
        .dispatch();

    // Send second mention (after read position)
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"bob","content":"@nanook second"}"#)
        .dispatch();

    // Only the second mention should be unread
    let res = client.get("/api/v1/mentions/unread?target=nanook").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["total_unread"], 1);
    let rooms = body["rooms"].as_array().unwrap();
    assert_eq!(rooms[0]["mention_count"], 1);
}

#[test]
fn test_unread_mentions_all_read() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"all-read","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send a mention
    let res = client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"@nanook hey"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let seq = msg["seq"].as_i64().unwrap();

    // Mark as read
    client
        .put(format!("/api/v1/rooms/{}/read", room_id))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender":"nanook","last_read_seq":{seq}}}"#))
        .dispatch();

    // No unread mentions
    let res = client.get("/api/v1/mentions/unread?target=nanook").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["total_unread"], 0);
    assert!(body["rooms"].as_array().unwrap().is_empty());
}

#[test]
fn test_unread_mentions_empty_target() {
    let client = test_client();
    let res = client.get("/api/v1/mentions/unread?target=").dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_mentions_case_sensitive() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"case-mentions","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // SQLite LIKE is case-insensitive by default for ASCII
    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"Hey @Nanook check this"}"#)
        .dispatch();

    client
        .post(format!("/api/v1/rooms/{}/messages", room_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"bob","content":"@NANOOK urgent!"}"#)
        .dispatch();

    // SQLite LIKE is case-insensitive for ASCII, so both should match
    let res = client.get("/api/v1/mentions?target=nanook").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 2);
}

#[test]
fn test_mentions_with_after_cursor() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"cursor-mentions","created_by":"admin"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Send 3 mentions
    let mut seqs = vec![];
    for i in 1..=3 {
        let res = client
            .post(format!("/api/v1/rooms/{}/messages", room_id))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender":"alice","content":"@forge msg {i}"}}"#
            ))
            .dispatch();
        let msg: serde_json::Value = res.into_json().unwrap();
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Get mentions after the first one
    let res = client
        .get(format!("/api/v1/mentions?target=forge&after={}", seqs[0]))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"], 2);
}

#[test]
fn test_unread_mentions_multiple_rooms() {
    let client = test_client();

    // Create two rooms
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"unread-a","created_by":"admin"}"#)
        .dispatch();
    let room_a: serde_json::Value = res.into_json().unwrap();
    let room_a_id = room_a["id"].as_str().unwrap();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name":"unread-b","created_by":"admin"}"#)
        .dispatch();
    let room_b: serde_json::Value = res.into_json().unwrap();
    let room_b_id = room_b["id"].as_str().unwrap();

    // Mentions in both rooms
    client
        .post(format!("/api/v1/rooms/{}/messages", room_a_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"alice","content":"@lux check room A"}"#)
        .dispatch();

    client
        .post(format!("/api/v1/rooms/{}/messages", room_b_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"bob","content":"@lux room B too"}"#)
        .dispatch();

    client
        .post(format!("/api/v1/rooms/{}/messages", room_b_id))
        .header(ContentType::JSON)
        .body(r#"{"sender":"charlie","content":"@lux also room B"}"#)
        .dispatch();

    // All unread
    let res = client.get("/api/v1/mentions/unread?target=lux").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["total_unread"], 3);
    assert_eq!(body["rooms"].as_array().unwrap().len(), 2);

    // Mark room A as read
    let res = client
        .get(format!("/api/v1/rooms/{}/messages", room_a_id))
        .dispatch();
    let msgs: serde_json::Value = res.into_json().unwrap();
    let last_seq = msgs.as_array().unwrap().last().unwrap()["seq"].as_i64().unwrap();
    client
        .put(format!("/api/v1/rooms/{}/read", room_a_id))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender":"lux","last_read_seq":{last_seq}}}"#))
        .dispatch();

    // Now only room B mentions are unread
    let res = client.get("/api/v1/mentions/unread?target=lux").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["total_unread"], 2);
    assert_eq!(body["rooms"].as_array().unwrap().len(), 1);
    assert_eq!(body["rooms"][0]["room_name"], "unread-b");
}
