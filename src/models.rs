use serde::{Deserialize, Serialize};

fn is_zero(v: &i64) -> bool {
    *v == 0
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmarked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_messages: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_message_age_hours: Option<i64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_by: Option<String>,
    #[serde(skip_serializing_if = "is_zero")]
    pub edit_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoom {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_anonymous")]
    pub created_by: String,
    #[serde(default)]
    pub max_messages: Option<i64>,
    #[serde(default)]
    pub max_message_age_hours: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoom {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Set to a number to enable max message count retention. Set to null to disable.
    #[serde(default, deserialize_with = "deserialize_optional_nullable_i64")]
    pub max_messages: Option<Option<i64>>,
    /// Set to a number to enable age-based retention. Set to null to disable.
    #[serde(default, deserialize_with = "deserialize_optional_nullable_i64")]
    pub max_message_age_hours: Option<Option<i64>>,
}

/// Deserializer for double-option fields: absent = None (skip), null = Some(None) (clear), value = Some(Some(v)).
/// This is only called when the field is PRESENT in JSON (absent uses #[serde(default)] → None).
fn deserialize_optional_nullable_i64<'de, D>(deserializer: D) -> Result<Option<Option<i64>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Field is present: null → Some(None), value → Some(Some(v))
    let v: Option<i64> = Option::deserialize(deserializer)?;
    Ok(Some(v))
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

#[derive(Debug, Serialize, Clone)]
pub struct MessageEdit {
    pub id: String,
    pub message_id: String,
    pub previous_content: String,
    pub edited_at: String,
    pub editor: String,
}

#[derive(Debug, Serialize)]
pub struct EditHistoryResponse {
    pub message_id: String,
    pub current_content: String,
    pub edits: Vec<MessageEdit>,
    pub edit_count: i64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_date: Option<String>,
    pub has_more: bool,
}

// --- Pins ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PinnedMessage {
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
    pub pinned_at: String,
    pub pinned_by: String,
}

// --- Presence ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PresenceEntry {
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_type: Option<String>,
    pub connected_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomPresenceResponse {
    pub room_id: String,
    pub online: Vec<PresenceEntry>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalPresenceResponse {
    pub rooms: std::collections::HashMap<String, Vec<PresenceEntry>>,
    pub total_online: usize,
}

// --- Webhooks ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Webhook {
    pub id: String,
    pub room_id: String,
    pub url: String,
    pub events: String,
    pub created_by: String,
    pub created_at: String,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhook {
    pub url: String,
    #[serde(default = "default_webhook_events")]
    pub events: String,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default = "default_anonymous")]
    pub created_by: String,
}

fn default_webhook_events() -> String {
    "*".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebhook {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub events: Option<String>,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookPayload {
    pub event: String,
    pub room_id: String,
    pub room_name: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookDeliveryLog {
    pub id: String,
    pub delivery_group: String,
    pub webhook_id: String,
    pub event: String,
    pub url: String,
    pub attempt: i32,
    pub status: String,
    pub status_code: Option<i64>,
    pub error_message: Option<String>,
    pub response_time_ms: i64,
    pub created_at: String,
}

// --- Read Positions ---

#[derive(Debug, Deserialize)]
pub struct UpdateReadPosition {
    pub sender: String,
    pub last_read_seq: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadPosition {
    pub room_id: String,
    pub sender: String,
    pub last_read_seq: i64,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnreadInfo {
    pub room_id: String,
    pub room_name: String,
    pub unread_count: i64,
    pub last_read_seq: i64,
    pub latest_seq: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnreadResponse {
    pub sender: String,
    pub rooms: Vec<UnreadInfo>,
    pub total_unread: i64,
}

// --- Reactions ---

#[derive(Debug, Deserialize)]
pub struct AddReaction {
    pub sender: String,
    pub emoji: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reaction {
    pub id: String,
    pub message_id: String,
    pub room_id: String,
    pub sender: String,
    pub emoji: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReactionSummary {
    pub emoji: String,
    pub count: i64,
    pub senders: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReactionsResponse {
    pub message_id: String,
    pub reactions: Vec<ReactionSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomReactionsResponse {
    pub room_id: String,
    pub reactions: std::collections::HashMap<String, Vec<ReactionSummary>>,
}

// --- Direct Messages ---

#[derive(Debug, Deserialize)]
pub struct SendDm {
    pub sender: String,
    pub recipient: String,
    pub content: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub sender_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DmConversation {
    pub room_id: String,
    pub other_participant: String,
    pub last_message_content: Option<String>,
    pub last_message_sender: Option<String>,
    pub last_message_at: Option<String>,
    pub message_count: i64,
    pub unread_count: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DmConversationsResponse {
    pub sender: String,
    pub conversations: Vec<DmConversation>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DmSendResponse {
    pub message: Message,
    pub room_id: String,
    pub created: bool, // true if a new DM room was created
}

// --- Mentions ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MentionResult {
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
pub struct MentionsResponse {
    pub target: String,
    pub mentions: Vec<MentionResult>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnreadMentionRoom {
    pub room_id: String,
    pub room_name: String,
    pub mention_count: i64,
    pub oldest_seq: i64,
    pub newest_seq: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnreadMentionsResponse {
    pub target: String,
    pub rooms: Vec<UnreadMentionRoom>,
    pub total_unread: i64,
}

// --- Bookmarks ---

#[derive(Debug, Deserialize)]
pub struct BookmarkAction {
    pub sender: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookmarkedRoom {
    pub room_id: String,
    pub room_name: String,
    pub description: String,
    pub created_at: String,
    pub bookmarked_at: String,
    pub message_count: i64,
    pub last_activity: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookmarksResponse {
    pub sender: String,
    pub bookmarks: Vec<BookmarkedRoom>,
    pub count: usize,
}

// --- Incoming Webhooks ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingWebhook {
    pub id: String,
    pub room_id: String,
    pub name: String,
    pub token: String,
    pub created_by: String,
    pub created_at: String,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>, // Computed: /api/v1/hook/{token}
}

#[derive(Debug, Deserialize)]
pub struct CreateIncomingWebhook {
    pub name: String,
    #[serde(default = "default_anonymous")]
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIncomingWebhook {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct IncomingWebhookMessage {
    pub content: String,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub sender_type: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

// --- Profiles ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpsertProfile {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub sender_type: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnrichedParticipant {
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_type: Option<String>,
    pub message_count: i64,
    pub first_seen: String,
    pub last_seen: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
}
