use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::Message;
use rocket::response::stream::{Event, EventStream};
use rocket::{get, State};
use rusqlite::params;
use tokio::time::{interval, Duration};

use super::{PresenceGuard, PresenceTracker};

#[get("/api/v1/rooms/<room_id>/stream?<since>&<after>&<sender>&<sender_type>")]
#[allow(clippy::too_many_arguments)]
pub fn message_stream(
    db: &State<Db>,
    events: &State<EventBus>,
    presence: &State<PresenceTracker>,
    room_id: &str,
    since: Option<&str>,
    after: Option<i64>,
    sender: Option<&str>,
    sender_type: Option<&str>,
) -> EventStream![] {
    let mut rx = events.sender.subscribe();
    let room_id = room_id.to_string();

    // Register presence if sender is provided
    let guard = sender.map(|s| {
        let s = s.trim().to_string();
        let st = sender_type.map(|v| v.trim().to_string());
        let is_new = presence.join(&room_id, &s, st.as_deref());
        if is_new {
            events.publish(ChatEvent::PresenceJoined {
                sender: s.clone(),
                sender_type: st.clone(),
                room_id: room_id.clone(),
            });
        }
        PresenceGuard {
            tracker: PresenceTracker {
                inner: presence.inner.clone(),
            },
            room_id: room_id.clone(),
            sender: s,
            events_sender: events.sender.clone(),
        }
    });

    // Replay missed messages if `after` or `since` provided
    let replay: Vec<Message> = if let Some(after_val) = after {
        // Preferred: cursor-based replay using monotonic seq
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq, pinned_at, pinned_by FROM messages WHERE room_id = ?1 AND seq > ?2 ORDER BY seq ASC LIMIT 100",
            )
            .ok();
        if let Some(ref mut s) = stmt {
            s.query_map(params![&room_id, after_val], |row| {
                let metadata_str: String = row.get(4)?;
                Ok(Message {
                    id: row.get(0)?,
                    room_id: row.get(1)?,
                    sender: row.get(2)?,
                    content: row.get(3)?,
                    metadata: serde_json::from_str(&metadata_str)
                        .unwrap_or(serde_json::json!({})),
                    created_at: row.get(5)?,
                    edited_at: row.get(6)?,
                    reply_to: row.get(7)?,
                    sender_type: row.get(8)?,
                    seq: row.get(9)?,
                    pinned_at: row.get(10)?,
                    pinned_by: row.get(11)?,
                })
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        } else {
            vec![]
        }
    } else if let Some(since_val) = since {
        // Backward compat: timestamp-based replay
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq, pinned_at, pinned_by FROM messages WHERE room_id = ?1 AND created_at > ?2 ORDER BY seq ASC LIMIT 100",
            )
            .ok();
        if let Some(ref mut s) = stmt {
            s.query_map(params![&room_id, since_val], |row| {
                let metadata_str: String = row.get(4)?;
                Ok(Message {
                    id: row.get(0)?,
                    room_id: row.get(1)?,
                    sender: row.get(2)?,
                    content: row.get(3)?,
                    metadata: serde_json::from_str(&metadata_str)
                        .unwrap_or(serde_json::json!({})),
                    created_at: row.get(5)?,
                    edited_at: row.get(6)?,
                    reply_to: row.get(7)?,
                    sender_type: row.get(8)?,
                    seq: row.get(9)?,
                    pinned_at: row.get(10)?,
                    pinned_by: row.get(11)?,
                })
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    EventStream! {
        // Keep presence guard alive for the lifetime of the stream.
        // When the stream is dropped (client disconnects), the guard is dropped,
        // which removes the presence entry and publishes a PresenceLeft event.
        let _presence_guard = guard;

        // Send replayed messages first
        for msg in replay {
            yield Event::json(&msg).event("message");
        }

        let mut heartbeat = interval(Duration::from_secs(15));

        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(ChatEvent::NewMessage(m)) if m.room_id == room_id => {
                            yield Event::json(&m).event("message");
                        }
                        Ok(ChatEvent::MessageEdited(m)) if m.room_id == room_id => {
                            yield Event::json(&m).event("message_edited");
                        }
                        Ok(ChatEvent::MessageDeleted { ref id, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"id": id, "room_id": rid})).event("message_deleted");
                        }
                        Ok(ChatEvent::RoomUpdated(ref r)) if r.id == room_id => {
                            yield Event::json(r).event("room_updated");
                        }
                        Ok(ChatEvent::Typing { ref sender, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"sender": sender, "room_id": rid})).event("typing");
                        }
                        Ok(ChatEvent::FileUploaded(ref f)) if f.room_id == room_id => {
                            yield Event::json(f).event("file_uploaded");
                        }
                        Ok(ChatEvent::FileDeleted { ref id, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"id": id, "room_id": rid})).event("file_deleted");
                        }
                        Ok(ChatEvent::ReactionAdded(ref r)) if r.room_id == room_id => {
                            yield Event::json(r).event("reaction_added");
                        }
                        Ok(ChatEvent::ReactionRemoved(ref r)) if r.room_id == room_id => {
                            yield Event::json(r).event("reaction_removed");
                        }
                        Ok(ChatEvent::MessagePinned(ref p)) if p.room_id == room_id => {
                            yield Event::json(p).event("message_pinned");
                        }
                        Ok(ChatEvent::MessageUnpinned { ref id, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"id": id, "room_id": rid})).event("message_unpinned");
                        }
                        Ok(ChatEvent::PresenceJoined { ref sender, ref sender_type, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"sender": sender, "sender_type": sender_type, "room_id": rid})).event("presence_joined");
                        }
                        Ok(ChatEvent::PresenceLeft { ref sender, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"sender": sender, "room_id": rid})).event("presence_left");
                        }
                        Ok(ChatEvent::ReadPositionUpdated(ref rp)) if rp.room_id == room_id => {
                            yield Event::json(rp).event("read_position_updated");
                        }
                        Ok(ChatEvent::ProfileUpdated(ref p)) => {
                            yield Event::json(p).event("profile_updated");
                        }
                        Ok(ChatEvent::ProfileDeleted { ref sender }) => {
                            yield Event::json(&serde_json::json!({"sender": sender})).event("profile_deleted");
                        }
                        Ok(ChatEvent::RoomArchived(ref r)) if r.id == room_id => {
                            yield Event::json(r).event("room_archived");
                        }
                        Ok(ChatEvent::RoomUnarchived(ref r)) if r.id == room_id => {
                            yield Event::json(r).event("room_unarchived");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        _ => {} // different room or lagged
                    }
                }
                _ = heartbeat.tick() => {
                    let now = chrono::Utc::now().to_rfc3339();
                    yield Event::json(&serde_json::json!({"time": now})).event("heartbeat");
                }
            }
        }
    }
}
