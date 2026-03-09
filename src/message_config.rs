use std::env;

/// Configurable message delivery limits.
///
/// Environment variables:
/// - `MESSAGE_DEFAULT_LIMIT_API` — Default `limit` for API queries (default: 50)
/// - `MESSAGE_DEFAULT_LIMIT_UI`  — Default limit hint for UI consumers (default: 200)
/// - `MESSAGE_MAX_LIMIT`         — Hard cap on `limit` for all queries (default: 500)
///
/// Full message history is always retained; these settings only affect
/// how many messages are *delivered* in a single request.
pub struct MessageConfig {
    pub default_limit_api: i64,
    pub default_limit_ui: i64,
    pub max_limit: i64,
}

impl Default for MessageConfig {
    fn default() -> Self {
        Self {
            default_limit_api: 50,
            default_limit_ui: 200,
            max_limit: 500,
        }
    }
}

impl MessageConfig {
    pub fn from_env() -> Self {
        let default_limit_api = env::var("MESSAGE_DEFAULT_LIMIT_API")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let default_limit_ui = env::var("MESSAGE_DEFAULT_LIMIT_UI")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(200);
        let max_limit = env::var("MESSAGE_MAX_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500);

        Self {
            default_limit_api,
            default_limit_ui,
            max_limit,
        }
    }

    /// Clamp a caller-supplied limit against the hard cap.
    /// Falls back to `default_limit_api` when None is given.
    pub fn resolve(&self, limit: Option<i64>) -> i64 {
        limit
            .unwrap_or(self.default_limit_api)
            .clamp(1, self.max_limit)
    }
}
