use crate::db::Db;
use crate::models::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, State};
#[get("/api/v1/activity?<since>&<limit>&<room_id>&<sender>&<sender_type>&<after>&<exclude_sender>")]
#[allow(clippy::too_many_arguments)]
pub fn activity_feed(
    db: &State<Db>,
    since: Option<&str>,
    limit: Option<i64>,
    room_id: Option<&str>,
    sender: Option<&str>,
    sender_type: Option<&str>,
    after: Option<i64>,
    exclude_sender: Option<&str>,
) -> Json<ActivityResponse> {
    let conn = db.conn();
    let limit = limit.unwrap_or(50).clamp(1, 500);

    let mut sql = String::from(
        "SELECT m.id, m.room_id, r.name, m.sender, m.sender_type, m.content, m.created_at, m.edited_at, m.reply_to, m.seq \
         FROM messages m JOIN rooms r ON m.room_id = r.id WHERE 1=1",
    );
    let mut param_values: Vec<String> = vec![];
    let mut idx = 1;

    if let Some(after_val) = after {
        sql.push_str(&format!(" AND m.seq > ?{idx}"));
        param_values.push(after_val.to_string());
        idx += 1;
    }
    if let Some(since_val) = since {
        sql.push_str(&format!(" AND m.created_at > ?{idx}"));
        param_values.push(since_val.to_string());
        idx += 1;
    }
    if let Some(room_val) = room_id {
        sql.push_str(&format!(" AND m.room_id = ?{idx}"));
        param_values.push(room_val.to_string());
        idx += 1;
    }
    if let Some(sender_val) = sender {
        sql.push_str(&format!(" AND m.sender = ?{idx}"));
        param_values.push(sender_val.to_string());
        idx += 1;
    }
    if let Some(sender_type_val) = sender_type {
        sql.push_str(&format!(" AND m.sender_type = ?{idx}"));
        param_values.push(sender_type_val.to_string());
        idx += 1;
    }
    if let Some(exclude_val) = exclude_sender {
        let excluded: Vec<&str> = exclude_val
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !excluded.is_empty() {
            let placeholders: Vec<String> = excluded
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", idx + i))
                .collect();
            sql.push_str(&format!(
                " AND m.sender NOT IN ({})",
                placeholders.join(",")
            ));
            for name in &excluded {
                param_values.push(name.to_string());
            }
            idx += excluded.len();
        }
    }

    sql.push_str(&format!(" ORDER BY m.seq DESC LIMIT ?{idx}"));
    param_values.push(limit.to_string());

    let mut stmt = conn.prepare(&sql).unwrap();
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let events: Vec<ActivityEvent> = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(ActivityEvent {
                event_type: "message".to_string(),
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

    let count = events.len();
    Json(ActivityResponse {
        events,
        count,
        since: since.map(String::from),
    })
}

#[get("/api/v1/search?<q>&<room_id>&<sender>&<sender_type>&<limit>")]
pub fn search_messages(
    db: &State<Db>,
    q: &str,
    room_id: Option<&str>,
    sender: Option<&str>,
    sender_type: Option<&str>,
    limit: Option<i64>,
) -> Result<Json<SearchResponse>, (Status, Json<serde_json::Value>)> {
    let query = q.trim();
    if query.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Query parameter 'q' must not be empty"})),
        ));
    }
    if query.len() > 500 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Query too long (max 500 characters)"})),
        ));
    }

    let conn = db.conn();
    let limit = limit.unwrap_or(50).clamp(1, 200);

    // Try FTS5 first — falls back to LIKE if FTS fails (e.g. syntax error in query)
    let fts_result: Result<Vec<SearchResult>, rusqlite::Error> = (|| {
        // Build FTS5 query: each word is searched with porter stemming (implicit AND).
        // We strip FTS5 special chars and quote each term for safety.
        let fts_query: String = query
            .split_whitespace()
            .map(|word| {
                // Remove FTS5 special characters to prevent syntax errors
                let clean: String = word
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '\'')
                    .collect();
                // Wrap in quotes for safe matching (porter stemmer still applies)
                let escaped = clean.replace('"', "\"\"");
                format!("\"{escaped}\"")
            })
            .filter(|s| s != "\"\"")
            .collect::<Vec<_>>()
            .join(" ");

        let mut sql = String::from(
            "SELECT m.id, m.room_id, r.name, m.sender, m.sender_type, m.content, \
             m.created_at, m.edited_at, m.reply_to, m.seq \
             FROM messages_fts f \
             JOIN messages m ON m.id = f.message_id \
             JOIN rooms r ON m.room_id = r.id \
             WHERE messages_fts MATCH ?1",
        );
        let mut param_values: Vec<String> = vec![fts_query];
        let mut idx = 2;

        if let Some(room_val) = room_id {
            sql.push_str(&format!(" AND m.room_id = ?{idx}"));
            param_values.push(room_val.to_string());
            idx += 1;
        }
        if let Some(sender_val) = sender {
            sql.push_str(&format!(" AND m.sender = ?{idx}"));
            param_values.push(sender_val.to_string());
            idx += 1;
        }
        if let Some(sender_type_val) = sender_type {
            sql.push_str(&format!(" AND m.sender_type = ?{idx}"));
            param_values.push(sender_type_val.to_string());
            idx += 1;
        }

        sql.push_str(&format!(" ORDER BY rank LIMIT ?{idx}"));
        param_values.push(limit.to_string());

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|v| v as &dyn rusqlite::types::ToSql)
            .collect();

        let results: Vec<SearchResult> = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(SearchResult {
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
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(results)
    })();

    let results = match fts_result {
        Ok(r) => r,
        Err(_) => {
            // Fallback to LIKE search for invalid FTS queries or edge cases
            let escaped = query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let like_pattern = format!("%{escaped}%");

            let mut sql = String::from(
                "SELECT m.id, m.room_id, r.name, m.sender, m.sender_type, m.content, \
                 m.created_at, m.edited_at, m.reply_to, m.seq \
                 FROM messages m JOIN rooms r ON m.room_id = r.id \
                 WHERE m.content LIKE ?1 ESCAPE '\\'",
            );
            let mut param_values: Vec<String> = vec![like_pattern];
            let mut idx = 2;

            if let Some(room_val) = room_id {
                sql.push_str(&format!(" AND m.room_id = ?{idx}"));
                param_values.push(room_val.to_string());
                idx += 1;
            }
            if let Some(sender_val) = sender {
                sql.push_str(&format!(" AND m.sender = ?{idx}"));
                param_values.push(sender_val.to_string());
                idx += 1;
            }
            if let Some(sender_type_val) = sender_type {
                sql.push_str(&format!(" AND m.sender_type = ?{idx}"));
                param_values.push(sender_type_val.to_string());
                idx += 1;
            }

            sql.push_str(&format!(" ORDER BY m.seq DESC LIMIT ?{idx}"));
            param_values.push(limit.to_string());

            let mut stmt = conn.prepare(&sql).map_err(|e| (Status::InternalServerError, Json(serde_json::json!({"error": e.to_string()}))))?;
            let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
                .iter()
                .map(|v| v as &dyn rusqlite::types::ToSql)
                .collect();

            stmt.query_map(params_refs.as_slice(), |row| {
                Ok(SearchResult {
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
            .collect()
        }
    };

    let count = results.len();
    Ok(Json(SearchResponse {
        results,
        count,
        query: query.to_string(),
    }))
}
