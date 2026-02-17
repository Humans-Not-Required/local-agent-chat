use rocket::http::{ContentType, Status};
use crate::common::test_client;

// Helper: create a room and return room_id
fn create_room(client: &impl std::ops::Deref<Target = rocket::local::blocking::Client>, name: &str) -> String {
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(format!(r#"{{"name": "{name}", "created_by": "tester"}}"#))
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    room["id"].as_str().unwrap().to_string()
}

// Helper: send a message and return message object
fn send_msg(
    client: &impl std::ops::Deref<Target = rocket::local::blocking::Client>,
    room_id: &str,
    sender: &str,
    content: &str,
) -> serde_json::Value {
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender": "{sender}", "content": "{content}"}}"#))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    res.into_json().unwrap()
}

// Helper: edit a message
fn edit_msg(
    client: &impl std::ops::Deref<Target = rocket::local::blocking::Client>,
    room_id: &str,
    msg_id: &str,
    sender: &str,
    content: &str,
) -> serde_json::Value {
    let res = client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(format!(r#"{{"sender": "{sender}", "content": "{content}"}}"#))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    res.into_json().unwrap()
}

#[test]
fn test_edit_history_empty_for_unedited_message() {
    let client = test_client();
    let room_id = create_room(&client, "edit-hist-empty");

    let msg = send_msg(&client, &room_id, "Nanook", "never edited");
    let msg_id = msg["id"].as_str().unwrap();

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    assert_eq!(body["message_id"], msg_id);
    assert_eq!(body["current_content"], "never edited");
    assert_eq!(body["edit_count"].as_i64().unwrap(), 0);
    assert_eq!(body["edits"].as_array().unwrap().len(), 0);
}

#[test]
fn test_edit_history_single_edit() {
    let client = test_client();
    let room_id = create_room(&client, "edit-hist-single");

    let msg = send_msg(&client, &room_id, "Nanook", "original content");
    let msg_id = msg["id"].as_str().unwrap();

    edit_msg(&client, &room_id, msg_id, "Nanook", "revised content");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    assert_eq!(body["current_content"], "revised content");
    assert_eq!(body["edit_count"].as_i64().unwrap(), 1);

    let edits = body["edits"].as_array().unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["previous_content"], "original content");
    assert_eq!(edits[0]["editor"], "Nanook");
    assert!(edits[0]["edited_at"].as_str().is_some());
    assert!(edits[0]["id"].as_str().is_some());
    assert_eq!(edits[0]["message_id"], msg_id);
}

#[test]
fn test_edit_history_multiple_edits() {
    let client = test_client();
    let room_id = create_room(&client, "edit-hist-multi");

    let msg = send_msg(&client, &room_id, "Nanook", "version 1");
    let msg_id = msg["id"].as_str().unwrap();

    edit_msg(&client, &room_id, msg_id, "Nanook", "version 2");
    edit_msg(&client, &room_id, msg_id, "Nanook", "version 3");
    edit_msg(&client, &room_id, msg_id, "Nanook", "version 4");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    assert_eq!(body["current_content"], "version 4");
    assert_eq!(body["edit_count"].as_i64().unwrap(), 3);

    let edits = body["edits"].as_array().unwrap();
    assert_eq!(edits.len(), 3);
    // Chronological order: oldest first
    assert_eq!(edits[0]["previous_content"], "version 1");
    assert_eq!(edits[1]["previous_content"], "version 2");
    assert_eq!(edits[2]["previous_content"], "version 3");
}

#[test]
fn test_edit_history_404_nonexistent_message() {
    let client = test_client();
    let room_id = create_room(&client, "edit-hist-404");

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/00000000-0000-0000-0000-000000000000/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_edit_history_404_wrong_room() {
    let client = test_client();
    let room_id = create_room(&client, "edit-hist-wrongroom-1");
    let room_id2 = create_room(&client, "edit-hist-wrongroom-2");

    let msg = send_msg(&client, &room_id, "Nanook", "in room 1");
    let msg_id = msg["id"].as_str().unwrap();

    // Try to get edit history from wrong room
    let res = client
        .get(format!("/api/v1/rooms/{room_id2}/messages/{msg_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_edit_count_in_message_response() {
    let client = test_client();
    let room_id = create_room(&client, "edit-count-resp");

    let msg = send_msg(&client, &room_id, "Nanook", "original text");
    let msg_id = msg["id"].as_str().unwrap();

    // Newly created message should have no edit_count field (skipped when 0)
    assert!(msg.get("edit_count").is_none());

    // Edit twice
    edit_msg(&client, &room_id, msg_id, "Nanook", "edit one");
    let edited = edit_msg(&client, &room_id, msg_id, "Nanook", "edit two");

    // After editing, the response should include edit_count
    assert_eq!(edited["edit_count"].as_i64().unwrap(), 2);
}

#[test]
fn test_edit_count_in_message_list() {
    let client = test_client();
    let room_id = create_room(&client, "edit-count-list");

    let msg = send_msg(&client, &room_id, "Nanook", "list original");
    let msg_id = msg["id"].as_str().unwrap();

    edit_msg(&client, &room_id, msg_id, "Nanook", "list edited");

    // Fetch messages for the room
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let messages: Vec<serde_json::Value> = res.into_json().unwrap();

    let edited_msg = messages.iter().find(|m| m["id"] == msg_id).unwrap();
    assert_eq!(edited_msg["edit_count"].as_i64().unwrap(), 1);
}

#[test]
fn test_edit_history_cascade_delete() {
    let client = test_client();
    let room_id = create_room(&client, "edit-hist-cascade");

    let msg = send_msg(&client, &room_id, "Nanook", "will be deleted");
    let msg_id = msg["id"].as_str().unwrap();

    edit_msg(&client, &room_id, msg_id, "Nanook", "edited then deleted");

    // Delete the message
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/messages/{msg_id}?sender=Nanook"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Edit history should now 404
    let res = client
        .get(format!("/api/v1/rooms/{room_id}/messages/{msg_id}/edits"))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}
