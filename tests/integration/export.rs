use rocket::http::{ContentType, Status};
use crate::common::{test_client, create_test_room};

/// Helper: send a message and return the response body
fn send_msg(client: &rocket::local::blocking::Client, room_id: &str, sender: &str, content: &str) {
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "{sender}", "content": "{content}", "sender_type": "agent"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
}

#[test]
fn test_export_json_default() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-json");

    send_msg(&client, &room_id, "alice", "Hello world");
    send_msg(&client, &room_id, "bob", "Hi alice!");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["room_name"], "export-json");
    assert_eq!(body["room_id"], room_id);
    assert_eq!(body["message_count"], 2);
    assert!(body["exported_at"].is_string());
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);

    // Messages should be in chronological order (ascending seq)
    let msgs = body["messages"].as_array().unwrap();
    assert_eq!(msgs[0]["sender"], "alice");
    assert_eq!(msgs[0]["content"], "Hello world");
    assert_eq!(msgs[1]["sender"], "bob");
    assert_eq!(msgs[1]["content"], "Hi alice!");

    // seq should be present and ascending
    let seq0 = msgs[0]["seq"].as_i64().unwrap();
    let seq1 = msgs[1]["seq"].as_i64().unwrap();
    assert!(seq1 > seq0);
}

#[test]
fn test_export_json_explicit_format() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-json-fmt");

    send_msg(&client, &room_id, "alice", "test msg");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=json"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message_count"], 1);
}

#[test]
fn test_export_markdown() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-md");

    send_msg(&client, &room_id, "alice", "Hello from alice");
    send_msg(&client, &room_id, "bob", "Reply from bob");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=markdown"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let content_type = res.content_type().unwrap();
    assert_eq!(content_type.top().as_str(), "text");
    assert_eq!(content_type.sub().as_str(), "markdown");

    let body = res.into_string().unwrap();
    assert!(body.contains("# #export-md"));
    assert!(body.contains("alice"));
    assert!(body.contains("Hello from alice"));
    assert!(body.contains("bob"));
    assert!(body.contains("Reply from bob"));
    assert!(body.contains("🤖")); // agent badge
}

#[test]
fn test_export_csv() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-csv");

    send_msg(&client, &room_id, "alice", "message one");
    send_msg(&client, &room_id, "bob", "message two");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=csv"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let content_type = res.content_type().unwrap();
    assert!(content_type.to_string().starts_with("text/csv"));

    let body = res.into_string().unwrap();
    let lines: Vec<&str> = body.lines().collect();

    // Header line
    assert_eq!(
        lines[0],
        "seq,sender,sender_type,content,created_at,edited_at,reply_to,pinned_at"
    );
    // 2 data rows
    assert_eq!(lines.len(), 3);
    assert!(lines[1].contains("alice"));
    assert!(lines[1].contains("message one"));
    assert!(lines[2].contains("bob"));
    assert!(lines[2].contains("message two"));
}

#[test]
fn test_export_sender_filter() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-filter-sender");

    send_msg(&client, &room_id, "alice", "msg 1");
    send_msg(&client, &room_id, "bob", "msg 2");
    send_msg(&client, &room_id, "alice", "msg 3");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?sender=alice"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message_count"], 2);
    let msgs = body["messages"].as_array().unwrap();
    assert!(msgs.iter().all(|m| m["sender"] == "alice"));
}

#[test]
fn test_export_limit() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-limit");

    for i in 0..10 {
        send_msg(&client, &room_id, "sender", &format!("msg {i}"));
    }

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?limit=3"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message_count"], 3);

    // Should get the first 3 messages (oldest, since ORDER BY seq ASC)
    let msgs = body["messages"].as_array().unwrap();
    assert_eq!(msgs[0]["content"], "msg 0");
    assert_eq!(msgs[2]["content"], "msg 2");
}

#[test]
fn test_export_limit_clamped() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-limit-clamp");

    send_msg(&client, &room_id, "alice", "msg");

    // Limit above max (10000) should be clamped
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?limit=99999"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message_count"], 1);
    // Filters should reflect the requested limit (server clamps internally)
    assert_eq!(body["filters"]["limit"], 99999);
}

#[test]
fn test_export_include_metadata() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-metadata");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "with meta", "metadata": {"key": "value"}}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Without include_metadata (default: metadata should be absent)
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let msgs = body["messages"].as_array().unwrap();
    assert!(msgs[0]["metadata"].is_null());

    // With include_metadata=true
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/export?include_metadata=true"
        ))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let msgs = body["messages"].as_array().unwrap();
    assert_eq!(msgs[0]["metadata"]["key"], "value");
}

#[test]
fn test_export_nonexistent_room() {
    let client = test_client();

    let res = client
        .get("/api/v1/rooms/nonexistent-room-id/export")
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["error"], "Room not found");
}

#[test]
fn test_export_invalid_format() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-bad-format");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=xml"))
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);

    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("Invalid format"));
}

