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
    sender_type: Option<&str>,
) -> serde_json::Value {
    let st = sender_type.unwrap_or("agent");
    let res = client
        .post(format!("/api/v1/rooms/{room_id}/messages"))
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"sender": "{sender}", "content": "{content}", "sender_type": "{st}"}}"#
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    res.into_json().unwrap()
}

// --- FTS5 Search: Basic ---

#[test]
fn test_search_fts5_word_matching() {
    let client = test_client();
    let room_id = create_room(&client, "fts-word-match");

    send_msg(&client, &room_id, "Nanook", "The frobulation process completed successfully", None);
    send_msg(&client, &room_id, "Forge", "Starting frobulation on all servers now", None);
    send_msg(&client, &room_id, "Drift", "The weather is nice today", None);

    // FTS5 word matching: "frobulation" should find exactly the 2 messages containing it
    let res = client.get("/api/v1/search?q=frobulation").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);

    // "weather" should find only 1 message
    let res = client.get("/api/v1/search?q=weather").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_fts5_multi_word() {
    let client = test_client();
    let room_id = create_room(&client, "fts-multi-word");

    send_msg(&client, &room_id, "Nanook", "The API test results look good", None);
    send_msg(&client, &room_id, "Forge", "Running API integration tests now", None);
    send_msg(&client, &room_id, "Drift", "The weather API is down", None);

    // Multi-word search: "API test" should match messages with both terms
    let res = client.get("/api/v1/search?q=API%20test").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Both first two messages contain "API" and "test" (via stemming)
    assert_eq!(body["count"].as_u64().unwrap(), 2);
}

#[test]
fn test_search_fts5_edited_message() {
    let client = test_client();
    let room_id = create_room(&client, "fts-edit");

    // Send a message
    let msg = send_msg(&client, &room_id, "Nanook", "original content here", None);
    let msg_id = msg["id"].as_str().unwrap();

    // Verify "original" is searchable
    let res = client.get("/api/v1/search?q=original").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);

    // Edit the message
    client
        .put(format!("/api/v1/rooms/{room_id}/messages/{msg_id}"))
        .header(ContentType::JSON)
        .body(r#"{"sender": "Nanook", "content": "updated content instead"}"#)
        .dispatch();

    // Old content should no longer be searchable
    let res = client.get("/api/v1/search?q=original").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);

    // New content should be searchable
    let res = client.get("/api/v1/search?q=updated").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_fts5_deleted_message() {
    let client = test_client();
    let room_id = create_room(&client, "fts-delete");

    // Send a message
    let msg = send_msg(&client, &room_id, "Nanook", "ephemeral message to delete", None);
    let msg_id = msg["id"].as_str().unwrap();

    // Verify it's searchable
    let res = client.get("/api/v1/search?q=ephemeral").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);

    // Delete the message
    client
        .delete(format!(
            "/api/v1/rooms/{room_id}/messages/{msg_id}?sender=Nanook"
        ))
        .dispatch();

    // Should no longer be searchable
    let res = client.get("/api/v1/search?q=ephemeral").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);
}

#[test]
fn test_search_fts5_sender_search() {
    let client = test_client();
    let room_id = create_room(&client, "fts-sender-search");

    send_msg(&client, &room_id, "Nanook", "hello from nanook", None);
    send_msg(&client, &room_id, "Forge", "hello from forge", None);

    // FTS5 indexes sender too — searching for a sender name matches content or sender
    let res = client.get("/api/v1/search?q=nanook").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Should find the message from Nanook (matches sender in FTS)
    assert!(body["count"].as_u64().unwrap() >= 1);
}

// --- FTS5 Search: Stemming ---

