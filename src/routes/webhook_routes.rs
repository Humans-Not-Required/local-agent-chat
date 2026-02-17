use crate::db::Db;
use crate::models::*;
use rocket::form::FromForm;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put, State};
use rusqlite::params;

use super::AdminKey;

/// Helper to verify room exists and admin key matches.
fn verify_room_admin(
    db: &Db,
    room_id: &str,
    admin: &AdminKey,
) -> Result<(), (Status, Json<serde_json::Value>)> {
    let conn = db.conn();
    let stored_key: Option<String> = conn
        .query_row(
            "SELECT admin_key FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get(0),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Room not found"})),
            )
        })?;

    match stored_key {
        Some(ref key) if key == &admin.0 => Ok(()),
        _ => Err((
            Status::Forbidden,
            Json(serde_json::json!({"error": "Invalid admin key for this room"})),
        )),
    }
}

#[post("/api/v1/rooms/<room_id>/webhooks", format = "json", data = "<body>")]
pub fn create_webhook(
    db: &State<Db>,
    room_id: &str,
    admin: AdminKey,
    body: Json<CreateWebhook>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    verify_room_admin(db, room_id, &admin)?;
    let conn = db.conn();

    // Validate URL
    let url = body.url.trim().to_string();
    if url.is_empty() || (!url.starts_with("http://") && !url.starts_with("https://")) {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Invalid webhook URL: must start with http:// or https://"})),
        ));
    }

    // Validate events filter
    let events = body.events.trim().to_string();
    if events.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Events filter cannot be empty. Use '*' for all events."})),
        ));
    }
    if events != "*" {
        let valid_events = [
            "message",
            "message_edited",
            "message_deleted",
            "file_uploaded",
            "file_deleted",
            "reaction_added",
            "reaction_removed",
            "message_pinned",
            "message_unpinned",
            "presence_joined",
            "presence_left",
            "room_updated",
        ];
        for ev in events.split(',').map(|s| s.trim()) {
            if !valid_events.contains(&ev) {
                return Err((
                    Status::BadRequest,
                    Json(serde_json::json!({"error": format!("Unknown event type: '{}'. Valid events: {}", ev, valid_events.join(", "))})),
                ));
            }
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO webhooks (id, room_id, url, events, secret, created_by, created_at, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![&id, room_id, &url, &events, &body.secret, &body.created_by, &now],
    )
    .map_err(|_e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": "Internal server error"})),
        )
    })?;

    Ok(Json(serde_json::json!({
        "id": id,
        "room_id": room_id,
        "url": url,
        "events": events,
        "has_secret": body.secret.is_some(),
        "created_by": body.created_by,
        "created_at": now,
        "active": true
    })))
}

#[get("/api/v1/rooms/<room_id>/webhooks")]
pub fn list_webhooks(
    db: &State<Db>,
    room_id: &str,
    admin: AdminKey,
) -> Result<Json<Vec<Webhook>>, (Status, Json<serde_json::Value>)> {
    verify_room_admin(db, room_id, &admin)?;
    let conn = db.conn();

    let mut stmt = conn
        .prepare(
            "SELECT id, room_id, url, events, created_by, created_at, active FROM webhooks WHERE room_id = ?1 ORDER BY created_at DESC",
        )
        .map_err(|_| (Status::InternalServerError, Json(serde_json::json!({"error": "Internal server error"}))))?;

    let webhooks: Vec<Webhook> = stmt
        .query_map(params![room_id], |row| {
            Ok(Webhook {
                id: row.get(0)?,
                room_id: row.get(1)?,
                url: row.get(2)?,
                events: row.get(3)?,
                created_by: row.get(4)?,
                created_at: row.get(5)?,
                active: row.get::<_, i32>(6)? != 0,
            })
        })
        .map_err(|_| (Status::InternalServerError, Json(serde_json::json!({"error": "Internal server error"}))))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(webhooks))
}

