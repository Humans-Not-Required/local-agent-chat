use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

/// Interval between retention sweeps (seconds).
const RETENTION_INTERVAL_SECS: u64 = 60;

/// Spawns a background task that periodically prunes messages based on room retention settings.
///
/// Rooms can configure:
/// - `max_messages`: Keep at most N messages (oldest pruned first). Pinned messages are exempt.
/// - `max_message_age_hours`: Delete messages older than N hours. Pinned messages are exempt.
///
/// Both settings can be combined. Pruning also cleans up the FTS index.
/// CASCADE deletes handle reactions automatically.
pub fn spawn_retention_task(db_path: String) {
    tokio::spawn(async move {
        let conn = Arc::new(Mutex::new(match Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("⚠️ Retention task: failed to open DB: {e}");
                return;
            }
        }));
        {
            let db = conn.lock().unwrap_or_else(|e| e.into_inner());
            db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
                .ok();
        }

        // Initial delay: let the server start up before the first sweep
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        loop {
            {
                let db = conn.lock().unwrap_or_else(|e| {
                    eprintln!("WARN: Retention task DB mutex poisoned, recovering");
                    e.into_inner()
                });
                run_retention(&db);
            }
            tokio::time::sleep(std::time::Duration::from_secs(RETENTION_INTERVAL_SECS)).await;
        }
    });
}

/// Execute one retention sweep across all rooms with retention settings.
fn run_retention(conn: &Connection) {
    // Find rooms with any retention settings
    let rooms: Vec<(String, Option<i64>, Option<i64>)> = {
        let mut stmt = match conn.prepare(
            "SELECT id, max_messages, max_message_age_hours FROM rooms
             WHERE max_messages IS NOT NULL OR max_message_age_hours IS NOT NULL",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        match stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => return,
        }
    };

    for (room_id, max_messages, max_age_hours) in rooms {
        let mut total_pruned = 0i64;

        // Prune by max_messages (keep newest N, pinned messages exempt)
        if let Some(max) = max_messages {
            let pruned = prune_by_count(conn, &room_id, max);
            total_pruned += pruned;
        }

        // Prune by max_message_age_hours (pinned messages exempt)
        if let Some(hours) = max_age_hours {
            let pruned = prune_by_age(conn, &room_id, hours);
            total_pruned += pruned;
        }

        if total_pruned > 0 {
            eprintln!(
                "🧹 Retention: pruned {} messages from room {}",
                total_pruned, room_id
            );
        }
    }
}

/// Delete oldest non-pinned messages beyond the count limit. Returns number pruned.
fn prune_by_count(conn: &Connection, room_id: &str, max_messages: i64) -> i64 {
    // Get IDs of non-pinned messages to delete (oldest first, beyond the limit)
    let ids_to_delete: Vec<String> = {
        // Count non-pinned messages
        let non_pinned_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE room_id = ?1 AND pinned_at IS NULL",
                params![room_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if non_pinned_count <= max_messages {
            return 0;
        }

        let excess = non_pinned_count - max_messages;
        let mut stmt = match conn.prepare(
            "SELECT id FROM messages WHERE room_id = ?1 AND pinned_at IS NULL ORDER BY seq ASC LIMIT ?2",
        ) {
            Ok(s) => s,
            Err(_) => return 0,
        };
        match stmt.query_map(params![room_id, excess], |row| row.get(0)) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => return 0,
        }
    };

    delete_messages(conn, &ids_to_delete)
}

/// Delete non-pinned messages older than the specified hours. Returns number pruned.
fn prune_by_age(conn: &Connection, room_id: &str, max_age_hours: i64) -> i64 {
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours);
    let cutoff_str = cutoff.to_rfc3339();

    let ids_to_delete: Vec<String> = {
        let mut stmt = match conn.prepare(
            "SELECT id FROM messages WHERE room_id = ?1 AND pinned_at IS NULL AND created_at < ?2",
        ) {
            Ok(s) => s,
            Err(_) => return 0,
        };
        match stmt.query_map(params![room_id, cutoff_str], |row| row.get(0)) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => return 0,
        }
    };

    delete_messages(conn, &ids_to_delete)
}

/// Delete messages by ID, cleaning up FTS index first. Returns count deleted.
fn delete_messages(conn: &Connection, ids: &[String]) -> i64 {
    if ids.is_empty() {
        return 0;
    }

    let mut deleted = 0i64;

    // Process in batches to avoid SQLite variable limit
    for chunk in ids.chunks(500) {
        let placeholders: Vec<String> = (0..chunk.len()).map(|i| format!("?{}", i + 1)).collect();
        let placeholder_str = placeholders.join(",");

        // Clean up FTS index
        let fts_sql = format!(
            "DELETE FROM messages_fts WHERE message_id IN ({})",
            placeholder_str
        );
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            chunk.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        conn.execute(&fts_sql, params_refs.as_slice()).ok();

        // Delete messages (CASCADE handles reactions)
        let del_sql = format!(
            "DELETE FROM messages WHERE id IN ({})",
            placeholder_str
        );
        if let Ok(n) = conn.execute(&del_sql, params_refs.as_slice()) {
            deleted += n as i64;
        }
    }

    deleted
}
