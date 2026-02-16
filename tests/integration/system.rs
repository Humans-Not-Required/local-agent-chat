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

// --- Well-Known Skills Discovery ---

#[test]
fn test_skills_index_json() {
    let client = test_client();
    let res = client.get("/.well-known/skills/index.json").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().unwrap();
    let skills = body["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "local-agent-chat");
    assert!(skills[0]["description"].as_str().unwrap().len() > 20, "Description should be meaningful");
    let files = skills[0]["files"].as_array().unwrap();
    assert!(files.contains(&serde_json::json!("SKILL.md")));
}

#[test]
fn test_skills_skill_md() {
    let client = test_client();
    let res = client.get("/.well-known/skills/local-agent-chat/SKILL.md").dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body = res.into_string().unwrap();
    // YAML frontmatter
    assert!(body.starts_with("---"), "SKILL.md should start with YAML frontmatter");
    assert!(body.contains("name: local-agent-chat"), "Should contain name field");
    assert!(body.contains("description:"), "Should contain description field");
    // Key content sections
    assert!(body.contains("## Quick Start"), "Should have Quick Start section");
    assert!(body.contains("## Core Patterns"), "Should have Core Patterns section");
    assert!(body.contains("## Auth Model"), "Should document auth model");
    assert!(body.contains("## Rate Limits"), "Should document rate limits");
    assert!(body.contains("## SSE Event Types"), "Should document SSE events");
    assert!(body.contains("## Gotchas"), "Should have gotchas section");
    assert!(body.contains("llms.txt"), "Should reference llms.txt for full API docs");
}

#[test]
fn test_skills_index_name_matches_spec() {
    // Per agentskills.io spec: name must be lowercase, hyphens only, 1-64 chars
    let client = test_client();
    let res = client.get("/.well-known/skills/index.json").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let name = body["skills"][0]["name"].as_str().unwrap();
    assert!(name.len() <= 64, "Name must be <= 64 chars");
    assert!(name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
        "Name must be lowercase alphanumeric + hyphens");
    assert!(!name.starts_with('-') && !name.ends_with('-'), "Name must not start/end with hyphen");
    assert!(!name.contains("--"), "Name must not contain consecutive hyphens");
}

#[test]
fn test_skills_description_within_spec_limits() {
    // Per agentskills.io spec: description must be 1-1024 chars
    let client = test_client();
    let res = client.get("/.well-known/skills/index.json").dispatch();
    let body: serde_json::Value = res.into_json().unwrap();
    let desc = body["skills"][0]["description"].as_str().unwrap();
    assert!(!desc.is_empty(), "Description must not be empty");
    assert!(desc.len() <= 1024, "Description must be <= 1024 chars");
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
