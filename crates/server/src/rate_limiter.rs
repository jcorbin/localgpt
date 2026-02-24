//! Token bucket rate limiter per IP address.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use localgpt_core::config::RateLimitConfig;

/// Token bucket state for a single IP.
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

/// Per-IP token bucket rate limiter.
pub struct RateLimiter {
    buckets: Mutex<HashMap<IpAddr, Bucket>>,
    rate: f64,       // tokens per second
    max_tokens: f64, // burst capacity
    enabled: bool,
}

impl RateLimiter {
    pub fn new(config: &RateLimitConfig) -> Self {
        let rate = config.requests_per_minute as f64 / 60.0;
        let max_tokens = rate + config.burst as f64;
        Self {
            buckets: Mutex::new(HashMap::new()),
            rate,
            max_tokens,
            enabled: config.enabled,
        }
    }

    /// Try to consume one token for the given IP. Returns true if allowed.
    pub async fn check(&self, ip: IpAddr) -> bool {
        if !self.enabled {
            return true;
        }

        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();

        let bucket = buckets.entry(ip).or_insert_with(|| Bucket {
            tokens: self.max_tokens,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.rate).min(self.max_tokens);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Remove buckets that haven't been used in 5 minutes.
    pub async fn cleanup(&self) {
        let mut buckets = self.buckets.lock().await;
        let cutoff = Duration::from_secs(300);
        buckets.retain(|_, b| b.last_refill.elapsed() < cutoff);
    }
}

/// Create a shared rate limiter and spawn a background cleanup task.
pub fn create_rate_limiter(config: &RateLimitConfig) -> Arc<RateLimiter> {
    let limiter = Arc::new(RateLimiter::new(config));

    let cleanup = limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            cleanup.cleanup().await;
        }
    });

    limiter
}
