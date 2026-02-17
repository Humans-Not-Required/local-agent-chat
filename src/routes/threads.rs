use crate::db::Db;
use crate::models::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, State};
use rusqlite::params;

/// Thread response: the root message and all replies in chronological order
#[derive(Debug, serde::Serialize)]
pub struct ThreadResponse {
    pub root: Message,
    pub replies: Vec<ThreadMessage>,
    pub total_replies: usize,
}

/// A message within a thread, with depth info
#[derive(Debug, serde::Serialize)]
pub struct ThreadMessage {
    #[serde(flatten)]
    pub message: Message,
    /// Depth in the thread tree (1 = direct reply to root, 2 = reply to a reply, etc.)
    pub depth: u32,
}

/// Get the full thread for a message.
/// Walks up reply_to chain to find root, then collects all descendants.
#[get("/api/v1/rooms/<room_id>/messages/<message_id>/thread")]
pub fn get_thread(
    db: &State<Db>,
    room_id: &str,
    message_id: &str,
) -> Result<Json<ThreadResponse>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn();

    // Verify room exists
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![room_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }

    // Fetch the target message
    let target = fetch_message(&conn, message_id, room_id)?;

    // Walk up reply_to chain to find the root message
    let mut root = target;
    let mut visited = std::collections::HashSet::new();
    visited.insert(root.id.clone());
    while let Some(ref parent_id) = root.reply_to {
        if visited.contains(parent_id) {
            break; // prevent infinite loops
        }
        visited.insert(parent_id.clone());
        match fetch_message(&conn, parent_id, room_id) {
            Ok(parent) => root = parent,
            Err(_) => break, // parent deleted or not found, treat current as root
        }
    }

    // Collect all descendants of the root using BFS
    let all_messages = fetch_all_room_messages(&conn, room_id);
    let mut replies: Vec<ThreadMessage> = Vec::new();
    let mut queue: Vec<(String, u32)> = vec![(root.id.clone(), 0)]; // (parent_id, parent_depth)
    let mut seen = std::collections::HashSet::new();
    seen.insert(root.id.clone());

    while let Some((parent_id, parent_depth)) = queue.pop() {
        let children: Vec<&Message> = all_messages
            .iter()
            .filter(|m| m.reply_to.as_deref() == Some(&parent_id) && !seen.contains(&m.id))
            .collect();

        for child in children {
            let depth = parent_depth + 1;
            seen.insert(child.id.clone());
            replies.push(ThreadMessage {
                message: child.clone(),
                depth,
            });
            queue.push((child.id.clone(), depth));
        }
    }

    // Sort replies by seq (chronological order)
    replies.sort_by_key(|r| r.message.seq);

    let total_replies = replies.len();

    Ok(Json(ThreadResponse {
        root,
        replies,
        total_replies,
    }))
}

/// Fetch a single message by ID from a specific room
fn fetch_message(
    conn: &std::sync::MutexGuard<rusqlite::Connection>,
    message_id: &str,
    room_id: &str,
) -> Result<Message, (Status, Json<serde_json::Value>)> {
    conn.query_row(
        "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq, pinned_at, pinned_by FROM messages WHERE id = ?1 AND room_id = ?2",
        params![message_id, room_id],
        |row| {
            Ok(Message {
                id: row.get(0)?,
                room_id: row.get(1)?,
                sender: row.get(2)?,
                content: row.get(3)?,
                metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(serde_json::json!({})),
                created_at: row.get(5)?,
                edited_at: row.get(6)?,
                reply_to: row.get(7)?,
                sender_type: row.get(8)?,
                seq: row.get(9)?,
                pinned_at: row.get(10)?,
                pinned_by: row.get(11)?,
                edit_count: 0,
            })
        },
    )
    .map_err(|_| {
        (
            Status::NotFound,
            Json(serde_json::json!({"error": "Message not found in this room"})),
        )
    })
}

/// Fetch all messages in a room (for thread tree traversal)
fn fetch_all_room_messages(
    conn: &std::sync::MutexGuard<rusqlite::Connection>,
    room_id: &str,
) -> Vec<Message> {
    let mut stmt = match conn
        .prepare("SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq, pinned_at, pinned_by FROM messages WHERE room_id = ?1 ORDER BY seq ASC")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    match stmt.query_map(params![room_id], |row| {
        Ok(Message {
            id: row.get(0)?,
            room_id: row.get(1)?,
            sender: row.get(2)?,
            content: row.get(3)?,
            metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(serde_json::json!({})),
            created_at: row.get(5)?,
            edited_at: row.get(6)?,
            reply_to: row.get(7)?,
            sender_type: row.get(8)?,
            seq: row.get(9)?,
            pinned_at: row.get(10)?,
            pinned_by: row.get(11)?,
            edit_count: 0,
        })
    }) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}
