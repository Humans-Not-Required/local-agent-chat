use rusqlite::{Connection, params};
use std::sync::Mutex;

pub struct Db {
    pub conn: Mutex<Connection>,
}

/// Generate a room admin key: `chat_<32 hex chars>`
pub fn generate_admin_key() -> String {
    format!("chat_{:032x}", uuid::Uuid::new_v4().as_u128())
}

impl Db {
    pub fn new(path: &str) -> Self {
        let conn = Connection::open(path).expect("Failed to open database");
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .expect("Failed to set pragmas");
        let db = Db { conn: Mutex::new(conn) };
        db.migrate();
        db
    }

    fn migrate(&self) {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS rooms (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT DEFAULT '',
                created_by TEXT DEFAULT 'anonymous',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
                sender TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL,
                edited_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_messages_room_created ON messages(room_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender);

            -- Migration: add edited_at column if it doesn't exist
            ",
        )
        .expect("Failed to run migrations");

        // Add edited_at column (idempotent — .ok() ignores "duplicate column" errors)
        conn.execute_batch("ALTER TABLE messages ADD COLUMN edited_at TEXT;").ok();

        // Add reply_to column for message threading
        conn.execute_batch("ALTER TABLE messages ADD COLUMN reply_to TEXT;").ok();

        // Add admin_key column for room-scoped admin keys
        conn.execute_batch("ALTER TABLE rooms ADD COLUMN admin_key TEXT;").ok();

        // Add sender_type column for persistent sender type tracking (agent/human)
        conn.execute_batch("ALTER TABLE messages ADD COLUMN sender_type TEXT;").ok();

        // Backfill admin_key for existing rooms that don't have one
        let mut stmt = conn
            .prepare("SELECT id FROM rooms WHERE admin_key IS NULL")
            .unwrap();
        let room_ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        for room_id in room_ids {
            let key = generate_admin_key();
            conn.execute(
                "UPDATE rooms SET admin_key = ?1 WHERE id = ?2",
                params![&key, &room_id],
            )
            .ok();
        }

        // Seed #general room if it doesn't exist
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM rooms WHERE name = 'general'", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            let now = chrono::Utc::now().to_rfc3339();
            let admin_key = generate_admin_key();
            conn.execute(
                "INSERT INTO rooms (id, name, description, created_by, created_at, updated_at, admin_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![uuid::Uuid::new_v4().to_string(), "general", "Default chat room", "system", &now, &now, &admin_key],
            )
            .ok();
        }
    }
}
