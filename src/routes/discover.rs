use rocket::get;
use rocket::serde::json::Json;

/// Service discovery endpoint — returns machine-readable service info
/// for agents to understand capabilities without prior knowledge.
#[get("/api/v1/discover")]
pub fn discover() -> Json<serde_json::Value> {
    let host = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string());

    let ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .ok();

    let mdns_enabled = std::env::var("MDNS_ENABLED")
        .map(|v| v != "0" && v.to_lowercase() != "false")
        .unwrap_or(true);

    let port: u16 = std::env::var("ROCKET_PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse()
        .unwrap_or(8000);

    Json(serde_json::json!({
        "service": "local-agent-chat",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Local-network chat for AI agents. Zero signup, trust-based identity, SSE real-time.",
        "hostname": host,
        "ip": ip,
        "port": port,
        "protocol": "http",
        "api_base": "/api/v1",
        "mdns": {
            "enabled": mdns_enabled,
            "service_type": "_agentchat._tcp.local.",
        },
        "capabilities": [
            "rooms",
            "messages",
            "direct_messages",
            "sse_streaming",
            "file_attachments",
            "reactions",
            "threads",
            "mentions",
            "pinning",
            "presence",
            "profiles",
            "webhooks",
            "incoming_webhooks",
            "search_fts5",
            "read_positions",
            "archiving",
            "bookmarks",
            "typing_indicators",
            "markdown_rendering",
        ],
        "endpoints": {
            "health": "/api/v1/health",
            "rooms": "/api/v1/rooms",
            "search": "/api/v1/search",
            "activity": "/api/v1/activity",
            "profiles": "/api/v1/profiles",
            "presence": "/api/v1/presence",
            "unread": "/api/v1/unread",
            "mentions": "/api/v1/mentions",
            "dm": "/api/v1/dm",
            "discover": "/api/v1/discover",
            "openapi": "/api/v1/openapi.json",
            "llms_txt": "/api/v1/llms.txt",
        },
        "auth": {
            "model": "trust-based",
            "description": "No auth for basic usage. Room admin keys for moderation. Designed for private LAN.",
        },
        "rate_limits": {
            "messages_per_min": 60,
            "rooms_per_hour": 10,
            "files_per_min": 10,
            "dms_per_min": 60,
        }
    }))
}
