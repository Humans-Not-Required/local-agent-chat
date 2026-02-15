use rocket::local::blocking::Client;

/// Wrapper around Client that auto-deletes the temp DB on drop.
/// Prevents /tmp/chat_test_*.db file accumulation (was 19K+ files / 2.4GB).
/// Uses Option<Client> so we can drop the DB connection before deleting the file.
pub struct TestClient {
    client: Option<Client>,
    db_path: String,
}

impl Drop for TestClient {
    fn drop(&mut self) {
        // Drop client first to release SQLite connection (WAL mode holds the file)
        drop(self.client.take());
        let _ = std::fs::remove_file(&self.db_path);
        let _ = std::fs::remove_file(format!("{}-wal", self.db_path));
        let _ = std::fs::remove_file(format!("{}-shm", self.db_path));
    }
}

impl std::ops::Deref for TestClient {
    type Target = Client;
    fn deref(&self) -> &Client {
        self.client.as_ref().unwrap()
    }
}

pub fn test_client() -> TestClient {
    // Use unique temp DB for each test (avoids parallel test contention)
    let db_path = format!(
        "/tmp/chat_test_{}.db",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    let rocket = local_agent_chat::rocket_with_db(&db_path);
    let client = Client::tracked(rocket).expect("valid rocket instance");
    TestClient { client: Some(client), db_path }
}

/// Create a test client with custom rate limit configuration.
/// Useful for testing configurable rate limits without env var races.
pub fn test_client_with_rate_limits(config: local_agent_chat::rate_limit::RateLimitConfig) -> TestClient {
    let db_path = format!(
        "/tmp/chat_test_{}.db",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    let rocket = local_agent_chat::rocket_with_db_and_config(&db_path, config);
    let client = Client::tracked(rocket).expect("valid rocket instance");
    TestClient { client: Some(client), db_path }
}

/// Helper: create a room and return (room_id, admin_key)
pub fn create_test_room(client: &Client, name: &str) -> (String, String) {
    use rocket::http::{ContentType, Status};
    let res = client
        .post("/api/v1/rooms")
        .header(ContentType::JSON)
        .body(format!(r#"{{"name": "{name}", "created_by": "tester"}}"#))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    (
        body["id"].as_str().unwrap().to_string(),
        body["admin_key"].as_str().unwrap().to_string(),
    )
}
