use crate::events::ChatEvent;
use crate::models::WebhookPayload;
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection};
use sha2::Sha256;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

type HmacSha256 = Hmac<Sha256>;

/// Maximum retry attempts for webhook delivery.
const MAX_ATTEMPTS: u32 = 3;

/// Backoff durations between retries (attempt 2 waits 2s, attempt 3 waits 4s).
const RETRY_BACKOFFS_MS: [u64; 2] = [2000, 4000];

/// Spawns a background task that subscribes to the EventBus and delivers webhooks.
pub fn spawn_dispatcher(mut receiver: broadcast::Receiver<ChatEvent>, db_path: String) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("⚠️ Webhook dispatcher: failed to create HTTP client: {e}");
                return;
            }
        };

        let conn = Arc::new(Mutex::new(match Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("⚠️ Webhook dispatcher: failed to open DB: {e}");
                return;
            }
        }));
        {
            let db = conn.lock().unwrap_or_else(|e| e.into_inner());
            db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
                .ok();
        }

        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Some((event_name, room_id, data)) = event_to_payload(&event) {
                        deliver_webhooks(&conn, &client, &event_name, &room_id, data).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("⚠️ Webhook dispatcher lagged, missed {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    eprintln!("Webhook dispatcher: channel closed, exiting");
                    break;
                }
            }
        }
    });
}

/// Convert a ChatEvent to (event_name, room_id, data) for webhook delivery.
fn event_to_payload(event: &ChatEvent) -> Option<(String, String, serde_json::Value)> {
    match event {
        ChatEvent::NewMessage(msg) => Some((
            "message".to_string(),
            msg.room_id.clone(),
            serde_json::to_value(msg).unwrap_or_default(),
        )),
        ChatEvent::MessageEdited(msg) => Some((
            "message_edited".to_string(),
            msg.room_id.clone(),
            serde_json::to_value(msg).unwrap_or_default(),
        )),
        ChatEvent::MessageDeleted { id, room_id } => Some((
            "message_deleted".to_string(),
            room_id.clone(),
            serde_json::json!({"id": id, "room_id": room_id}),
        )),
        ChatEvent::FileUploaded(file) => Some((
            "file_uploaded".to_string(),
            file.room_id.clone(),
            serde_json::to_value(file).unwrap_or_default(),
        )),
        ChatEvent::FileDeleted { id, room_id } => Some((
            "file_deleted".to_string(),
            room_id.clone(),
            serde_json::json!({"id": id, "room_id": room_id}),
        )),
        ChatEvent::ReactionAdded(reaction) => Some((
            "reaction_added".to_string(),
            reaction.room_id.clone(),
            serde_json::to_value(reaction).unwrap_or_default(),
        )),
        ChatEvent::ReactionRemoved(reaction) => Some((
            "reaction_removed".to_string(),
            reaction.room_id.clone(),
            serde_json::to_value(reaction).unwrap_or_default(),
        )),
        ChatEvent::MessagePinned(pinned) => Some((
            "message_pinned".to_string(),
            pinned.room_id.clone(),
            serde_json::to_value(pinned).unwrap_or_default(),
        )),
        ChatEvent::MessageUnpinned { id, room_id } => Some((
            "message_unpinned".to_string(),
            room_id.clone(),
            serde_json::json!({"id": id, "room_id": room_id}),
        )),
        ChatEvent::PresenceJoined {
            sender,
            sender_type,
            room_id,
        } => Some((
            "presence_joined".to_string(),
            room_id.clone(),
            serde_json::json!({"sender": sender, "sender_type": sender_type, "room_id": room_id}),
        )),
        ChatEvent::PresenceLeft { sender, room_id } => Some((
            "presence_left".to_string(),
            room_id.clone(),
            serde_json::json!({"sender": sender, "room_id": room_id}),
        )),
        ChatEvent::Typing { .. } => None,
        ChatEvent::ReadPositionUpdated(_) => None,
        ChatEvent::ProfileUpdated(_) => None,
        ChatEvent::ProfileDeleted { .. } => None,
        ChatEvent::RoomUpdated(room) => Some((
            "room_updated".to_string(),
            room.id.clone(),
            serde_json::to_value(room).unwrap_or_default(),
        )),
        ChatEvent::RoomArchived(room) => Some((
            "room_archived".to_string(),
            room.id.clone(),
            serde_json::to_value(room).unwrap_or_default(),
        )),
        ChatEvent::RoomUnarchived(room) => Some((
            "room_unarchived".to_string(),
            room.id.clone(),
            serde_json::to_value(room).unwrap_or_default(),
        )),
        ChatEvent::RoomBookmarked { room_id, sender } => Some((
            "room_bookmarked".to_string(),
            room_id.clone(),
            serde_json::json!({"room_id": room_id, "sender": sender}),
        )),
        ChatEvent::RoomUnbookmarked { room_id, sender } => Some((
            "room_unbookmarked".to_string(),
            room_id.clone(),
            serde_json::json!({"room_id": room_id, "sender": sender}),
        )),
    }
}

