use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, State};
use rusqlite::params;

#[post(
    "/api/v1/rooms/<room_id>/messages/<message_id>/reactions",
    format = "json",
    data = "<body>"
)]
pub fn add_reaction(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    message_id: &str,
    body: Json<AddReaction>,
) -> Result<Json<Reaction>, (Status, Json<serde_json::Value>)> {
    let sender = body.sender.trim();
    let emoji = body.emoji.trim();
    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender must be 1-100 characters"})),
        ));
    }
    if emoji.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Emoji must not be empty"})),
        ));
    }
    // Limit emoji length (single emoji or short code, max 32 chars)
    if emoji.len() > 32 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Emoji too long (max 32 characters)"})),
        ));
    }

    let conn = db.conn.lock().unwrap();

    // Verify message exists and belongs to this room
    let msg_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE id = ?1 AND room_id = ?2",
            params![message_id, room_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !msg_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Message not found in this room"})),
        ));
    }

    // Check if reaction already exists (toggle behavior)
    let existing_id: Option<String> = conn
        .query_row(
            "SELECT id FROM message_reactions WHERE message_id = ?1 AND sender = ?2 AND emoji = ?3",
            params![message_id, sender, emoji],
            |r| r.get(0),
        )
        .ok();

    if let Some(existing) = existing_id {
        // Remove existing reaction (toggle off)
        conn.execute(
            "DELETE FROM message_reactions WHERE id = ?1",
            params![&existing],
        )
        .ok();
        let reaction = Reaction {
            id: existing,
            message_id: message_id.to_string(),
            room_id: room_id.to_string(),
            sender: sender.to_string(),
            emoji: emoji.to_string(),
            created_at: String::new(),
        };
        events.publish(ChatEvent::ReactionRemoved(reaction.clone()));
        // Return the removed reaction with a note
        return Ok(Json(reaction));
    }

    // Add new reaction
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO message_reactions (id, message_id, sender, emoji, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![&id, message_id, sender, emoji, &now],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": format!("Failed to add reaction: {e}")})),
        )
    })?;

    let reaction = Reaction {
        id,
        message_id: message_id.to_string(),
        room_id: room_id.to_string(),
        sender: sender.to_string(),
        emoji: emoji.to_string(),
        created_at: now,
    };

    events.publish(ChatEvent::ReactionAdded(reaction.clone()));

    Ok(Json(reaction))
}

#[delete("/api/v1/rooms/<room_id>/messages/<message_id>/reactions?<sender>&<emoji>")]
pub fn remove_reaction(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    message_id: &str,
    sender: &str,
    emoji: &str,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let sender = sender.trim();
    let emoji = emoji.trim();
    if sender.is_empty() || emoji.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender and emoji are required"})),
        ));
    }

    let conn = db.conn.lock().unwrap();

    // Verify message exists in this room
    let msg_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE id = ?1 AND room_id = ?2",
            params![message_id, room_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !msg_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Message not found in this room"})),
        ));
    }

    // Find and delete the reaction
    let reaction_id: Option<String> = conn
        .query_row(
            "SELECT id FROM message_reactions WHERE message_id = ?1 AND sender = ?2 AND emoji = ?3",
            params![message_id, sender, emoji],
            |r| r.get(0),
        )
        .ok();

    match reaction_id {
        Some(rid) => {
            conn.execute(
                "DELETE FROM message_reactions WHERE id = ?1",
                params![&rid],
            )
            .ok();

            let reaction = Reaction {
                id: rid,
                message_id: message_id.to_string(),
                room_id: room_id.to_string(),
                sender: sender.to_string(),
                emoji: emoji.to_string(),
                created_at: String::new(),
            };
            events.publish(ChatEvent::ReactionRemoved(reaction));

            Ok(Json(serde_json::json!({"status": "removed"})))
        }
        None => Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Reaction not found"})),
        )),
    }
}

#[get("/api/v1/rooms/<room_id>/messages/<message_id>/reactions")]
pub fn get_reactions(
    db: &State<Db>,
    room_id: &str,
    message_id: &str,
) -> Result<Json<ReactionsResponse>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify message exists in this room
    let msg_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE id = ?1 AND room_id = ?2",
            params![message_id, room_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !msg_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Message not found in this room"})),
        ));
    }

    // Get grouped reactions
    let mut stmt = conn
        .prepare(
            "SELECT emoji, GROUP_CONCAT(sender, ','), COUNT(*) \
             FROM message_reactions WHERE message_id = ?1 \
             GROUP BY emoji ORDER BY MIN(created_at) ASC",
        )
        .unwrap();

    let reactions: Vec<ReactionSummary> = stmt
        .query_map(params![message_id], |row| {
            let emoji: String = row.get(0)?;
            let senders_str: String = row.get(1)?;
            let count: i64 = row.get(2)?;
            let senders: Vec<String> = senders_str.split(',').map(|s| s.to_string()).collect();
            Ok(ReactionSummary {
                emoji,
                count,
                senders,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(ReactionsResponse {
        message_id: message_id.to_string(),
        reactions,
    }))
}

/// Get all reactions for all messages in a room (bulk fetch)
#[get("/api/v1/rooms/<room_id>/reactions")]
pub fn get_room_reactions(
    db: &State<Db>,
    room_id: &str,
) -> Result<Json<RoomReactionsResponse>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists
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

    // Get all reactions for messages in this room, grouped by message_id and emoji
    let mut stmt = conn
        .prepare(
            "SELECT mr.message_id, mr.emoji, GROUP_CONCAT(mr.sender, ','), COUNT(*) \
             FROM message_reactions mr \
             JOIN messages m ON mr.message_id = m.id AND m.room_id = ?1 \
             GROUP BY mr.message_id, mr.emoji \
             ORDER BY mr.message_id, MIN(mr.created_at) ASC",
        )
        .unwrap();

    let rows: Vec<(String, String, String, i64)> = stmt
        .query_map(params![room_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let mut reactions_map: std::collections::HashMap<String, Vec<ReactionSummary>> =
        std::collections::HashMap::new();

    for (message_id, emoji, senders_str, count) in rows {
        let senders: Vec<String> = senders_str.split(',').map(|s| s.to_string()).collect();
        reactions_map
            .entry(message_id)
            .or_default()
            .push(ReactionSummary {
                emoji,
                count,
                senders,
            });
    }

    Ok(Json(RoomReactionsResponse {
        room_id: room_id.to_string(),
        reactions: reactions_map,
    }))
}
