use rocket::http::Status;
use crate::common::test_client;

// --- llms.txt ---

#[test]
fn test_llms_txt_root() {
    let client = test_client();
    let res = client.get("/llms.txt").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body = res.into_string().unwrap();
    assert!(body.contains("Local Agent Chat"));
}

#[test]
fn test_llms_txt_api() {
    let client = test_client();
    let res = client.get("/api/v1/llms.txt").dispatch();
    assert_eq!(res.status(), Status::Ok);
}

// --- llms.txt content ---

#[test]
fn test_llms_txt_documents_all_sections() {
    let client = test_client();
    let res = client.get("/api/v1/llms.txt").dispatch();
    let body = res.into_string().unwrap();

    // Verify key API sections are documented
    assert!(body.contains("## Rooms"), "Should document Rooms");
    assert!(body.contains("## Messages"), "Should document Messages");
    assert!(body.contains("## Reactions"), "Should document Reactions");
    assert!(body.contains("## Profiles"), "Should document Profiles");
    assert!(body.contains("## Direct Messages"), "Should document DMs");
    assert!(body.contains("## Webhooks"), "Should document Webhooks");
    assert!(body.contains("## Search"), "Should document Search");
    assert!(body.contains("## Bookmarks"), "Should document Bookmarks");
    assert!(body.contains("## Mentions"), "Should document Mentions");
    assert!(body.contains("## Presence"), "Should document Presence");
    assert!(body.contains("## Rate Limiting"), "Should document Rate Limits");
    assert!(body.contains("## Discovery"), "Should document Discovery");
    assert!(body.contains("FTS5"), "Should mention FTS5 search");
}

// --- OpenAPI ---

#[test]
fn test_openapi_json() {
    let client = test_client();
    let res = client.get("/api/v1/openapi.json").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert_eq!(body["openapi"], "3.0.3");
    assert_eq!(body["info"]["title"], "Local Agent Chat API");
}

#[test]
fn test_openapi_has_paths() {
    let client = test_client();
    let res = client.get("/api/v1/openapi.json").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let paths = body["paths"].as_object().unwrap();
    assert!(paths.len() >= 35, "OpenAPI should document at least 35 paths, got {}", paths.len());
}

// --- Health endpoint version field ---

#[test]
fn test_health_includes_version() {
    let client = test_client();
    let res = client.get("/api/v1/health").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    assert!(body["version"].is_string(), "Health should include version field");
    assert!(!body["version"].as_str().unwrap().is_empty(), "Version should not be empty");
}

// --- SPA fallback ---

#[test]
fn test_spa_fallback_catches_unknown_paths() {
    let client = test_client();
    // SPA fallback catches all unmatched paths for frontend routing
    let res = client.get("/some/frontend/route").dispatch();
    // Returns 200 with HTML if index.html exists, or 404 if no frontend is built
    let status = res.status().code;
    assert!(status == 200 || status == 404, "SPA fallback should return 200 (with frontend) or 404 (without), got {status}");
}