#[test]
fn test_search_fts5_porter_stemming() {
    let client = test_client();
    let room_id = create_room(&client, "fts-stemming");

    send_msg(&client, &room_id, "Nanook", "deploying the service now", None);
    send_msg(&client, &room_id, "Forge", "deployment completed yesterday", None);
    send_msg(&client, &room_id, "Drift", "I deployed it last week", None);
    send_msg(&client, &room_id, "Lux", "The weather is sunny", None);

    // Porter stemmer: "deploy" matches deploying + deployed (same stem)
    // Note: "deployment" has a different porter stem — this is expected FTS5 behavior
    let res = client.get("/api/v1/search?q=deploy").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2, "deploy/deploying/deployed share a porter stem");

    // "deployed" matches the same set as "deploy" (same stem)
    let res = client.get("/api/v1/search?q=deployed").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);

    // "deployment" matches only "deployment" (different porter stem)
    let res = client.get("/api/v1/search?q=deployment").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_fts5_stemming_run_running_ran() {
    let client = test_client();
    let room_id = create_room(&client, "fts-stem2");

    send_msg(&client, &room_id, "Nanook", "running the tests now", None);
    send_msg(&client, &room_id, "Forge", "we ran the suite yesterday", None);
    send_msg(&client, &room_id, "Drift", "please run the checks", None);
    send_msg(&client, &room_id, "Lux", "nothing related here", None);

    // "run" should match run/running/ran via porter stemming
    let res = client.get("/api/v1/search?q=run").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Porter stemmer handles common verb forms
    assert!(body["count"].as_u64().unwrap() >= 2, "Should match at least run + running");
}

// --- FTS5 Search: Filters ---

#[test]
fn test_search_room_id_filter() {
    let client = test_client();
    let room_a = create_room(&client, "search-room-a");
    let room_b = create_room(&client, "search-room-b");

    send_msg(&client, &room_a, "Nanook", "important update about servers", None);
    send_msg(&client, &room_b, "Forge", "another important update here", None);

    // Search without room filter — both rooms
    let res = client.get("/api/v1/search?q=important").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);

    // Search with room_id filter — only room_a
    let res = client
        .get(format!("/api/v1/search?q=important&room_id={room_a}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(body["results"][0]["room_id"], room_a);

    // Search with room_id filter — only room_b
    let res = client
        .get(format!("/api/v1/search?q=important&room_id={room_b}"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(body["results"][0]["room_id"], room_b);
}

#[test]
fn test_search_sender_filter() {
    let client = test_client();
    let room_id = create_room(&client, "search-sender");

    send_msg(&client, &room_id, "Nanook", "the infrastructure is ready", None);
    send_msg(&client, &room_id, "Forge", "the infrastructure needs work", None);
    send_msg(&client, &room_id, "Drift", "the infrastructure looks good", None);

    // Search with sender filter — only Nanook's messages
    let res = client
        .get("/api/v1/search?q=infrastructure&sender=Nanook")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(body["results"][0]["sender"], "Nanook");

    // Sender filter is exact match — "nanook" (lowercase) shouldn't match "Nanook"
    let res = client
        .get("/api/v1/search?q=infrastructure&sender=nanook")
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    // SQLite sender column is case-sensitive for exact match
    assert_eq!(body["count"].as_u64().unwrap(), 0);
}

#[test]
fn test_search_sender_type_filter() {
    let client = test_client();
    let room_id = create_room(&client, "search-sender-type");

    send_msg(&client, &room_id, "Nanook", "monitoring server health", Some("agent"));
    send_msg(&client, &room_id, "Jordan", "checking server health too", Some("human"));
    send_msg(&client, &room_id, "Forge", "server health is nominal", Some("agent"));

    // Filter by agent sender_type
    let res = client
        .get("/api/v1/search?q=server%20health&sender_type=agent")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    for result in body["results"].as_array().unwrap() {
        assert_eq!(result["sender_type"], "agent");
    }

    // Filter by human sender_type
    let res = client
        .get("/api/v1/search?q=server%20health&sender_type=human")
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(body["results"][0]["sender"], "Jordan");
}

#[test]
fn test_search_limit_parameter() {
    let client = test_client();
    let room_id = create_room(&client, "search-limit");

    // Send 5 messages with the same keyword
    for i in 1..=5 {
        send_msg(&client, &room_id, "Nanook", &format!("batch message number {i}"), None);
    }

    // Default limit (50) — should return all 5
    let res = client.get("/api/v1/search?q=batch").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 5);

    // Limit to 2
    let res = client.get("/api/v1/search?q=batch&limit=2").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);

    // Limit to 1
    let res = client.get("/api/v1/search?q=batch&limit=1").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_combined_filters() {
    let client = test_client();
    let room_a = create_room(&client, "search-combo-a");
    let room_b = create_room(&client, "search-combo-b");

    send_msg(&client, &room_a, "Nanook", "database migration complete", Some("agent"));
    send_msg(&client, &room_a, "Jordan", "database migration reviewed", Some("human"));
    send_msg(&client, &room_b, "Nanook", "database migration started", Some("agent"));
    send_msg(&client, &room_b, "Forge", "database backup done", Some("agent"));

    // Combine room_id + sender_type: agent messages about migration in room_a
    let res = client
        .get(format!("/api/v1/search?q=migration&room_id={room_a}&sender_type=agent"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(body["results"][0]["sender"], "Nanook");
    assert_eq!(body["results"][0]["room_id"], room_a);

    // Combine sender + room_id: Nanook's messages in room_b
    let res = client
        .get(format!("/api/v1/search?q=database&room_id={room_b}&sender=Nanook"))
        .dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert!(body["results"][0]["content"].as_str().unwrap().contains("started"));
}

// --- FTS5 Search: Error Handling ---

#[test]
fn test_search_empty_query_returns_400() {
    let client = test_client();

    let res = client.get("/api/v1/search?q=").dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("empty"));
}

#[test]
fn test_search_whitespace_only_query_returns_400() {
    let client = test_client();

    let res = client.get("/api/v1/search?q=%20%20%20").dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("empty"));
}

#[test]
fn test_search_query_too_long_returns_400() {
    let client = test_client();

    // 501 characters should exceed the 500 char limit
    let long_query = "a".repeat(501);
    let res = client
        .get(format!("/api/v1/search?q={long_query}"))
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("long"));
}

// --- FTS5 Search: Edge Cases ---

#[test]
fn test_search_no_results() {
    let client = test_client();
    let room_id = create_room(&client, "search-no-results");

    send_msg(&client, &room_id, "Nanook", "hello world", None);

    let res = client
        .get("/api/v1/search?q=xyznonexistentterm")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);
    assert!(body["results"].as_array().unwrap().is_empty());
    assert_eq!(body["query"], "xyznonexistentterm");
}

