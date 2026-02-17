use rocket::http::{ContentType, Header, Status};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::{get, FromForm, Request, Response};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

use crate::db::Db;

/// Query parameters for export
#[derive(Debug, Deserialize, FromForm)]
pub struct ExportQuery {
    /// Export format: json (default), markdown, csv
    pub format: Option<String>,
    /// Filter: only messages after this ISO-8601 timestamp
    pub after: Option<String>,
    /// Filter: only messages before this ISO-8601 timestamp
    pub before: Option<String>,
    /// Filter: only messages from this sender
    pub sender: Option<String>,
    /// Maximum number of messages to export (default: all, max 10000)
    pub limit: Option<i64>,
    /// Include metadata JSON in export (default: false)
    pub include_metadata: Option<bool>,
}

/// A single exported message
#[derive(Debug, Serialize)]
pub struct ExportedMessage {
    pub seq: i64,
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_type: Option<String>,
    pub content: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// JSON export response
#[derive(Debug, Serialize)]
pub struct JsonExportResponse {
    pub room_id: String,
    pub room_name: String,
    pub exported_at: String,
    pub message_count: usize,
    pub filters: ExportFilters,
    pub messages: Vec<ExportedMessage>,
}

#[derive(Debug, Serialize)]
pub struct ExportFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

/// Custom responder for different export formats
pub enum ExportResponse {
    Json(String),
    Markdown(String),
    Csv(String),
}

impl<'r> Responder<'r, 'static> for ExportResponse {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        match self {
            ExportResponse::Json(body) => {
                let filename = "chat-export.json";
                Response::build()
                    .header(ContentType::JSON)
                    .header(Header::new(
                        "Content-Disposition",
                        format!("attachment; filename=\"{filename}\""),
                    ))
                    .sized_body(body.len(), Cursor::new(body))
                    .ok()
            }
            ExportResponse::Markdown(body) => {
                let filename = "chat-export.md";
                Response::build()
                    .header(ContentType::new("text", "markdown"))
                    .header(Header::new(
                        "Content-Disposition",
                        format!("attachment; filename=\"{filename}\""),
                    ))
                    .sized_body(body.len(), Cursor::new(body))
                    .ok()
            }
            ExportResponse::Csv(body) => {
                let filename = "chat-export.csv";
                Response::build()
                    .header(ContentType::CSV)
                    .header(Header::new(
                        "Content-Disposition",
                        format!("attachment; filename=\"{filename}\""),
                    ))
                    .sized_body(body.len(), Cursor::new(body))
                    .ok()
            }
        }
    }
}

/// Export room messages in JSON, Markdown, or CSV format
#[get("/api/v1/rooms/<room_id>/export?<params..>")]
pub fn export_room(
    room_id: &str,
    params: ExportQuery,
    db: &rocket::State<Db>,
) -> Result<ExportResponse, (Status, Json<serde_json::Value>)> {
    let conn = db.conn();

    // Verify room exists and get name
    let room_name: String = conn
        .query_row(
            "SELECT name FROM rooms WHERE id = ?1",
            rusqlite::params![room_id],
            |row| row.get(0),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Room not found"})),
            )
        })?;

    let format = params.format.as_deref().unwrap_or("json");
    if !["json", "markdown", "csv"].contains(&format) {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Invalid format. Supported: json, markdown, csv"})),
        ));
    }

    let limit = params.limit.map(|l| l.clamp(1, 10_000)).unwrap_or(10_000);
    let include_metadata = params.include_metadata.unwrap_or(false);

    // Build query with filters
    let mut conditions = vec!["m.room_id = ?1".to_string()];
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
        vec![Box::new(room_id.to_string())];
    let mut param_idx = 2;

    if let Some(ref after) = params.after {
        conditions.push(format!("m.created_at > ?{param_idx}"));
        param_values.push(Box::new(after.clone()));
        param_idx += 1;
    }

    if let Some(ref before) = params.before {
        conditions.push(format!("m.created_at < ?{param_idx}"));
        param_values.push(Box::new(before.clone()));
        param_idx += 1;
    }

    if let Some(ref sender) = params.sender {
        conditions.push(format!("m.sender = ?{param_idx}"));
        param_values.push(Box::new(sender.clone()));
        param_idx += 1;
    }
    let _ = param_idx; // suppress unused warning

    let where_clause = conditions.join(" AND ");
    let sql = format!(
        "SELECT m.seq, m.sender, m.sender_type, m.content, m.created_at, \
         m.edited_at, m.reply_to, m.pinned_at, m.pinned_by, m.metadata \
         FROM messages m WHERE {where_clause} ORDER BY m.seq ASC LIMIT ?{limit_idx}",
        limit_idx = param_values.len() + 1
    );

    let params_with_limit: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|p| p.as_ref() as &dyn rusqlite::types::ToSql)
        .chain(std::iter::once(&limit as &dyn rusqlite::types::ToSql))
        .collect();

    let messages: Vec<ExportedMessage> = {
        let mut stmt = conn.prepare(&sql).map_err(|_| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?;

        let rows = stmt
            .query_map(params_with_limit.as_slice(), |row| {
                let metadata_str: String = row.get(9)?;
                let metadata_val: serde_json::Value =
                    serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({}));

                Ok(ExportedMessage {
                    seq: row.get(0)?,
                    sender: row.get(1)?,
                    sender_type: row.get(2)?,
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                    edited_at: row.get(5)?,
                    reply_to: row.get(6)?,
                    pinned_at: row.get(7)?,
                    metadata: if include_metadata {
                        Some(metadata_val)
                    } else {
                        None
                    },
                })
            })
            .map_err(|_| {
                (
                    Status::InternalServerError,
                    Json(serde_json::json!({"error": "Internal server error"})),
                )
            })?;

        rows.filter_map(|r| r.ok()).collect()
    };

    let exported_at = chrono::Utc::now().to_rfc3339();

    match format {
        "markdown" => {
            let md = render_markdown(&room_name, room_id, &exported_at, &messages);
            Ok(ExportResponse::Markdown(md))
        }
        "csv" => {
            let csv = render_csv(&messages, include_metadata);
            Ok(ExportResponse::Csv(csv))
        }
        _ => {
            let response = JsonExportResponse {
                room_id: room_id.to_string(),
                room_name,
                exported_at,
                message_count: messages.len(),
                filters: ExportFilters {
                    after: params.after,
                    before: params.before,
                    sender: params.sender,
                    limit: params.limit,
                },
                messages,
            };
            let json_str = serde_json::to_string_pretty(&response).unwrap_or_default();
            Ok(ExportResponse::Json(json_str))
        }
    }
}

