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
}

#[derive(Debug, Deserialize)]
pub struct TypingNotification {
    pub sender: String,
}

fn default_anonymous() -> String {
    "anonymous".to_string()
}
