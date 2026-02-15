// Integration test suite for Local Agent Chat
//
// Organized into focused modules by feature area.
// Each module tests a specific API surface.
// All modules share the common::TestClient for DB lifecycle management.

mod common;

mod discover;
mod health_stats;
mod rooms;
mod messages;
mod edit_delete;
mod threading;
mod typing;
mod system;
mod activity;
mod files;
mod pagination;
mod participants;
mod search;
mod reactions;
mod pins;
mod presence;
mod webhooks;
mod threads;
mod read_positions;
mod profiles;
mod dm;
mod mentions;
mod archiving;
mod incoming_webhooks;
mod validation;
mod rate_limit_config;
