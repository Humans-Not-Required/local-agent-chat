use std::collections::HashMap;
use std::env;
use std::sync::Mutex;
use std::time::Instant;

use rocket::http::Header;
use rocket::response::{self, Responder, Response};
use rocket::serde::json::Json;
use rocket::Request;

/// Configurable rate limit values. All read from environment variables with sensible defaults.
///
/// Environment variables:
/// - `RATE_LIMIT_MESSAGES` — Max messages per minute per IP (default: 60)
/// - `RATE_LIMIT_ROOMS` — Max room creations per hour per IP (default: 10)
/// - `RATE_LIMIT_FILES` — Max file uploads per minute per IP (default: 10)
/// - `RATE_LIMIT_DMS` — Max DMs per minute per IP (default: 60)
/// - `RATE_LIMIT_WEBHOOKS` — Max incoming webhook messages per minute per token (default: 60)
pub struct RateLimitConfig {
    /// Messages per minute per IP
    pub messages_max: usize,
    pub messages_window_secs: u64,
    /// Room creations per hour per IP
    pub rooms_max: usize,
    pub rooms_window_secs: u64,
    /// File uploads per minute per IP
    pub files_max: usize,
    pub files_window_secs: u64,
    /// DMs per minute per IP
    pub dms_max: usize,
    pub dms_window_secs: u64,
    /// Incoming webhook messages per minute per token
    pub webhooks_max: usize,
    pub webhooks_window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            messages_max: 60,
            messages_window_secs: 60,
            rooms_max: 10,
            rooms_window_secs: 3600,
            files_max: 10,
            files_window_secs: 60,
            dms_max: 60,
            dms_window_secs: 60,
            webhooks_max: 60,
            webhooks_window_secs: 60,
        }
    }
}

impl RateLimitConfig {
    /// Create a new RateLimitConfig from environment variables, with defaults.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = env::var("RATE_LIMIT_MESSAGES")
            && let Ok(n) = val.parse::<usize>()
        {
            config.messages_max = n;
        }
        if let Ok(val) = env::var("RATE_LIMIT_ROOMS")
            && let Ok(n) = val.parse::<usize>()
        {
            config.rooms_max = n;
        }
        if let Ok(val) = env::var("RATE_LIMIT_FILES")
            && let Ok(n) = val.parse::<usize>()
        {
            config.files_max = n;
        }
        if let Ok(val) = env::var("RATE_LIMIT_DMS")
            && let Ok(n) = val.parse::<usize>()
        {
            config.dms_max = n;
        }
        if let Ok(val) = env::var("RATE_LIMIT_WEBHOOKS")
            && let Ok(n) = val.parse::<usize>()
        {
            config.webhooks_max = n;
        }

        config
    }
}

pub struct RateLimiter {
    limits: Mutex<HashMap<String, Vec<Instant>>>,
}

/// Wrapper that adds standard rate limit headers to any JSON response.
/// Headers: X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
pub struct RateLimited<T> {
    pub inner: Json<T>,
    pub info: RateLimitInfo,
}

impl<T> RateLimited<T> {
    pub fn new(inner: Json<T>, info: RateLimitInfo) -> Self {
        Self { inner, info }
    }
}

impl<'r, 'o: 'r, T: serde::Serialize + 'o> Responder<'r, 'o> for RateLimited<T> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        let mut response = Response::build_from(self.inner.respond_to(req)?)
            .header(Header::new(
                "X-RateLimit-Limit",
                self.info.limit.to_string(),
            ))
            .header(Header::new(
                "X-RateLimit-Remaining",
                self.info.remaining.to_string(),
            ))
            .header(Header::new(
                "X-RateLimit-Reset",
                self.info.retry_after_secs.to_string(),
            ))
            .finalize();

        // Add Retry-After header for 429 responses
        if !self.info.allowed {
            response.set_header(Header::new(
                "Retry-After",
                self.info.retry_after_secs.to_string(),
            ));
        }

        Ok(response)
    }
}

/// Error responder for rate-limited (429) responses with proper headers.
pub struct RateLimitedError {
    pub info: RateLimitInfo,
    pub message: String,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for RateLimitedError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        let body = Json(serde_json::json!({
            "error": self.message,
            "retry_after_secs": self.info.retry_after_secs,
            "limit": self.info.limit,
            "remaining": 0
        }));

        Response::build_from(body.respond_to(req)?)
            .status(rocket::http::Status::TooManyRequests)
            .header(Header::new(
                "X-RateLimit-Limit",
                self.info.limit.to_string(),
            ))
            .header(Header::new("X-RateLimit-Remaining", "0".to_string()))
            .header(Header::new(
                "X-RateLimit-Reset",
                self.info.retry_after_secs.to_string(),
            ))
            .header(Header::new(
                "Retry-After",
                self.info.retry_after_secs.to_string(),
            ))
            .ok()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about rate limit status for a given key.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub allowed: bool,
    pub limit: usize,
    pub remaining: usize,
    /// Seconds until the oldest request in the window expires (i.e. a slot opens).
    /// 0 if there's remaining capacity.
    pub retry_after_secs: u64,
}

impl RateLimiter {
    pub fn new() -> Self {
        RateLimiter {
            limits: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a request is allowed. Returns true if allowed, false if rate limited.
    /// `key` is typically "action:ip", `max` is max requests, `window_secs` is the time window.
    pub fn check(&self, key: &str, max: usize, window_secs: u64) -> bool {
        self.check_with_info(key, max, window_secs).allowed
    }

    /// Check rate limit and return detailed info for response headers.
    pub fn check_with_info(&self, key: &str, max: usize, window_secs: u64) -> RateLimitInfo {
        let mut limits = self.limits.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let window = std::time::Duration::from_secs(window_secs);

        let entries = limits.entry(key.to_string()).or_default();

        // Remove expired entries
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= max {
            // Calculate when the oldest entry will expire
            let oldest = match entries.iter().min() {
                Some(t) => t,
                None => return RateLimitInfo { allowed: false, remaining: 0, limit: max, retry_after_secs: 1 },
            };
            let elapsed = now.duration_since(*oldest);
            let retry_after = if elapsed < window {
                (window - elapsed).as_secs() + 1 // +1 to ensure the slot is actually open
            } else {
                1
            };

            return RateLimitInfo {
                allowed: false,
                limit: max,
                remaining: 0,
                retry_after_secs: retry_after,
            };
        }

        entries.push(now);
        let remaining = max - entries.len();

        RateLimitInfo {
            allowed: true,
            limit: max,
            remaining,
            retry_after_secs: 0,
        }
    }
}