#[test]
fn test_search_special_characters_handled() {
    let client = test_client();
    let room_id = create_room(&client, "search-special");

    send_msg(&client, &room_id, "Nanook", "error in config.yaml: missing key", None);
    send_msg(&client, &room_id, "Forge", "check the config file please", None);

    // Special chars like : and . should not break FTS5 (they get stripped)
    let res = client.get("/api/v1/search?q=config").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
}

#[test]
fn test_search_results_include_room_name() {
    let client = test_client();
    let room_id = create_room(&client, "named-search-room");

    send_msg(&client, &room_id, "Nanook", "findable message here", None);

    let res = client.get("/api/v1/search?q=findable").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
    assert_eq!(body["results"][0]["room_name"], "named-search-room");
    assert_eq!(body["results"][0]["room_id"], room_id);
    // Verify seq field is present
    assert!(body["results"][0]["seq"].as_i64().is_some());
}

#[test]
fn test_search_cross_room() {
    let client = test_client();
    let room_a = create_room(&client, "cross-a");
    let room_b = create_room(&client, "cross-b");
    let room_c = create_room(&client, "cross-c");

    send_msg(&client, &room_a, "Nanook", "critical alert in production", None);
    send_msg(&client, &room_b, "Forge", "critical bug found", None);
    send_msg(&client, &room_c, "Drift", "nothing to see here", None);

    // Cross-room search (no room_id filter) should find results from multiple rooms
    let res = client.get("/api/v1/search?q=critical").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);

    let room_names: Vec<&str> = body["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["room_name"].as_str().unwrap())
        .collect();
    assert!(room_names.contains(&"cross-a"));
    assert!(room_names.contains(&"cross-b"));
}

#[test]
fn test_search_limit_clamped() {
    let client = test_client();
    let room_id = create_room(&client, "search-clamp");

    for i in 1..=3 {
        send_msg(&client, &room_id, "Nanook", &format!("clamp test message {i}"), None);
    }

    // Limit 0 should be clamped to 1
    let res = client.get("/api/v1/search?q=clamp&limit=0").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);

    // Negative limit should also clamp to 1
    let res = client.get("/api/v1/search?q=clamp&limit=-5").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);
}

