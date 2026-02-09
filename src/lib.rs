pub mod db;
pub mod events;
pub mod models;
pub mod rate_limit;
pub mod routes;

use db::Db;
use events::EventBus;
use rate_limit::RateLimiter;
use rocket_cors::CorsOptions;
use std::env;

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

    let cors = CorsOptions::default()
        .to_cors()
        .expect("Failed to create CORS");

    rocket::build()
        .manage(db)
        .manage(events)
        .manage(rate_limiter)
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
                routes::get_messages,
                routes::message_stream,
                routes::llms_txt_root,
                routes::llms_txt_api,
                routes::openapi_json,
            ],
        )
}