fn render_markdown(
    room_name: &str,
    room_id: &str,
    exported_at: &str,
    messages: &[ExportedMessage],
) -> String {
    let mut md = String::new();

    md.push_str(&format!("# #{room_name}\n\n"));
    md.push_str(&format!(
        "> Exported {count} messages on {exported_at}\n",
        count = messages.len()
    ));
    md.push_str(&format!("> Room ID: `{room_id}`\n\n"));
    md.push_str("---\n\n");

    let mut current_date = String::new();

    for msg in messages {
        // Insert date headers when the date changes
        let date = msg.created_at.get(..10).unwrap_or(&msg.created_at);
        if date != current_date {
            if !current_date.is_empty() {
                md.push('\n');
            }
            md.push_str(&format!("## {date}\n\n"));
            current_date = date.to_string();
        }

        // Time only (HH:MM:SS)
        let time = msg
            .created_at
            .get(11..19)
            .unwrap_or(&msg.created_at);

        let sender_badge = match msg.sender_type.as_deref() {
            Some("agent") => " 🤖",
            Some("human") => " 👤",
            _ => "",
        };

        let pin_marker = if msg.pinned_at.is_some() { " 📌" } else { "" };
        let edit_marker = if msg.edited_at.is_some() {
            " *(edited)*"
        } else {
            ""
        };

        let reply_prefix = if let Some(ref reply_to) = msg.reply_to {
            format!("↩ *replying to {reply_to}*\n> ")
        } else {
            String::new()
        };

        md.push_str(&format!(
            "**[{time}] {sender}{sender_badge}**{pin_marker}{edit_marker}\n{reply_prefix}{content}\n\n",
            sender = msg.sender,
            content = msg.content,
        ));
    }

    md
}

fn render_csv(messages: &[ExportedMessage], include_metadata: bool) -> String {
    let mut csv = String::new();

    // Header
    if include_metadata {
        csv.push_str("seq,sender,sender_type,content,created_at,edited_at,reply_to,pinned_at,metadata\n");
    } else {
        csv.push_str("seq,sender,sender_type,content,created_at,edited_at,reply_to,pinned_at\n");
    }

    for msg in messages {
        let sender_type = msg.sender_type.as_deref().unwrap_or("");
        let edited_at = msg.edited_at.as_deref().unwrap_or("");
        let reply_to = msg.reply_to.as_deref().unwrap_or("");
        let pinned_at = msg.pinned_at.as_deref().unwrap_or("");

        csv.push_str(&format!(
            "{seq},{sender},{sender_type},{content},{created_at},{edited_at},{reply_to},{pinned_at}",
            seq = msg.seq,
            sender = csv_escape(&msg.sender),
            content = csv_escape(&msg.content),
            created_at = csv_escape(&msg.created_at),
            edited_at = csv_escape(edited_at),
            reply_to = csv_escape(reply_to),
            pinned_at = csv_escape(pinned_at),
        ));

        if include_metadata {
            let meta_str = msg
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_default())
                .unwrap_or_default();
            csv.push_str(&format!(",{}", csv_escape(&meta_str)));
        }

        csv.push('\n');
    }

    csv
}

/// Escape a string for CSV: wrap in quotes if it contains commas, quotes, or newlines
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