#[test]
fn test_export_empty_room() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-empty");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["message_count"], 0);
    assert_eq!(body["messages"].as_array().unwrap().len(), 0);
}

#[test]
fn test_export_csv_with_commas_in_content() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-csv-escape");

    // Send message with commas and quotes
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "hello, world, \"quoted\""}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=csv"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body = res.into_string().unwrap();
    // The content field should be properly escaped (wrapped in quotes)
    assert!(body.contains("\"hello, world,"));
}

#[test]
fn test_export_markdown_with_pins_and_edits() {
    let client = test_client();
    let (room_id, admin_key) = create_test_room(&client, "export-md-pins");

    // Send a message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "pin me"}"#)
        .dispatch();
    let msg: serde_json::Value = res.into_json().unwrap();
    let msg_id = msg["id"].as_str().unwrap();

    // Pin it
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/pin"))
        .header(rocket::http::Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Edit a second message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "content": "edit me"}"#)
        .dispatch();
    let msg2: serde_json::Value = res.into_json().unwrap();
    let msg2_id = msg2["id"].as_str().unwrap();

    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg2_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bob", "content": "edited content"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Export as markdown
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=markdown"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body = res.into_string().unwrap();
    assert!(body.contains("📌")); // pin marker
    assert!(body.contains("*(edited)*")); // edit marker
}

#[test]
fn test_export_json_filters_in_response() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-filters-resp");

    send_msg(&client, &room_id, "alice", "test");

    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/export?sender=alice&limit=5"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["filters"]["sender"], "alice");
    assert_eq!(body["filters"]["limit"], 5);
    assert!(body["filters"]["after"].is_null());
    assert!(body["filters"]["before"].is_null());
}

#[test]
fn test_export_content_disposition_headers() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-headers");

    send_msg(&client, &room_id, "alice", "msg");

    // JSON format
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=json"))
        .dispatch();
    let cd = res.headers().get_one("Content-Disposition").unwrap();
    assert!(cd.contains("chat-export.json"));

    // Markdown format
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=markdown"))
        .dispatch();
    let cd = res.headers().get_one("Content-Disposition").unwrap();
    assert!(cd.contains("chat-export.md"));

    // CSV format
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=csv"))
        .dispatch();
    let cd = res.headers().get_one("Content-Disposition").unwrap();
    assert!(cd.contains("chat-export.csv"));
}

#[test]
fn test_export_with_replies() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-replies");

    // Send parent message
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "parent"}"#)
        .dispatch();
    let parent: serde_json::Value = res.into_json().unwrap();
    let parent_id = parent["id"].as_str().unwrap();

    // Send reply
    client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "bob", "content": "reply", "reply_to": "{parent_id}"}}"#
        ))
        .dispatch();

    // Export JSON
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let msgs = body["messages"].as_array().unwrap();

    assert!(msgs[0]["reply_to"].is_null());
    assert_eq!(msgs[1]["reply_to"], parent_id);

    // Export markdown should show reply indicator
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=markdown"))
        .dispatch();
    let md = res.into_string().unwrap();
    assert!(md.contains("↩"));
}

#[test]
fn test_export_csv_with_metadata() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-csv-meta");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "alice", "content": "msg", "metadata": {"tool": "curl"}}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Without metadata — header should NOT have metadata column
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=csv"))
        .dispatch();
    let body = res.into_string().unwrap();
    let header = body.lines().next().unwrap();
    assert!(!header.contains("metadata"));

    // With metadata — header SHOULD have metadata column
    let res = client
        .get(format!(
            "/api/v1/rooms/{room_id}/export?format=csv&include_metadata=true"
        ))
        .dispatch();
    let body = res.into_string().unwrap();
    let header = body.lines().next().unwrap();
    assert!(header.contains("metadata"));

    let data_line = body.lines().nth(1).unwrap();
    assert!(data_line.contains("tool"));
}

#[test]
fn test_export_sender_type_field() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-sender-type");

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "bot1", "content": "robot msg", "sender_type": "agent"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "human1", "content": "human msg", "sender_type": "human"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let msgs = body["messages"].as_array().unwrap();

    assert_eq!(msgs[0]["sender_type"], "agent");
    assert_eq!(msgs[1]["sender_type"], "human");
}

#[test]
fn test_export_markdown_date_headers() {
    let client = test_client();
    let (room_id, _) = create_test_room(&client, "export-md-dates");

    // Send a few messages (all will be same date since they're created now)
    send_msg(&client, &room_id, "alice", "msg 1");
    send_msg(&client, &room_id, "bob", "msg 2");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/export?format=markdown"))
        .dispatch();
    let body = res.into_string().unwrap();

    // Should have a date header (## YYYY-MM-DD)
    assert!(body.contains("## 2026-02-"));
    // Should have timestamps [HH:MM:SS]
    assert!(body.contains("["));
    assert!(body.contains("]"));
}
