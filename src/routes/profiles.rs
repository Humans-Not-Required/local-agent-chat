use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::{Profile, UpsertProfile};
use rocket::serde::json::Json;
use rocket::{delete, get, put, State};
use rusqlite::params;

/// PUT /api/v1/profiles/<sender> — Create or update a profile
#[put("/api/v1/profiles/<sender>", format = "json", data = "<body>")]
pub fn upsert_profile(
    sender: &str,
    body: Json<UpsertProfile>,
    db: &State<Db>,
    events: &State<EventBus>,
) -> Result<Json<Profile>, rocket::http::Status> {
    let conn = db.conn.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();

    // Check if profile already exists
    let existing: Option<Profile> = conn
        .query_row(
            "SELECT sender, display_name, sender_type, avatar_url, bio, status_text, metadata, created_at, updated_at FROM profiles WHERE sender = ?1",
            params![sender],
            |row| {
                let metadata_str: String = row.get(6)?;
                Ok(Profile {
                    sender: row.get(0)?,
                    display_name: row.get(1)?,
                    sender_type: row.get(2)?,
                    avatar_url: row.get(3)?,
                    bio: row.get(4)?,
                    status_text: row.get(5)?,
                    metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .ok();

    let created_at = existing
        .as_ref()
        .map(|p| p.created_at.clone())
        .unwrap_or_else(|| now.clone());

    // Merge: use new values if provided, otherwise keep existing
    let display_name = body
        .display_name
        .clone()
        .or_else(|| existing.as_ref().and_then(|p| p.display_name.clone()));
    let sender_type = body
        .sender_type
        .clone()
        .or_else(|| existing.as_ref().and_then(|p| p.sender_type.clone()));
    let avatar_url = body
        .avatar_url
        .clone()
        .or_else(|| existing.as_ref().and_then(|p| p.avatar_url.clone()));
    let bio = body
        .bio
        .clone()
        .or_else(|| existing.as_ref().and_then(|p| p.bio.clone()));
    let status_text = body
        .status_text
        .clone()
        .or_else(|| existing.as_ref().and_then(|p| p.status_text.clone()));
    let metadata = body
        .metadata
        .clone()
        .unwrap_or_else(|| {
            existing
                .as_ref()
                .map(|p| p.metadata.clone())
                .unwrap_or(serde_json::json!({}))
        });
    let metadata_str = serde_json::to_string(&metadata).unwrap_or_else(|_| "{}".to_string());

    conn.execute(
        "INSERT INTO profiles (sender, display_name, sender_type, avatar_url, bio, status_text, metadata, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(sender) DO UPDATE SET
           display_name = ?2, sender_type = ?3, avatar_url = ?4, bio = ?5,
           status_text = ?6, metadata = ?7, updated_at = ?9",
        params![
            sender,
            &display_name,
            &sender_type,
            &avatar_url,
            &bio,
            &status_text,
            &metadata_str,
            &created_at,
            &now,
        ],
    )
    .map_err(|_| rocket::http::Status::InternalServerError)?;

    let profile = Profile {
        sender: sender.to_string(),
        display_name,
        sender_type,
        avatar_url,
        bio,
        status_text,
        metadata,
        created_at,
        updated_at: now,
    };

    events.publish(ChatEvent::ProfileUpdated(profile.clone()));
    Ok(Json(profile))
}

/// GET /api/v1/profiles/<sender> — Get a single profile
#[get("/api/v1/profiles/<sender>")]
pub fn get_profile(sender: &str, db: &State<Db>) -> Result<Json<Profile>, rocket::http::Status> {
    let conn = db.conn.lock().unwrap();
    let profile = conn
        .query_row(
            "SELECT sender, display_name, sender_type, avatar_url, bio, status_text, metadata, created_at, updated_at FROM profiles WHERE sender = ?1",
            params![sender],
            |row| {
                let metadata_str: String = row.get(6)?;
                Ok(Profile {
                    sender: row.get(0)?,
                    display_name: row.get(1)?,
                    sender_type: row.get(2)?,
                    avatar_url: row.get(3)?,
                    bio: row.get(4)?,
                    status_text: row.get(5)?,
                    metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .map_err(|_| rocket::http::Status::NotFound)?;

    Ok(Json(profile))
}

/// GET /api/v1/profiles?sender_type=agent — List all profiles
#[get("/api/v1/profiles?<sender_type>")]
pub fn list_profiles(
    sender_type: Option<&str>,
    db: &State<Db>,
) -> Json<Vec<Profile>> {
    let conn = db.conn.lock().unwrap();

    let (sql, param_values): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(st) = sender_type {
        (
            "SELECT sender, display_name, sender_type, avatar_url, bio, status_text, metadata, created_at, updated_at FROM profiles WHERE sender_type = ?1 ORDER BY updated_at DESC",
            vec![Box::new(st.to_string()) as Box<dyn rusqlite::types::ToSql>],
        )
    } else {
        (
            "SELECT sender, display_name, sender_type, avatar_url, bio, status_text, metadata, created_at, updated_at FROM profiles ORDER BY updated_at DESC",
            vec![],
        )
    };

    let mut stmt = conn.prepare(sql).unwrap();
    let params: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let profiles = stmt
        .query_map(params.as_slice(), |row| {
            let metadata_str: String = row.get(6)?;
            Ok(Profile {
                sender: row.get(0)?,
                display_name: row.get(1)?,
                sender_type: row.get(2)?,
                avatar_url: row.get(3)?,
                bio: row.get(4)?,
                status_text: row.get(5)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Json(profiles)
}

/// DELETE /api/v1/profiles/<sender> — Delete a profile
#[delete("/api/v1/profiles/<sender>")]
pub fn delete_profile(
    sender: &str,
    db: &State<Db>,
    events: &State<EventBus>,
) -> rocket::http::Status {
    let conn = db.conn.lock().unwrap();

    let affected = conn
        .execute("DELETE FROM profiles WHERE sender = ?1", params![sender])
        .unwrap_or(0);

    if affected == 0 {
        return rocket::http::Status::NotFound;
    }

    events.publish(ChatEvent::ProfileDeleted {
        sender: sender.to_string(),
    });

    rocket::http::Status::NoContent
}
