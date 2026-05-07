//! Request-rate limiter for the Crunchyroll HTTP client.
//!
//! Token bucket where each token = one outbound CR-API request. The
//! [`CrunchyrollClient`] holds a chain of these (per-user + global) and
//! [`acquire`]s a token before every request. Segment-CDN traffic uses a
//! separate `reqwest::Client` outside this code path and is intentionally
//! NOT throttled — segment fetches are CDN-side, not API-plane.
//!
//! Modeled after the byte-rate `download::throttle::RateLimiter`. Same
//! token-bucket pattern, just counts requests instead of bytes.
//!
//! [`CrunchyrollClient`]: super::CrunchyrollClient
//! [`acquire`]: RequestRateLimiter::acquire

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Token-bucket rate limiter for outbound HTTP requests.
///
/// Refills `rps` tokens per second up to `burst`. `acquire()` consumes one
/// token, sleeping until one is available if the bucket is dry.
#[derive(Debug)]
pub struct RequestRateLimiter {
    inner: Mutex<Inner>,
    rps: f64,
}

#[derive(Debug)]
struct Inner {
    tokens: f64,
    max_tokens: f64,
    last_refill: Instant,
}

impl RequestRateLimiter {
    /// Build a limiter that sustains `rps` requests/second with up to `burst`
    /// bunched together. `burst` is clamped to be at least 1.
    pub fn new(rps: u32, burst: u32) -> Arc<Self> {
        let burst = burst.max(1) as f64;
        Arc::new(Self {
            inner: Mutex::new(Inner {
                tokens: burst,
                max_tokens: burst,
                last_refill: Instant::now(),
            }),
            rps: rps.max(1) as f64,
        })
    }

    /// Consume one token, awaiting refill if the bucket is dry.
    pub async fn acquire(&self) {
        loop {
            let wait = {
                let mut inner = self.inner.lock().await;
                self.refill(&mut inner);

                if inner.tokens >= 1.0 {
                    inner.tokens -= 1.0;
                    return;
                }

                let needed = 1.0 - inner.tokens;
                Duration::from_secs_f64(needed / self.rps)
            };

            tokio::time::sleep(wait).await;
        }
    }

    fn refill(&self, inner: &mut Inner) {
        let now = Instant::now();
        let elapsed = now.duration_since(inner.last_refill).as_secs_f64();
        inner.tokens = (inner.tokens + elapsed * self.rps).min(inner.max_tokens);
        inner.last_refill = now;
    }
}

/// Acquires one token from each limiter in `chain`, in order. The chain
/// composes per-user + global caps so a request only proceeds when both
/// allow it.
pub async fn acquire_all(chain: &[Arc<RequestRateLimiter>]) {
    for limiter in chain {
        limiter.acquire().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn initial_burst_is_immediate() {
        let limiter = RequestRateLimiter::new(2, 5);
        let start = Instant::now();
        for _ in 0..5 {
            limiter.acquire().await;
        }
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn steady_state_throttles() {
        // 5 RPS, burst 5: 10 acquires should take ~1s (first 5 instant, next 5 paced).
        let limiter = RequestRateLimiter::new(5, 5);
        let start = Instant::now();
        for _ in 0..10 {
            limiter.acquire().await;
        }
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(900), "elapsed {:?}", elapsed);
        assert!(elapsed < Duration::from_millis(1500), "elapsed {:?}", elapsed);
    }

    #[tokio::test]
    async fn concurrent_acquires_serialize() {
        // 10 RPS, burst 1: 5 concurrent acquires take ~400ms (first instant, then 4 × 100ms).
        let limiter = RequestRateLimiter::new(10, 1);
        let start = Instant::now();
        let mut handles = Vec::new();
        for _ in 0..5 {
            let l = limiter.clone();
            handles.push(tokio::spawn(async move { l.acquire().await }));
        }
        for h in handles {
            h.await.unwrap();
        }
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(350), "elapsed {:?}", elapsed);
    }

    #[tokio::test]
    async fn chain_acquires_each_in_order() {
        let a = RequestRateLimiter::new(10, 1);
        let b = RequestRateLimiter::new(10, 1);
        let chain = vec![a, b];
        let start = Instant::now();
        // 3 calls × 2 limiters at 10 rps each, burst 1: first instant, next 2 ≥ 100ms each.
        for _ in 0..3 {
            acquire_all(&chain).await;
        }
        assert!(start.elapsed() >= Duration::from_millis(180));
    }
}
