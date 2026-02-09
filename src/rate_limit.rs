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

impl RateLimiter {
    pub fn new() -> Self {
        RateLimiter {
            limits: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a request is allowed. Returns true if allowed, false if rate limited.
    /// `key` is typically "action:ip", `max` is max requests, `window_secs` is the time window.
    pub fn check(&self, key: &str, max: usize, window_secs: u64) -> bool {
        let mut limits = self.limits.lock().unwrap();
        let now = Instant::now();
        let window = std::time::Duration::from_secs(window_secs);

        let entries = limits.entry(key.to_string()).or_default();

        // Remove expired entries
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= max {
            return false;
        }

        entries.push(now);
        true
    }
}
