use rocket::http::Status;

use crate::common::test_client;

#[test]
fn test_discover_returns_service_info() {
    let client = test_client();
    let res = client.get("/api/v1/discover").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();

    assert_eq!(body["service"], "local-agent-chat");
    assert!(body["version"].is_string());
    assert!(body["hostname"].is_string());
    assert_eq!(body["protocol"], "http");
    assert_eq!(body["api_base"], "/api/v1");
}

#[test]
fn test_discover_has_capabilities() {
    let client = test_client();
    let res = client.get("/api/v1/discover").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();

    let caps = body["capabilities"].as_array().unwrap();
    assert!(caps.len() > 10, "should list many capabilities");

    // Check essential capabilities are present
    let cap_strs: Vec<&str> = caps.iter().map(|c| c.as_str().unwrap()).collect();
    assert!(cap_strs.contains(&"rooms"));
    assert!(cap_strs.contains(&"messages"));
    assert!(cap_strs.contains(&"direct_messages"));
    assert!(cap_strs.contains(&"sse_streaming"));
    assert!(cap_strs.contains(&"reactions"));
    assert!(cap_strs.contains(&"threads"));
    assert!(cap_strs.contains(&"mentions"));
    assert!(cap_strs.contains(&"presence"));
    assert!(cap_strs.contains(&"profiles"));
    assert!(cap_strs.contains(&"webhooks"));
    assert!(cap_strs.contains(&"search_fts5"));
}

#[test]
fn test_discover_has_endpoints() {
    let client = test_client();
    let res = client.get("/api/v1/discover").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();

    let endpoints = &body["endpoints"];
    assert_eq!(endpoints["health"], "/api/v1/health");
    assert_eq!(endpoints["rooms"], "/api/v1/rooms");
    assert_eq!(endpoints["search"], "/api/v1/search");
    assert_eq!(endpoints["activity"], "/api/v1/activity");
    assert_eq!(endpoints["profiles"], "/api/v1/profiles");
    assert_eq!(endpoints["presence"], "/api/v1/presence");
    assert_eq!(endpoints["unread"], "/api/v1/unread");
    assert_eq!(endpoints["mentions"], "/api/v1/mentions");
    assert_eq!(endpoints["dm"], "/api/v1/dm");
    assert_eq!(endpoints["discover"], "/api/v1/discover");
    assert_eq!(endpoints["openapi"], "/api/v1/openapi.json");
    assert_eq!(endpoints["llms_txt"], "/api/v1/llms.txt");
}

#[test]
fn test_discover_has_mdns_info() {
    let client = test_client();
    let res = client.get("/api/v1/discover").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();

    let mdns = &body["mdns"];
    assert_eq!(mdns["service_type"], "_agentchat._tcp.local.");
    // enabled field is a boolean
    assert!(mdns["enabled"].is_boolean());
}

#[test]
fn test_discover_has_auth_model() {
    let client = test_client();
    let res = client.get("/api/v1/discover").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();

    let auth = &body["auth"];
    assert_eq!(auth["model"], "trust-based");
    assert!(auth["description"].is_string());
}

#[test]
fn test_discover_has_rate_limits() {
    let client = test_client();
    let res = client.get("/api/v1/discover").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();

    let limits = &body["rate_limits"];
    assert_eq!(limits["messages_per_min"], 60);
    assert_eq!(limits["rooms_per_hour"], 10);
    assert_eq!(limits["files_per_min"], 10);
    assert_eq!(limits["dms_per_min"], 60);
}

#[test]
fn test_discover_has_description() {
    let client = test_client();
    let res = client.get("/api/v1/discover").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();

    assert!(body["description"].is_string());
    let desc = body["description"].as_str().unwrap();
    assert!(desc.contains("agent"), "description should mention agents");
}

#[test]
fn test_discover_port_is_number() {
    let client = test_client();
    let res = client.get("/api/v1/discover").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();

    assert!(body["port"].is_number(), "port should be a number");
}
