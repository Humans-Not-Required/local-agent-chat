use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

pub struct RateLimiter {
    limits: Mutex<HashMap<String, Vec<Instant>>>,
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
        let mut limits = self.limits.lock().unwrap();
        let now = Instant::now();
        let window = std::time::Duration::from_secs(window_secs);

        let entries = limits.entry(key.to_string()).or_default();

        // Remove expired entries
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= max {
            // Calculate when the oldest entry will expire
            let oldest = entries.iter().min().unwrap();
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
