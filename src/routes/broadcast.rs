use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use crate::rate_limit::RateLimiter;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{post, State};
use rusqlite::params;

use super::ClientIp;

/// Broadcast the same message to multiple rooms in a single API call.
///
/// POST /api/v1/broadcast
///
/// Delivers the message to each specified room as a first-class message:
/// - FTS-indexed (searchable)
/// - SSE-delivered to connected streams
/// - Appears in activity feed and message history
///
/// Rate limit: 10 broadcasts/minute per IP.
/// Max 20 rooms per broadcast.
#[post("/api/v1/broadcast", format = "json", data = "<body>")]
pub fn broadcast_message(
    db: &State<Db>,
    events: &State<EventBus>,
    rate_limiter: &State<RateLimiter>,
    ip: ClientIp,
    body: Json<BroadcastMessage>,
) -> Result<Json<BroadcastResponse>, (Status, Json<serde_json::Value>)> {
    // Rate limit: 10 broadcasts/min per IP
    let rl = rate_limiter.check_with_info(&format!("broadcast:{}", ip.0), 10, 60);
    if !rl.allowed {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({
                "error": "Rate limited: max 10 broadcasts per minute",
                "retry_after_secs": rl.retry_after_secs,
                "limit": rl.limit,
                "remaining": 0
            })),
        ));
    }

    // Validate sender
    let sender = body.sender.trim().to_string();
    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender must be 1-100 characters"})),
        ));
    }

    // Validate content
    let content = body.content.trim().to_string();
    if content.is_empty() || content.len() > 10_000 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Content must be 1-10000 characters"})),
        ));
    }

    // Validate room list
    if body.room_ids.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "room_ids must not be empty"})),
        ));
    }
    if body.room_ids.len() > 20 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Broadcast is limited to 20 rooms per call"})),
        ));
    }

    let metadata = body.metadata.clone().unwrap_or(serde_json::json!({}));
    let sender_type = body.sender_type.clone();
    let now = chrono::Utc::now().to_rfc3339();

    let conn = db.conn();
    let mut results: Vec<BroadcastDelivery> = Vec::with_capacity(body.room_ids.len());

    for room_id in &body.room_ids {
        let room_id = room_id.trim();

        // Skip empty room IDs
        if room_id.is_empty() {
            results.push(BroadcastDelivery {
                room_id: String::new(),
                success: false,
                message_id: None,
                error: Some("room_id must not be empty".to_string()),
            });
            continue;
        }

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
            results.push(BroadcastDelivery {
                room_id: room_id.to_string(),
                success: false,
                message_id: None,
                error: Some("Room not found".to_string()),
            });
            continue;
        }

        // Insert message
        let msg_id = uuid::Uuid::new_v4().to_string();
        let seq: i64 = conn
            .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM messages", [], |r| {
                r.get(0)
            })
            .unwrap_or(1);

        let insert_result = conn.execute(
            "INSERT INTO messages (id, room_id, sender, content, metadata, created_at, reply_to, sender_type, seq) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)",
            params![
                &msg_id,
                room_id,
                &sender,
                &content,
                serde_json::to_string(&metadata).unwrap_or_default(),
                &now,
                &sender_type,
                seq
            ],
        );

        match insert_result {
            Ok(_) => {
                // Update room updated_at
                conn.execute(
                    "UPDATE rooms SET updated_at = ?1 WHERE id = ?2",
                    params![&now, room_id],
                )
                .ok();

                // Update FTS index
                crate::db::upsert_fts(&conn, &msg_id);

                // Fire SSE event
                let msg = Message {
                    id: msg_id.clone(),
                    room_id: room_id.to_string(),
                    sender: sender.clone(),
                    content: content.clone(),
                    metadata: metadata.clone(),
                    created_at: now.clone(),
                    edited_at: None,
                    reply_to: None,
                    sender_type: sender_type.clone(),
                    seq,
                    pinned_at: None,
                    pinned_by: None,
                    edit_count: 0,
                };
                events.publish(ChatEvent::NewMessage(msg));

                results.push(BroadcastDelivery {
                    room_id: room_id.to_string(),
                    success: true,
                    message_id: Some(msg_id),
                    error: None,
                });
            }
            Err(_) => {
                results.push(BroadcastDelivery {
                    room_id: room_id.to_string(),
                    success: false,
                    message_id: None,
                    error: Some("Internal server error".to_string()),
                });
            }
        }
    }

    let sent = results.iter().filter(|r| r.success).count();
    let failed = results.len() - sent;

    Ok(Json(BroadcastResponse {
        sent,
        failed,
        results,
    }))
}