/// Look up matching webhooks and deliver with retry + audit logging.
async fn deliver_webhooks(
    conn: &Arc<Mutex<Connection>>,
    client: &reqwest::Client,
    event_name: &str,
    room_id: &str,
    data: serde_json::Value,
) {
    // Query matching webhooks (id, url, secret, events filter)
    let webhooks: Vec<(String, String, Option<String>, String)> = {
        let db = conn.lock().unwrap_or_else(|e| {
            eprintln!("WARN: Webhook dispatcher DB mutex poisoned, recovering");
            e.into_inner()
        });
        let mut stmt = match db.prepare(
            "SELECT id, url, secret, events FROM webhooks WHERE room_id = ?1 AND active = 1",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("⚠️ Webhook dispatcher: failed to prepare query: {e}");
                return;
            }
        };
        match stmt.query_map(params![room_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        }) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                eprintln!("⚠️ Webhook dispatcher: query failed: {e}");
                return;
            }
        }
    };

    // Get room name for the payload
    let room_name: String = {
        let db = conn.lock().unwrap_or_else(|e| e.into_inner());
        db.query_row(
            "SELECT name FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "unknown".to_string())
    };

    for (webhook_id, url, secret, events_str) in webhooks {
        // Check event filter
        if events_str != "*" {
            let allowed: Vec<&str> = events_str.split(',').map(|s| s.trim()).collect();
            if !allowed.contains(&event_name) {
                continue;
            }
        }

        let payload = WebhookPayload {
            event: event_name.to_string(),
            room_id: room_id.to_string(),
            room_name: room_name.clone(),
            data: data.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let body = serde_json::to_string(&payload).unwrap_or_default();
        let delivery_group = uuid::Uuid::new_v4().to_string();

        // Retry loop with exponential backoff
        for attempt in 1..=MAX_ATTEMPTS {
            if attempt > 1 {
                let backoff = RETRY_BACKOFFS_MS[(attempt - 2) as usize];
                tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
            }

            let mut request = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Chat-Event", event_name)
                .header("X-Chat-Webhook-Id", &webhook_id);

            // HMAC-SHA256 signature if secret is set
            if let Some(ref secret) = secret
                && let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes())
            {
                mac.update(body.as_bytes());
                let signature = hex::encode(mac.finalize().into_bytes());
                request = request.header("X-Chat-Signature", format!("sha256={}", signature));
            }

            let start = std::time::Instant::now();
            let result = request.body(body.clone()).send().await;
            let elapsed_ms = start.elapsed().as_millis() as i64;

            match result {
                Ok(resp) => {
                    let status_code = resp.status().as_u16() as i64;
                    if resp.status().is_success() {
                        log_delivery(
                            conn,
                            &delivery_group,
                            &webhook_id,
                            event_name,
                            &url,
                            attempt,
                            "success",
                            Some(status_code),
                            None,
                            elapsed_ms,
                        );
                        if attempt > 1 {
                            eprintln!(
                                "✅ Webhook {} delivered to {} after {} attempts",
                                webhook_id, url, attempt
                            );
                        }
                        break;
                    } else {
                        let error_msg = format!("HTTP {}", status_code);
                        log_delivery(
                            conn,
                            &delivery_group,
                            &webhook_id,
                            event_name,
                            &url,
                            attempt,
                            "failed",
                            Some(status_code),
                            Some(&error_msg),
                            elapsed_ms,
                        );
                        if attempt == MAX_ATTEMPTS {
                            eprintln!(
                                "⚠️ Webhook {} delivery to {} exhausted after {} attempts (last: {})",
                                webhook_id, url, MAX_ATTEMPTS, error_msg
                            );
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    log_delivery(
                        conn,
                        &delivery_group,
                        &webhook_id,
                        event_name,
                        &url,
                        attempt,
                        "failed",
                        None,
                        Some(&error_msg),
                        elapsed_ms,
                    );
                    if attempt == MAX_ATTEMPTS {
                        eprintln!(
                            "⚠️ Webhook {} delivery to {} exhausted after {} attempts (last: {})",
                            webhook_id, url, MAX_ATTEMPTS, error_msg
                        );
                    }
                }
            }
        }
    }
}

/// Log a single webhook delivery attempt to the database.
#[allow(clippy::too_many_arguments)]
fn log_delivery(
    conn: &Arc<Mutex<Connection>>,
    delivery_group: &str,
    webhook_id: &str,
    event: &str,
    url: &str,
    attempt: u32,
    status: &str,
    status_code: Option<i64>,
    error_message: Option<&str>,
    response_time_ms: i64,
) {
    let db = conn.lock().unwrap_or_else(|e| e.into_inner());
    let id = uuid::Uuid::new_v4().to_string();
    let _ = db.execute(
        "INSERT INTO webhook_deliveries (id, delivery_group, webhook_id, event, url, attempt, status, status_code, error_message, response_time_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![id, delivery_group, webhook_id, event, url, attempt as i32, status, status_code, error_message, response_time_ms],
    );
}
