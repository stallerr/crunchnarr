//! Rate limiting for downloads using a token bucket algorithm.
//!
//! Provides a global rate limiter that can be shared across concurrent downloads
//! to enforce a maximum total download speed.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// A token bucket rate limiter for controlling download speed.
///
/// Tokens represent bytes that can be downloaded. Tokens are refilled at
/// `rate_bps` (bytes per second). When acquiring tokens, if not enough are
/// available, the caller sleeps until sufficient tokens have accumulated.
#[derive(Debug)]
pub struct RateLimiter {
    inner: Mutex<RateLimiterInner>,
    rate_bps: u64,
}

#[derive(Debug)]
struct RateLimiterInner {
    /// Available tokens (bytes).
    tokens: f64,
    /// Maximum tokens (burst size).
    max_tokens: f64,
    /// Last time tokens were refilled.
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the given rate in bytes per second.
    ///
    /// The burst size is set to 1 second worth of tokens, allowing for
    /// some burstiness while maintaining the average rate.
    pub fn new(rate_bps: u64) -> Arc<Self> {
        let max_tokens = rate_bps as f64; // 1 second burst

        Arc::new(Self {
            inner: Mutex::new(RateLimiterInner {
                tokens: max_tokens, // Start with full bucket
                max_tokens,
                last_refill: Instant::now(),
            }),
            rate_bps,
        })
    }

    /// Acquire tokens (bytes) from the bucket, waiting if necessary.
    ///
    /// This method will sleep if there aren't enough tokens available,
    /// then deduct the requested amount.
    pub async fn acquire(&self, bytes: u64) {
        let bytes = bytes as f64;

        loop {
            let sleep_duration = {
                let mut inner = self.inner.lock().await;
                self.refill(&mut inner);

                if inner.tokens >= bytes {
                    inner.tokens -= bytes;
                    return;
                }

                // Calculate how long to wait for enough tokens
                let needed = bytes - inner.tokens;
                let seconds = needed / self.rate_bps as f64;
                Duration::from_secs_f64(seconds)
            };

            // Sleep outside the lock
            tokio::time::sleep(sleep_duration).await;
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&self, inner: &mut RateLimiterInner) {
        let now = Instant::now();
        let elapsed = now.duration_since(inner.last_refill);
        let new_tokens = elapsed.as_secs_f64() * self.rate_bps as f64;

        inner.tokens = (inner.tokens + new_tokens).min(inner.max_tokens);
        inner.last_refill = now;
    }
}

/// Optional rate limiter wrapper for cleaner API.
///
/// When `None`, no rate limiting is applied.
#[derive(Clone)]
pub struct Throttle {
    limiter: Option<Arc<RateLimiter>>,
}

impl Throttle {
    /// Create a throttle with rate limiting enabled.
    pub fn limited(rate_bps: u64) -> Self {
        Self {
            limiter: Some(RateLimiter::new(rate_bps)),
        }
    }

    /// Create a throttle with no rate limiting.
    pub fn unlimited() -> Self {
        Self { limiter: None }
    }

    /// Create a throttle from an optional rate in bytes per second.
    pub fn from_bps(rate_bps: Option<u64>) -> Self {
        match rate_bps {
            Some(rate) if rate > 0 => Self::limited(rate),
            _ => Self::unlimited(),
        }
    }

    /// Acquire tokens if rate limiting is enabled.
    pub async fn acquire(&self, bytes: u64) {
        if let Some(ref limiter) = self.limiter {
            limiter.acquire(bytes).await;
        }
    }

    /// Check if rate limiting is enabled.
    pub fn is_limited(&self) -> bool {
        self.limiter.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_throttle_unlimited() {
        let throttle = Throttle::unlimited();
        assert!(!throttle.is_limited());

        // Should return immediately
        throttle.acquire(1_000_000).await;
    }

    #[tokio::test]
    async fn test_throttle_from_bps() {
        let throttle = Throttle::from_bps(Some(1024));
        assert!(throttle.is_limited());

        let throttle = Throttle::from_bps(Some(0));
        assert!(!throttle.is_limited());

        let throttle = Throttle::from_bps(None);
        assert!(!throttle.is_limited());
    }

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        // 1000 bytes per second
        let limiter = RateLimiter::new(1000);

        // First acquire should be instant (bucket starts full)
        let start = Instant::now();
        limiter.acquire(500).await;
        assert!(start.elapsed() < Duration::from_millis(50));

        // Second acquire of remaining tokens should also be fast
        limiter.acquire(500).await;
        assert!(start.elapsed() < Duration::from_millis(100));

        // Third acquire should need to wait
        let before_wait = Instant::now();
        limiter.acquire(500).await;
        // Should have waited ~500ms (500 bytes at 1000 bps)
        let waited = before_wait.elapsed();
        assert!(waited >= Duration::from_millis(400));
        assert!(waited < Duration::from_millis(700));
    }
}
