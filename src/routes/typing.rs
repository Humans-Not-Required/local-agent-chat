use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::TypingNotification;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{post, State};
use rusqlite::params;

use super::TypingTracker;

#[post("/api/v1/rooms/<room_id>/typing", format = "json", data = "<body>")]
pub fn notify_typing(
    db: &State<Db>,
    events: &State<EventBus>,
    typing_tracker: &State<TypingTracker>,
    room_id: &str,
    body: Json<TypingNotification>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let sender = body.sender.trim().to_string();
    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender must be 1-100 characters"})),
        ));
    }

    // Verify room exists
    let conn = db.conn();
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }
    drop(conn);

    // Dedup: only publish if last typing notification was >2 seconds ago
    let key = format!("{}:{}", room_id, sender);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    {
        let mut tracker = typing_tracker.last_typing.lock().unwrap();
        if let Some(&last) = tracker.get(&key)
            && now - last < 2
        {
            return Ok(Json(serde_json::json!({"ok": true})));
        }
        tracker.insert(key, now);

        // Prune old entries (>30s) to prevent memory leak
        tracker.retain(|_, &mut ts| now - ts < 30);
    }

    events.publish(ChatEvent::Typing {
        sender,
        room_id: room_id.to_string(),
    });

    Ok(Json(serde_json::json!({"ok": true})))
}
