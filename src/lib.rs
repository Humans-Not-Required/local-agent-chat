pub mod db;
pub mod events;
pub mod mdns;
pub mod models;
pub mod rate_limit;
pub mod retention;
pub mod routes;
pub mod webhooks;

use db::Db;
use events::EventBus;
use rate_limit::{RateLimitConfig, RateLimiter};
use rocket::fs::{FileServer, Options};
use rocket_cors::CorsOptions;
use routes::{PresenceTracker, TypingTracker};
use std::env;
use std::path::PathBuf;

pub fn rocket() -> rocket::Rocket<rocket::Build> {
    let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "data/chat.db".to_string());
    rocket_with_db(&db_path)
}

pub fn rocket_with_db_and_config(db_path: &str, rate_config: RateLimitConfig) -> rocket::Rocket<rocket::Build> {
    build_rocket(db_path, rate_config)
}

pub fn rocket_with_db(db_path: &str) -> rocket::Rocket<rocket::Build> {
    let rate_limit_config = RateLimitConfig::from_env();
    build_rocket(db_path, rate_limit_config)
}

fn build_rocket(db_path: &str, rate_limit_config: RateLimitConfig) -> rocket::Rocket<rocket::Build> {
    // Ensure data directory exists
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db = Db::new(db_path);
    let events = EventBus::new();

    // Subscribe webhook dispatcher BEFORE handing EventBus to Rocket
    let webhook_receiver = events.sender.subscribe();
    let webhook_db_path = db_path.to_string();

    let rate_limiter = RateLimiter::new();
    let typing_tracker = TypingTracker::default();
    let presence_tracker = PresenceTracker::default();

    let cors = CorsOptions::default()
        .to_cors()
        .expect("Failed to create CORS");

    // Increase JSON data limit to 10MB to accommodate base64-encoded file uploads
    // (5MB file = ~6.7MB base64 + JSON wrapper)
    let figment = rocket::Config::figment().merge(("limits.json", 10 * 1024 * 1024)); // 10MB

    // Frontend static files directory
    let static_dir: PathBuf = env::var("STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("frontend/dist"));

    let mut build = rocket::custom(figment)
        .manage(db)
        .manage(events)
        .manage(rate_limit_config)
        .manage(rate_limiter)
        .manage(typing_tracker)
        .manage(presence_tracker)
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
                routes::update_room,
                routes::archive_room,
                routes::unarchive_room,
                routes::delete_room,
                routes::send_message,
                routes::edit_message,
                routes::get_edit_history,
                routes::delete_message,
                routes::get_messages,
                routes::activity_feed,
                routes::search_messages,
                routes::room_participants,
                routes::notify_typing,
                routes::message_stream,
                routes::upload_file,
                routes::download_file,
                routes::file_info,
                routes::list_files,
                routes::delete_file,
                routes::add_reaction,
                routes::remove_reaction,
                routes::get_reactions,
                routes::get_room_reactions,
                routes::pin_message,
                routes::unpin_message,
                routes::list_pins,
                routes::room_presence,
                routes::global_presence,
                routes::create_webhook,
                routes::list_webhooks,
                routes::update_webhook,
                routes::delete_webhook,
                routes::get_webhook_deliveries,
                routes::get_thread,
                routes::update_read_position,
                routes::get_read_positions,
                routes::get_unread,
                routes::upsert_profile,
                routes::get_profile,
                routes::list_profiles,
                routes::delete_profile,
                routes::send_dm,
                routes::list_dm_conversations,
                routes::get_dm_conversation,
                routes::get_mentions,
                routes::get_unread_mentions,
                routes::create_incoming_webhook,
                routes::list_incoming_webhooks,
                routes::update_incoming_webhook,
                routes::delete_incoming_webhook,
                routes::post_via_hook,
                routes::add_bookmark,
                routes::remove_bookmark,
                routes::list_bookmarks,
                routes::service_discover,
                routes::llms_txt_root,
                routes::llms_txt_api,
                routes::openapi_json,
                routes::skills_index,
                routes::skills_skill_md,
                routes::api_skills_skill_md,
                routes::run_retention_now,
                routes::export_room,
                routes::broadcast_message,
            ],
        )
        .attach(rocket::fairing::AdHoc::on_liftoff(
            "Webhook Dispatcher",
            move |_rocket| {
                Box::pin(async move {
                    webhooks::spawn_dispatcher(webhook_receiver, webhook_db_path);
                    println!("🔗 Webhook dispatcher started");
                })
            },
        ))
        .attach(rocket::fairing::AdHoc::on_liftoff(
            "Message Retention",
            {
                let retention_db_path = db_path.to_string();
                move |_rocket| {
                    Box::pin(async move {
                        retention::spawn_retention_task(retention_db_path);
                        println!("🧹 Message retention task started");
                    })
                }
            },
        ))
        .attach(rocket::fairing::AdHoc::on_liftoff(
            "mDNS Service Discovery",
            |_rocket| {
                Box::pin(async move {
                    let mdns_enabled = env::var("MDNS_ENABLED")
                        .map(|v| v != "0" && v.to_lowercase() != "false")
                        .unwrap_or(true);

                    if !mdns_enabled {
                        println!("📡 mDNS service discovery disabled (MDNS_ENABLED=false)");
                        return;
                    }

                    let port: u16 = env::var("ROCKET_PORT")
                        .unwrap_or_else(|_| "8000".to_string())
                        .parse()
                        .unwrap_or(8000);

                    let instance_name = env::var("MDNS_INSTANCE_NAME")
                        .unwrap_or_else(|_| "local-agent-chat".to_string());

                    match mdns::start_mdns(port, &instance_name) {
                        Ok(handle) => {
                            println!(
                                "📡 mDNS advertising: {} on port {}",
                                handle.fullname(),
                                port
                            );
                            // Leak the handle to keep mDNS alive for the lifetime of the server.
                            // Rocket doesn't provide a clean on_shutdown hook for managed state
                            // cleanup, so this is the pragmatic approach. The OS reclaims
                            // resources on process exit.
                            std::mem::forget(handle);
                        }
                        Err(e) => {
                            eprintln!("⚠️  mDNS failed to start: {e} (discovery disabled, API still works)");
                        }
                    }
                })
            },
        ));

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
