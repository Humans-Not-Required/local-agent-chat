use rocket::local::blocking::Client;
use rocket::http::{ContentType, Header, Status};
use crate::common::test_client;

// --- File Attachments ---

fn get_general_room_id(client: &Client) -> String {
    let res = client.get("/api/v1/rooms").dispatch();
    let rooms: Vec<serde_json::Value> = res.into_json().unwrap();
    rooms.iter().find(|r| r["name"] == "general").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_upload_and_download_file() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let file_data = b"Hello, this is a test file!";
    let b64 = base64::engine::general_purpose::STANDARD.encode(file_data);

    // Upload
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "nanook",
                "filename": "test.txt",
                "content_type": "text/plain",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["filename"], "test.txt");
    assert_eq!(body["content_type"], "text/plain");
    assert_eq!(body["size"], file_data.len() as i64);
    assert_eq!(body["sender"], "nanook");
    assert_eq!(body["room_id"], room_id);
    let file_id = body["id"].as_str().unwrap();
    let url = body["url"].as_str().unwrap();
    assert_eq!(url, format!("/api/v1/files/{file_id}"));

    // Download
    let res = client.get(format!("/api/v1/files/{file_id}")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    let bytes = res.into_bytes().unwrap();
    assert_eq!(bytes, file_data);
}

#[test]
fn test_file_info_endpoint() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let b64 = base64::engine::general_purpose::STANDARD.encode(b"info test");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "agent1",
                "filename": "data.json",
                "content_type": "application/json",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    let upload: serde_json::Value = res.into_json().unwrap();
    let file_id = upload["id"].as_str().unwrap();

    // Get file info
    let res = client
        .get(format!("/api/v1/files/{file_id}/info"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let info: serde_json::Value = res.into_json().unwrap();
    assert_eq!(info["filename"], "data.json");
    assert_eq!(info["sender"], "agent1");
    assert_eq!(info["content_type"], "application/json");
    assert_eq!(info["size"], 9); // "info test" = 9 bytes
}

#[test]
fn test_list_files_in_room() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    // Upload 2 files
    for name in &["file1.txt", "file2.txt"] {
        let b64 = base64::engine::general_purpose::STANDARD.encode(name.as_bytes());
        client
            .post(format!("/api/v1/rooms/{room_id}/files"))
            .header(ContentType::JSON)
            .body(
                serde_json::json!({
                    "sender": "uploader",
                    "filename": name,
                    "data": b64
                })
                .to_string(),
            )
            .dispatch();
    }

    let res = client
        .get(format!("/api/v1/rooms/{room_id}/files"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let files: Vec<serde_json::Value> = res.into_json().unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn test_delete_file_by_sender() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let b64 = base64::engine::general_purpose::STANDARD.encode(b"delete me");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "owner",
                "filename": "temp.txt",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    let upload: serde_json::Value = res.into_json().unwrap();
    let file_id = upload["id"].as_str().unwrap();

    // Delete by correct sender
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/files/{file_id}?sender=owner"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // Verify gone
    let res = client.get(format!("/api/v1/files/{file_id}")).dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_delete_file_wrong_sender_forbidden() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let b64 = base64::engine::general_purpose::STANDARD.encode(b"protected");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "alice",
                "filename": "secret.txt",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    let upload: serde_json::Value = res.into_json().unwrap();
    let file_id = upload["id"].as_str().unwrap();

    // Wrong sender
    let res = client
        .delete(format!(
            "/api/v1/rooms/{room_id}/files/{file_id}?sender=bob"
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Forbidden);
}

#[test]
fn test_delete_file_with_admin_key() {
    use base64::Engine;
    let client = test_client();

    // Create a room to get admin key
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(r#"{"name": "file-test-room"}"#)
        .dispatch();
    let room: serde_json::Value = res.into_json().unwrap();
    let room_id = room["id"].as_str().unwrap();
    let admin_key = room["admin_key"].as_str().unwrap();

    // Upload a file
    let b64 = base64::engine::general_purpose::STANDARD.encode(b"admin delete");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "someone",
                "filename": "moderated.txt",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    let upload: serde_json::Value = res.into_json().unwrap();
    let file_id = upload["id"].as_str().unwrap();

    // Delete with admin key (different sender)
    let res = client
        .delete(format!("/api/v1/rooms/{room_id}/files/{file_id}"))
        .header(Header::new("Authorization", format!("Bearer {admin_key}")))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
}

#[test]
fn test_upload_file_invalid_base64() {
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "nanook",
                "filename": "bad.txt",
                "data": "not-valid-base64!!!"
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("base64"));
}

#[test]
fn test_upload_file_empty_sender() {
    let client = test_client();
    let room_id = get_general_room_id(&client);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "",
                "filename": "test.txt",
                "data": "aGVsbG8="
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
}

#[test]
fn test_upload_file_nonexistent_room() {
    let client = test_client();

    let res = client
        .post("/api/v1/rooms/nonexistent-room-id/files")
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "nanook",
                "filename": "test.txt",
                "data": "aGVsbG8="
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_upload_file_too_large() {
    use base64::Engine;
    let client = test_client();
    let room_id = get_general_room_id(&client);

    // Create a 6MB payload (over 5MB limit)
    let big_data = vec![0u8; 6 * 1024 * 1024];
    let b64 = base64::engine::general_purpose::STANDARD.encode(&big_data);

    let res = client
        .post(format!("/api/v1/rooms/{room_id}/files"))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "sender": "nanook",
                "filename": "huge.bin",
                "data": b64
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("too large"));
}

#[test]
fn test_list_files_nonexistent_room() {
    let client = test_client();
    let res = client.get("/api/v1/rooms/fake-room-id/files").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_download_nonexistent_file() {
    let client = test_client();
    let res = client.get("/api/v1/files/nonexistent-file-id").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}