#[test]
fn test_search_nonexistent_room_filter() {
    let client = test_client();
    let room_id = create_room(&client, "search-exists");

    send_msg(&client, &room_id, "Nanook", "findable content", None);

    // Filter by a room_id that doesn't exist — should return 0 results
    let res = client
        .get("/api/v1/search?q=findable&room_id=00000000-0000-0000-0000-000000000000")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);
}

#[test]
fn test_search_query_field_in_response() {
    let client = test_client();
    let room_id = create_room(&client, "search-query-echo");

    send_msg(&client, &room_id, "Nanook", "echo test message", None);

    let res = client.get("/api/v1/search?q=echo%20test").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Response should echo back the query
    assert_eq!(body["query"], "echo test");
}

#[test]
fn test_search_result_fields_complete() {
    let client = test_client();
    let room_id = create_room(&client, "search-fields");

    let msg = send_msg(&client, &room_id, "Nanook", "complete field check message", Some("agent"));

    let res = client
        .get("/api/v1/search?q=complete%20field%20check")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);

    let result = &body["results"][0];
    assert_eq!(result["message_id"], msg["id"]);
    assert_eq!(result["room_id"], room_id);
    assert_eq!(result["room_name"], "search-fields");
    assert_eq!(result["sender"], "Nanook");
    assert_eq!(result["sender_type"], "agent");
    assert_eq!(result["content"], "complete field check message");
    assert!(result["created_at"].as_str().is_some());
    assert!(result["seq"].as_i64().is_some());
}

// --- Search Pagination ---

#[test]
fn test_search_has_more_false_when_under_limit() {
    let client = test_client();
    let room_id = create_room(&client, "search-hasmore-false");

    send_msg(&client, &room_id, "Nanook", "pagination alpha test", None);
    send_msg(&client, &room_id, "Nanook", "pagination alpha check", None);

    let res = client.get("/api/v1/search?q=pagination%20alpha&limit=10").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    assert!(!body["has_more"].as_bool().unwrap());
}

#[test]
fn test_search_has_more_true_when_over_limit() {
    let client = test_client();
    let room_id = create_room(&client, "search-hasmore-true");

    for i in 1..=5 {
        send_msg(&client, &room_id, "Nanook", &format!("overflow beta item {i}"), None);
    }

    let res = client.get("/api/v1/search?q=overflow%20beta&limit=3").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 3);
    assert!(body["has_more"].as_bool().unwrap());
}