#[put(
    "/api/v1/rooms/<room_id>/webhooks/<webhook_id>",
    format = "json",
    data = "<body>"
)]
pub fn update_webhook(
    db: &State<Db>,
    room_id: &str,
    webhook_id: &str,
    admin: AdminKey,
    body: Json<UpdateWebhook>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    verify_room_admin(db, room_id, &admin)?;
    let conn = db.conn();

    // Verify webhook exists in this room
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM webhooks WHERE id = ?1 AND room_id = ?2",
            params![webhook_id, room_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if !exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Webhook not found"})),
        ));
    }

    // Build dynamic UPDATE
    let mut updates: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(ref url) = body.url {
        let url = url.trim();
        if url.is_empty() || (!url.starts_with("http://") && !url.starts_with("https://")) {
            return Err((
                Status::BadRequest,
                Json(serde_json::json!({"error": "Invalid webhook URL"})),
            ));
        }
        updates.push(format!("url = ?{}", idx));
        values.push(Box::new(url.to_string()));
        idx += 1;
    }
    if let Some(ref events) = body.events {
        updates.push(format!("events = ?{}", idx));
        values.push(Box::new(events.clone()));
        idx += 1;
    }
    if let Some(ref secret) = body.secret {
        updates.push(format!("secret = ?{}", idx));
        values.push(Box::new(secret.clone()));
        idx += 1;
    }
    if let Some(active) = body.active {
        updates.push(format!("active = ?{}", idx));
        values.push(Box::new(active as i32));
        idx += 1;
    }

    if updates.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "No fields to update"})),
        ));
    }

    // Add webhook_id and room_id as final params
    let sql = format!(
        "UPDATE webhooks SET {} WHERE id = ?{} AND room_id = ?{}",
        updates.join(", "),
        idx,
        idx + 1
    );
    values.push(Box::new(webhook_id.to_string()));
    values.push(Box::new(room_id.to_string()));

    let param_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, param_refs.as_slice()).map_err(|_e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": "Internal server error"})),
        )
    })?;

    Ok(Json(
        serde_json::json!({"updated": true, "id": webhook_id}),
    ))
}

#[delete("/api/v1/rooms/<room_id>/webhooks/<webhook_id>")]
pub fn delete_webhook(
    db: &State<Db>,
    room_id: &str,
    webhook_id: &str,
    admin: AdminKey,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    verify_room_admin(db, room_id, &admin)?;
    let conn = db.conn();

    let deleted = conn
        .execute(
            "DELETE FROM webhooks WHERE id = ?1 AND room_id = ?2",
            params![webhook_id, room_id],
        )
        .unwrap_or(0);

    if deleted == 0 {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Webhook not found"})),
        ));
    }

    Ok(Json(
        serde_json::json!({"deleted": true, "id": webhook_id}),
    ))
}

#[derive(Debug, FromForm)]
pub struct DeliveryQuery {
    pub limit: Option<i64>,
    pub after: Option<String>,
    pub event: Option<String>,
    pub status: Option<String>,
}

#[get("/api/v1/rooms/<room_id>/webhooks/<webhook_id>/deliveries?<query..>")]
pub fn get_webhook_deliveries(
    db: &State<Db>,
    room_id: &str,
    webhook_id: &str,
    admin: AdminKey,
    query: DeliveryQuery,
) -> Result<Json<Vec<WebhookDeliveryLog>>, (Status, Json<serde_json::Value>)> {
    verify_room_admin(db, room_id, &admin)?;
    let conn = db.conn();

    // Verify webhook exists in this room
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM webhooks WHERE id = ?1 AND room_id = ?2",
            params![webhook_id, room_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if !exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Webhook not found"})),
        ));
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let mut sql = String::from(
        "SELECT id, delivery_group, webhook_id, event, url, attempt, status, status_code, error_message, response_time_ms, created_at FROM webhook_deliveries WHERE webhook_id = ?1",
    );
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(webhook_id.to_string())];
    let mut idx = 2;

    if let Some(ref after_cursor) = query.after {
        sql.push_str(&format!(" AND created_at < ?{}", idx));
        param_values.push(Box::new(after_cursor.clone()));
        idx += 1;
    }
    if let Some(ref ev) = query.event {
        sql.push_str(&format!(" AND event = ?{}", idx));
        param_values.push(Box::new(ev.clone()));
        idx += 1;
    }
    if let Some(ref st) = query.status {
        sql.push_str(&format!(" AND status = ?{}", idx));
        param_values.push(Box::new(st.clone()));
        idx += 1;
    }

    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ?{}", idx));
    param_values.push(Box::new(limit));

    let param_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|v| v.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).map_err(|_| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": "Internal server error"})),
        )
    })?;

    let deliveries: Vec<WebhookDeliveryLog> = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(WebhookDeliveryLog {
                id: row.get(0)?,
                delivery_group: row.get(1)?,
                webhook_id: row.get(2)?,
                event: row.get(3)?,
                url: row.get(4)?,
                attempt: row.get(5)?,
                status: row.get(6)?,
                status_code: row.get(7)?,
                error_message: row.get(8)?,
                response_time_ms: row.get(9)?,
                created_at: row.get(10)?,
            })
        })
        .map_err(|_| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(deliveries))
}
