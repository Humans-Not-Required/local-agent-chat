use crate::events::ChatEvent;
use crate::models::WebhookDelivery;
use hmac::{Hmac, Mac};
use rusqlite::{Connection, params};
use sha2::Sha256;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

type HmacSha256 = Hmac<Sha256>;

/// Spawns a background task that subscribes to the EventBus and delivers webhooks.
pub fn spawn_dispatcher(
    mut receiver: broadcast::Receiver<ChatEvent>,
    db_path: String,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        // Open a separate DB connection for the webhook dispatcher
        let conn = Arc::new(Mutex::new(
            Connection::open(&db_path).expect("Webhook dispatcher: failed to open DB"),
        ));
        conn.lock()
            .unwrap()
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .ok();

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
        // Typing and read position events are ephemeral — skip webhook delivery
        ChatEvent::Typing { .. } => None,
        ChatEvent::ReadPositionUpdated(_) => None,
        // Room updates don't have a single room_id target in the same way
        ChatEvent::RoomUpdated(room) => Some((
            "room_updated".to_string(),
            room.id.clone(),
            serde_json::to_value(room).unwrap_or_default(),
        )),
    }
}

/// Look up matching webhooks and deliver the payload.
async fn deliver_webhooks(
    conn: &Arc<Mutex<Connection>>,
    client: &reqwest::Client,
    event_name: &str,
    room_id: &str,
    data: serde_json::Value,
) {
    // Query matching webhooks
    let webhooks: Vec<(String, String, Option<String>)> = {
        let db = conn.lock().unwrap();
        let mut stmt = db
            .prepare(
                "SELECT id, url, secret FROM webhooks WHERE room_id = ?1 AND active = 1",
            )
            .unwrap();
        stmt.query_map(params![room_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    };

    // Also get room name for the payload
    let room_name: String = {
        let db = conn.lock().unwrap();
        db.query_row(
            "SELECT name FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "unknown".to_string())
    };

    for (webhook_id, url, secret) in webhooks {
        // Check event filter
        let events_str: String = {
            let db = conn.lock().unwrap();
            db.query_row(
                "SELECT events FROM webhooks WHERE id = ?1",
                params![webhook_id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "*".to_string())
        };

        if events_str != "*" {
            let allowed: Vec<&str> = events_str.split(',').map(|s| s.trim()).collect();
            if !allowed.contains(&event_name) {
                continue;
            }
        }

        let payload = WebhookDelivery {
            event: event_name.to_string(),
            room_id: room_id.to_string(),
            room_name: room_name.clone(),
            data: data.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let body = serde_json::to_string(&payload).unwrap_or_default();

        let mut request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Chat-Event", event_name)
            .header("X-Chat-Webhook-Id", &webhook_id);

        // HMAC-SHA256 signature if secret is set
        if let Some(ref secret) = secret {
            if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
                mac.update(body.as_bytes());
                let signature = hex::encode(mac.finalize().into_bytes());
                request = request.header("X-Chat-Signature", format!("sha256={}", signature));
            }
        }

        // Fire-and-forget: spawn a task for each delivery
        let request = request.body(body);
        tokio::spawn(async move {
            match request.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        eprintln!(
                            "⚠️ Webhook {} delivery failed: HTTP {}",
                            webhook_id,
                            resp.status()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("⚠️ Webhook {} delivery error: {}", webhook_id, e);
                }
            }
        });
    }
}