#[test]
fn test_search_cursor_after_seq() {
    let client = test_client();
    let room_id = create_room(&client, "search-cursor-after");

    let mut seqs = Vec::new();
    for i in 1..=5 {
        let msg = send_msg(&client, &room_id, "Nanook", &format!("gamma cursor item {i}"), None);
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Search with after=seq[2] should only return items 4 and 5
    let res = client
        .get(format!("/api/v1/search?q=gamma%20cursor&after={}", seqs[2]))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    let results = body["results"].as_array().unwrap();
    for r in results {
        assert!(r["seq"].as_i64().unwrap() > seqs[2]);
    }
}

#[test]
fn test_search_cursor_before_seq() {
    let client = test_client();
    let room_id = create_room(&client, "search-cursor-before");

    let mut seqs = Vec::new();
    for i in 1..=5 {
        let msg = send_msg(&client, &room_id, "Nanook", &format!("delta cursor item {i}"), None);
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Search with before_seq=seq[3] should exclude items 4 and 5
    let res = client
        .get(format!("/api/v1/search?q=delta%20cursor&before_seq={}", seqs[3]))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 3);
    let results = body["results"].as_array().unwrap();
    for r in results {
        assert!(r["seq"].as_i64().unwrap() < seqs[3]);
    }
}

#[test]
fn test_search_cursor_after_and_before_seq_window() {
    let client = test_client();
    let room_id = create_room(&client, "search-cursor-window");

    let mut seqs = Vec::new();
    for i in 1..=6 {
        let msg = send_msg(&client, &room_id, "Nanook", &format!("epsilon window item {i}"), None);
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // after=seq[1] AND before_seq=seq[4] should return items 3 and 4 (seq[2], seq[3])
    let res = client
        .get(format!(
            "/api/v1/search?q=epsilon%20window&after={}&before_seq={}",
            seqs[1], seqs[4]
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    let results = body["results"].as_array().unwrap();
    for r in results {
        let seq = r["seq"].as_i64().unwrap();
        assert!(seq > seqs[1] && seq < seqs[4]);
    }
}

// --- Search Date Filtering ---

#[test]
fn test_search_after_date_filter() {
    let client = test_client();
    let room_id = create_room(&client, "search-after-date");

    send_msg(&client, &room_id, "Nanook", "zeta date test one", None);
    send_msg(&client, &room_id, "Nanook", "zeta date test two", None);

    // Use a date far in the past — should return all results
    let res = client
        .get("/api/v1/search?q=zeta%20date&after_date=2020-01-01T00:00:00Z")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
    assert_eq!(body["after_date"], "2020-01-01T00:00:00Z");
}

#[test]
fn test_search_before_date_filter() {
    let client = test_client();
    let room_id = create_room(&client, "search-before-date");

    send_msg(&client, &room_id, "Nanook", "eta date test recent", None);

    // Use a date far in the past — should return no results
    let res = client
        .get("/api/v1/search?q=eta%20date&before_date=2020-01-01T00:00:00Z")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);
    assert_eq!(body["before_date"], "2020-01-01T00:00:00Z");
}

#[test]
fn test_search_future_before_date_returns_all() {
    let client = test_client();
    let room_id = create_room(&client, "search-future-date");

    send_msg(&client, &room_id, "Nanook", "theta future test one", None);
    send_msg(&client, &room_id, "Nanook", "theta future test two", None);

    // Use a date far in the future — should return all results
    let res = client
        .get("/api/v1/search?q=theta%20future&before_date=2099-12-31T23:59:59Z")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 2);
}

#[test]
fn test_search_date_range_combined() {
    let client = test_client();
    let room_id = create_room(&client, "search-date-range");

    send_msg(&client, &room_id, "Nanook", "iota range combined test", None);

    // Date range that encompasses now should find the message
    let res = client
        .get("/api/v1/search?q=iota%20range&after_date=2020-01-01T00:00:00Z&before_date=2099-12-31T23:59:59Z")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 1);

    // Date range that excludes the message (all in the past)
    let res = client
        .get("/api/v1/search?q=iota%20range&after_date=2020-01-01T00:00:00Z&before_date=2020-12-31T23:59:59Z")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 0);
}

#[test]
fn test_search_pagination_with_cursor_and_limit() {
    let client = test_client();
    let room_id = create_room(&client, "search-paginate-combo");

    let mut seqs = Vec::new();
    for i in 1..=8 {
        let msg = send_msg(&client, &room_id, "Nanook", &format!("kappa paginate item {i}"), None);
        seqs.push(msg["seq"].as_i64().unwrap());
    }

    // Page 1: first 3 results
    let res = client.get("/api/v1/search?q=kappa%20paginate&limit=3").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 3);
    assert!(body["has_more"].as_bool().unwrap());

    // Page 2: use cursor from last result to get next batch
    // FTS5 uses rank ordering so we just verify cursor works for LIKE fallback
    // by also using a before_seq to constrain the window
    let last_seq = body["results"].as_array().unwrap().last().unwrap()["seq"].as_i64().unwrap();

    // The after cursor combined with limit should paginate through results
    let res = client
        .get(format!("/api/v1/search?q=kappa%20paginate&limit=3&after={last_seq}"))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    // Should have results (the cursor narrows the window)
    assert!(body["count"].as_u64().unwrap() > 0);
}

#[test]
fn test_search_date_filters_reflected_in_response() {
    let client = test_client();
    let room_id = create_room(&client, "search-date-resp");

    send_msg(&client, &room_id, "Nanook", "lambda date response check", None);

    let res = client
        .get("/api/v1/search?q=lambda%20date&after_date=2025-01-01&before_date=2027-01-01")
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["after_date"], "2025-01-01");
    assert_eq!(body["before_date"], "2027-01-01");
}

#[test]
fn test_search_has_more_with_exact_limit() {
    let client = test_client();
    let room_id = create_room(&client, "search-exact-limit");

    for i in 1..=3 {
        send_msg(&client, &room_id, "Nanook", &format!("mu exact limit item {i}"), None);
    }

    // Limit equals exact count — has_more should be false
    let res = client.get("/api/v1/search?q=mu%20exact%20limit&limit=3").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["count"].as_u64().unwrap(), 3);
    assert!(!body["has_more"].as_bool().unwrap());
}
