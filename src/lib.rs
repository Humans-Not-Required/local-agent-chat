pub mod db;
pub mod events;
pub mod models;
pub mod rate_limit;
pub mod routes;

use db::Db;
use events::EventBus;
use rate_limit::RateLimiter;
use routes::TypingTracker;
use rocket::fs::{FileServer, Options};
use rocket_cors::CorsOptions;
use std::env;
use std::path::PathBuf;

pub fn rocket() -> rocket::Rocket<rocket::Build> {
    let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "data/chat.db".to_string());
    rocket_with_db(&db_path)
}

pub fn rocket_with_db(db_path: &str) -> rocket::Rocket<rocket::Build> {
    // Ensure data directory exists
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db = Db::new(db_path);
    let events = EventBus::new();
    let rate_limiter = RateLimiter::new();
    let typing_tracker = TypingTracker::default();

    let cors = CorsOptions::default()
        .to_cors()
        .expect("Failed to create CORS");

    // Increase JSON data limit to 10MB to accommodate base64-encoded file uploads
    // (5MB file = ~6.7MB base64 + JSON wrapper)
    let figment = rocket::Config::figment()
        .merge(("limits.json", 10 * 1024 * 1024)); // 10MB

    // Frontend static files directory
    let static_dir: PathBuf = env::var("STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("frontend/dist"));

    let mut build = rocket::custom(figment)
        .manage(db)
        .manage(events)
        .manage(rate_limiter)
        .manage(typing_tracker)
        .attach(cors)
        .register(
            "/",
            rocket::catchers![routes::too_many_requests, routes::not_found],
        )
        .mount(
            "/",
            rocket::routes![
                routes::health,
                routes::stats,
                routes::create_room,
                routes::list_rooms,
                routes::get_room,
                routes::delete_room,
                routes::send_message,
                routes::edit_message,
                routes::delete_message,
                routes::get_messages,
                routes::activity_feed,
                routes::notify_typing,
                routes::message_stream,
                routes::upload_file,
                routes::download_file,
                routes::file_info,
                routes::list_files,
                routes::delete_file,
                routes::llms_txt_root,
                routes::llms_txt_api,
                routes::openapi_json,
            ],
        );

    // Serve frontend static files if the directory exists
    if static_dir.is_dir() {
        println!("📦 Serving frontend from: {}", static_dir.display());
        build = build
            .mount("/", FileServer::new(&static_dir, Options::Index))
            .mount("/", rocket::routes![routes::spa_fallback]);
    } else {
        println!(
            "⚠️  Frontend directory not found: {} (API-only mode)",
            static_dir.display()
        );
    }

    build
}
