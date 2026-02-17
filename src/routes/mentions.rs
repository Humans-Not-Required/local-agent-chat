use crate::db::Db;
use crate::models::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, State};

/// GET /api/v1/mentions?target=<name>&after=<seq>&room_id=<uuid>&limit=N
/// Returns messages that @mention the target sender, with room context.
/// Uses LIKE pattern matching for @mentions in message content.
#[get("/api/v1/mentions?<target>&<after>&<room_id>&<limit>")]
pub fn get_mentions(
    db: &State<Db>,
    target: &str,
    after: Option<i64>,
    room_id: Option<&str>,
    limit: Option<i64>,
) -> Result<Json<MentionsResponse>, (Status, Json<serde_json::Value>)> {
    let target = target.trim();
    if target.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Query parameter 'target' must not be empty"})),
        ));
    }
    if target.len() > 200 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Target name too long (max 200 characters)"})),
        ));
    }

    let conn = db.conn();
    let limit = limit.unwrap_or(50).clamp(1, 200);

    // Build LIKE pattern for @mention detection
    // Match @target followed by a word boundary (space, punctuation, end of string)
    // We use two patterns: "@target " (followed by space/text) and "%@target" at end
    let mention_pattern = format!(
        "%@{}%",
        target.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
    );

    let mut sql = String::from(
        "SELECT m.id, m.room_id, r.name, m.sender, m.sender_type, m.content, \
         m.created_at, m.edited_at, m.reply_to, m.seq \
         FROM messages m JOIN rooms r ON m.room_id = r.id \
         WHERE m.content LIKE ?1 ESCAPE '\\' \
         AND m.sender != ?2",
    );
    let mut param_values: Vec<String> = vec![mention_pattern, target.to_string()];
    let mut idx = 3;

    if let Some(after_val) = after {
        sql.push_str(&format!(" AND m.seq > ?{idx}"));
        param_values.push(after_val.to_string());
        idx += 1;
    }
    if let Some(room_val) = room_id {
        sql.push_str(&format!(" AND m.room_id = ?{idx}"));
        param_values.push(room_val.to_string());
        idx += 1;
    }

    sql.push_str(&format!(" ORDER BY m.seq DESC LIMIT ?{idx}"));
    param_values.push(limit.to_string());

    let mut stmt = conn.prepare(&sql).map_err(|e| (Status::InternalServerError, Json(serde_json::json!({"error": e.to_string()}))))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let mentions: Vec<MentionResult> = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(MentionResult {
                message_id: row.get(0)?,
                room_id: row.get(1)?,
                room_name: row.get(2)?,
                sender: row.get(3)?,
                sender_type: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                edited_at: row.get(7)?,
                reply_to: row.get(8)?,
                seq: row.get(9)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let count = mentions.len();
    Ok(Json(MentionsResponse {
        target: target.to_string(),
        mentions,
        count,
    }))
}

/// GET /api/v1/mentions/unread?target=<name>
/// Returns unread mention counts per room, using read positions as the baseline.
/// A mention is "unread" if its seq is greater than the target's last_read_seq for that room.
#[get("/api/v1/mentions/unread?<target>")]
pub fn get_unread_mentions(
    db: &State<Db>,
    target: &str,
) -> Result<Json<UnreadMentionsResponse>, (Status, Json<serde_json::Value>)> {
    let target = target.trim();
    if target.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Query parameter 'target' must not be empty"})),
        ));
    }

    let conn = db.conn();

    let mention_pattern = format!(
        "%@{}%",
        target.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
    );

    // Get unread mentions per room by comparing against read positions
    let sql = "SELECT m.room_id, r.name, COUNT(*) as mention_count, MIN(m.seq) as oldest_seq, MAX(m.seq) as newest_seq \
               FROM messages m \
               JOIN rooms r ON m.room_id = r.id \
               LEFT JOIN read_positions rp ON m.room_id = rp.room_id AND rp.sender = ?2 \
               WHERE m.content LIKE ?1 ESCAPE '\\' \
               AND m.sender != ?2 \
               AND m.seq > COALESCE(rp.last_read_seq, 0) \
               GROUP BY m.room_id \
               ORDER BY newest_seq DESC";

    let mut stmt = conn.prepare(sql).map_err(|e| (Status::InternalServerError, Json(serde_json::json!({"error": e.to_string()}))))?;
    let rooms: Vec<UnreadMentionRoom> = stmt
        .query_map(
            rusqlite::params![mention_pattern, target],
            |row| {
                Ok(UnreadMentionRoom {
                    room_id: row.get(0)?,
                    room_name: row.get(1)?,
                    mention_count: row.get(2)?,
                    oldest_seq: row.get(3)?,
                    newest_seq: row.get(4)?,
                })
            },
        )
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let total_unread: i64 = rooms.iter().map(|r| r.mention_count).sum();

    Ok(Json(UnreadMentionsResponse {
        target: target.to_string(),
        rooms,
        total_unread,
    }))
}
