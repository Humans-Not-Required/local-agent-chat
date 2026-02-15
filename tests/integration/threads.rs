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

#[test]
fn test_thread_deep_nesting() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-deep", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Create a chain: root -> d1 -> d2 -> d3 -> d4 -> d5
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "a", "content": "depth 0 (root)"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let mut parent_id = root["id"].as_str().unwrap().to_string();
    let root_id = parent_id.clone();

    for depth in 1..=5 {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender": "a", "content": "depth {depth}", "reply_to": "{parent_id}"}}"#
            ))
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let msg: serde_json::Value = res.into_json().unwrap();
        parent_id = msg["id"].as_str().unwrap().to_string();
    }

    // Get thread from root
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["total_replies"], 5);
    let replies = thread["replies"].as_array().unwrap();
    // Verify depth increases: 1, 2, 3, 4, 5
    for (i, reply) in replies.iter().enumerate() {
        assert_eq!(reply["depth"], i as i64 + 1, "Reply {} should be depth {}", i, i + 1);
        assert_eq!(reply["content"].as_str().unwrap(), format!("depth {}", i + 1));
    }

    // Get thread from the deepest leaf — should walk back to root
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{parent_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread2: serde_json::Value = res.into_json().unwrap();
    assert_eq!(thread2["root"]["id"].as_str().unwrap(), root_id);
    assert_eq!(thread2["total_replies"], 5);
}

#[test]
fn test_thread_chronological_ordering() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-order", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Root
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Root"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    // Add replies in order: first, second, third
    for label in ["first", "second", "third"] {
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender": "bob", "content": "{label}", "reply_to": "{root_id}"}}"#
            ))
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
    }

    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    let thread: serde_json::Value = res.into_json().unwrap();
    let replies = thread["replies"].as_array().unwrap();

    // Should be in chronological order (by seq)
    assert_eq!(replies[0]["content"], "first");
    assert_eq!(replies[1]["content"], "second");
    assert_eq!(replies[2]["content"], "third");

    // Verify seq values are increasing
    let seq0 = replies[0]["seq"].as_i64().unwrap();
    let seq1 = replies[1]["seq"].as_i64().unwrap();
    let seq2 = replies[2]["seq"].as_i64().unwrap();
    assert!(seq0 < seq1 && seq1 < seq2, "seqs should be increasing: {seq0}, {seq1}, {seq2}");
}

#[test]
fn test_thread_with_deleted_reply() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-deleted", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Root -> reply1, reply2
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Root"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "Reply to delete", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    let reply1: serde_json::Value = res.into_json().unwrap();
    let reply1_id = reply1["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "charlie", "content": "Kept reply", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Delete reply1
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{reply1_id}?sender=bob"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Thread should now only have 1 reply
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();
    assert_eq!(thread["total_replies"], 1);
    let replies = thread["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["content"], "Kept reply");
}

#[test]
fn test_thread_many_replies() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-many", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Root
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "Discussion topic"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    // Add 15 direct replies
    for i in 0..15 {
        let sender = format!("agent-{i}");
        let content = format!("Reply number {i}");
        let res = client
            .post(format!("/api/v1/rooms/{room_id}/messages"))
            .header(ContentType::JSON)
            .body(format!(
                r#"{{"sender": "{sender}", "content": "{content}", "reply_to": "{root_id}"}}"#
            ))
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
    }

    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["total_replies"], 15);
    let replies = thread["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 15);

    // All should be depth 1 (direct replies to root)
    assert!(replies.iter().all(|r| r["depth"] == 1));

    // Verify all senders are present
    let senders: Vec<&str> = replies.iter().map(|r| r["sender"].as_str().unwrap()).collect();
    for i in 0..15 {
        assert!(senders.contains(&format!("agent-{i}").as_str()));
    }
}

#[test]
fn test_thread_mixed_branches_and_depth() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "thread-mixed", "created_by": "tester"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();

    // Structure:
    // root
    //   ├── branch_a (depth 1)
    //   │   └── branch_a_reply (depth 2)
    //   └── branch_b (depth 1)
    //       └── branch_b_reply (depth 2)
    //           └── branch_b_deep (depth 3)

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "root_author", "content": "Root"}"#)
        .dispatch();
    let root: serde_json::Value = res.into_json().unwrap();
    let root_id = root["id"].as_str().unwrap();

    // Branch A
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "a", "content": "branch_a", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    let branch_a: serde_json::Value = res.into_json().unwrap();
    let branch_a_id = branch_a["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "a", "content": "branch_a_reply", "reply_to": "{branch_a_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Branch B
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "b", "content": "branch_b", "reply_to": "{root_id}"}}"#
        ))
        .dispatch();
    let branch_b: serde_json::Value = res.into_json().unwrap();
    let branch_b_id = branch_b["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "b", "content": "branch_b_reply", "reply_to": "{branch_b_id}"}}"#
        ))
        .dispatch();
    let branch_b_reply: serde_json::Value = res.into_json().unwrap();
    let branch_b_reply_id = branch_b_reply["id"].as_str().unwrap();

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "b", "content": "branch_b_deep", "reply_to": "{branch_b_reply_id}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Get full thread
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/messages/{root_id}/thread"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let thread: serde_json::Value = res.into_json().unwrap();

    assert_eq!(thread["total_replies"], 5);
    let replies = thread["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 5);

    // Verify depths by content
    let find_depth = |content: &str| -> i64 {
        replies.iter().find(|r| r["content"] == content).unwrap()["depth"].as_i64().unwrap()
    };
    assert_eq!(find_depth("branch_a"), 1);
    assert_eq!(find_depth("branch_a_reply"), 2);
    assert_eq!(find_depth("branch_b"), 1);
    assert_eq!(find_depth("branch_b_reply"), 2);
    assert_eq!(find_depth("branch_b_deep"), 3);
}
