// Route module decomposition — each domain area in its own file.
// Shared types (request guards, trackers) live here; route functions in submodules.

mod dm;
mod files;
mod mentions;
mod messages;
mod participants;
mod pins;
mod presence;
mod profiles;
mod reactions;
mod read_positions;
mod rooms;
mod search;
mod stream;
mod system;
mod typing;
mod threads;
mod webhook_routes;

// --- Re-exports (all route functions used by lib.rs mount) ---

pub use dm::{send_dm, list_dm_conversations, get_dm_conversation};
pub use mentions::{get_mentions, get_unread_mentions};
pub use files::{delete_file, download_file, file_info, list_files, upload_file};
pub use messages::{delete_message, edit_message, get_messages, send_message};
pub use participants::room_participants;
pub use pins::{list_pins, pin_message, unpin_message};
pub use presence::{global_presence, room_presence};
pub use profiles::{delete_profile, get_profile, list_profiles, upsert_profile};
pub use read_positions::{get_read_positions, get_unread, update_read_position};
pub use reactions::{add_reaction, get_reactions, get_room_reactions, remove_reaction};
pub use rooms::{archive_room, create_room, delete_room, get_room, list_rooms, unarchive_room, update_room};
pub use search::{activity_feed, search_messages};
pub use stream::message_stream;
pub use threads::get_thread;
pub use system::{
    health, llms_txt_api, llms_txt_root, not_found, openapi_json, spa_fallback, stats,
    too_many_requests,
};
pub use typing::notify_typing;
pub use webhook_routes::{create_webhook, delete_webhook, list_webhooks, update_webhook};

// --- Shared request guards ---

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex, RwLock};

pub struct ClientIp(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientIp {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = req
            .headers()
            .get_one("X-Forwarded-For")
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .or_else(|| req.remote().map(|r| r.ip().to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        Outcome::Success(ClientIp(ip))
    }
}

pub struct AdminKey(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminKey {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(auth) = req.headers().get_one("Authorization")
            && let Some(key) = auth.strip_prefix("Bearer ")
        {
            return Outcome::Success(AdminKey(key.to_string()));
        }
        if let Some(key) = req.headers().get_one("X-Admin-Key") {
            return Outcome::Success(AdminKey(key.to_string()));
        }
        Outcome::Forward(Status::Unauthorized)
    }
}

// --- Typing Tracker ---

/// In-memory dedup: tracks last typing notification per (room, sender) to avoid spam.
/// Key: "room_id:sender", Value: timestamp (seconds since epoch).
pub struct TypingTracker {
    pub last_typing: StdMutex<HashMap<String, u64>>,
}

impl Default for TypingTracker {
    fn default() -> Self {
        Self {
            last_typing: StdMutex::new(HashMap::new()),
        }
    }
}

// --- Presence Tracker ---

/// Internal entry with connection count (not serialized directly)
pub(crate) struct PresenceInner {
    sender: String,
    sender_type: Option<String>,
    connected_at: String,
    connections: usize,
}

#[derive(Clone)]
pub struct PresenceTracker {
    pub(crate) inner: Arc<RwLock<HashMap<String, HashMap<String, PresenceInner>>>>,
}

impl Default for PresenceTracker {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl PresenceTracker {
    /// Register a sender as present in a room. Returns true if this is their first connection (new presence).
    pub fn join(&self, room_id: &str, sender: &str, sender_type: Option<&str>) -> bool {
        let mut map = self.inner.write().unwrap();
        let room = map.entry(room_id.to_string()).or_default();
        let is_new = !room.contains_key(sender);
        let entry = room
            .entry(sender.to_string())
            .or_insert_with(|| PresenceInner {
                sender: sender.to_string(),
                sender_type: sender_type.map(String::from),
                connected_at: chrono::Utc::now().to_rfc3339(),
                connections: 0,
            });
        entry.connections += 1;
        // Update sender_type if provided and was previously None
        if sender_type.is_some() && entry.sender_type.is_none() {
            entry.sender_type = sender_type.map(String::from);
        }
        is_new
    }

    /// Remove a sender's connection from a room. Returns true if fully disconnected (last connection).
    pub fn leave(&self, room_id: &str, sender: &str) -> bool {
        let mut map = self.inner.write().unwrap();
        if let Some(room) = map.get_mut(room_id)
            && let Some(entry) = room.get_mut(sender)
        {
            entry.connections = entry.connections.saturating_sub(1);
            if entry.connections == 0 {
                room.remove(sender);
                if room.is_empty() {
                    map.remove(room_id);
                }
                return true;
            }
        }
        false
    }

    /// Get all online users in a room.
    pub fn get_room(&self, room_id: &str) -> Vec<crate::models::PresenceEntry> {
        let map = self.inner.read().unwrap();
        map.get(room_id)
            .map(|room| {
                room.values()
                    .map(|e| crate::models::PresenceEntry {
                        sender: e.sender.clone(),
                        sender_type: e.sender_type.clone(),
                        connected_at: e.connected_at.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all online users across all rooms.
    pub fn get_all(&self) -> HashMap<String, Vec<crate::models::PresenceEntry>> {
        let map = self.inner.read().unwrap();
        map.iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    v.values()
                        .map(|e| crate::models::PresenceEntry {
                            sender: e.sender.clone(),
                            sender_type: e.sender_type.clone(),
                            connected_at: e.connected_at.clone(),
                        })
                        .collect(),
                )
            })
            .collect()
    }
}

/// RAII guard that removes presence when the SSE stream is dropped (client disconnects).
pub(crate) struct PresenceGuard {
    pub(crate) tracker: PresenceTracker,
    pub(crate) room_id: String,
    pub(crate) sender: String,
    pub(crate) events_sender: tokio::sync::broadcast::Sender<crate::events::ChatEvent>,
}

impl Drop for PresenceGuard {
    fn drop(&mut self) {
        let fully_left = self.tracker.leave(&self.room_id, &self.sender);
        if fully_left {
            let _ = self
                .events_sender
                .send(crate::events::ChatEvent::PresenceLeft {
                    sender: self.sender.clone(),
                    room_id: self.room_id.clone(),
                });
        }
    }
}
