use crate::db::Db;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, State};
use rusqlite::params;

use super::PresenceTracker;

#[get("/api/v1/rooms/<room_id>/presence")]
pub fn room_presence(
    db: &State<Db>,
    presence: &State<PresenceTracker>,
    room_id: &str,
) -> Result<Json<crate::models::RoomPresenceResponse>, (Status, Json<serde_json::Value>)> {
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

    let online = presence.get_room(room_id);
    let count = online.len();

    Ok(Json(crate::models::RoomPresenceResponse {
        room_id: room_id.to_string(),
        online,
        count,
    }))
}

#[get("/api/v1/presence")]
pub fn global_presence(
    presence: &State<PresenceTracker>,
) -> Json<crate::models::GlobalPresenceResponse> {
    let rooms = presence.get_all();
    let total_online: usize = {
        // Count unique senders across all rooms
        let mut unique = std::collections::HashSet::new();
        for entries in rooms.values() {
            for e in entries {
                unique.insert(e.sender.clone());
            }
        }
        unique.len()
    };

    Json(crate::models::GlobalPresenceResponse {
        rooms,
        total_online,
    })
}
