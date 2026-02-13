use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomWithStats {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
    pub last_activity: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub room_id: String,
    pub sender: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_type: Option<String>,
    pub seq: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoom {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_anonymous")]
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct SendMessage {
    pub sender: String,
    pub content: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub sender_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EditMessage {
    pub sender: String,
    pub content: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub since: Option<String>,
    pub limit: Option<i64>,
    pub before: Option<String>,
    pub sender: Option<String>,
    pub sender_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TypingNotification {
    pub sender: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivityEvent {
    pub event_type: String,
    pub room_id: String,
    pub room_name: String,
    pub message_id: String,
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_type: Option<String>,
    pub content: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    pub seq: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityResponse {
    pub events: Vec<ActivityEvent>,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
}

// --- File Attachments ---

#[derive(Debug, Deserialize)]
pub struct FileUpload {
    pub sender: String,
    pub filename: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    pub data: String, // base64-encoded
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub id: String,
    pub room_id: String,
    pub sender: String,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub url: String,
    pub created_at: String,
}

fn default_anonymous() -> String {
    "anonymous".to_string()
}

fn default_content_type() -> String {
    "application/octet-stream".to_string()
}

// --- Room Participants ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Participant {
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_type: Option<String>,
    pub message_count: i64,
    pub first_seen: String,
    pub last_seen: String,
}

// --- Search ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub message_id: String,
    pub room_id: String,
    pub room_name: String,
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_type: Option<String>,
    pub content: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    pub seq: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub count: usize,
    pub query: String,
}
