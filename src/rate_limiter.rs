use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Simple rate limiter implementation to prevent API abuse and DoS attacks
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum requests per time window
    max_requests: u32,
    /// Time window duration
    window_duration: Duration,
    /// Track request counts per identifier (URL or endpoint)
    requests: Mutex<HashMap<String, (u32, Instant)>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_requests: u32, window_duration: Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            requests: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a request is allowed for the given identifier
    /// Returns true if allowed, false if rate limited
    pub async fn is_allowed(&self, identifier: &str) -> Result<bool> {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        // Clean up expired entries periodically
        if requests.len() > 1000 {
            requests.retain(|_, (_, timestamp)| {
                now.duration_since(*timestamp) < self.window_duration * 2
            });
        }

        match requests.get_mut(identifier) {
            Some((count, timestamp)) => {
                // Check if we're still in the same time window
                if now.duration_since(*timestamp) < self.window_duration {
                    // Still in the same window
                    if *count >= self.max_requests {
                        // Rate limited
                        Ok(false)
                    } else {
                        // Increment counter and allow
                        *count += 1;
                        Ok(true)
                    }
                } else {
                    // New time window, reset counter
                    *count = 1;
                    *timestamp = now;
                    Ok(true)
                }
            }
            None => {
                // First request for this identifier
                requests.insert(identifier.to_string(), (1, now));
                Ok(true)
            }
        }
    }

    /// Wait until a request would be allowed (backoff strategy)
    pub async fn wait_for_slot(&self, identifier: &str) -> Result<()> {
        loop {
            if self.is_allowed(identifier).await? {
                break;
            }

            // Wait a short time before retrying
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }

    /// Get remaining requests for an identifier
    pub async fn remaining_requests(&self, identifier: &str) -> Result<u32> {
        let requests = self.requests.lock().await;
        let now = Instant::now();

        match requests.get(identifier) {
            Some((count, timestamp)) => {
                if now.duration_since(*timestamp) < self.window_duration {
                    Ok(self.max_requests.saturating_sub(*count))
                } else {
                    // New window, all requests available
                    Ok(self.max_requests)
                }
            }
            None => Ok(self.max_requests),
        }
    }

    /// Get time until next request is allowed
    pub async fn time_until_reset(&self, identifier: &str) -> Result<Option<Duration>> {
        let requests = self.requests.lock().await;
        let now = Instant::now();

        match requests.get(identifier) {
            Some((count, timestamp)) => {
                if *count >= self.max_requests {
                    let elapsed = now.duration_since(*timestamp);
                    if elapsed < self.window_duration {
                        Ok(Some(self.window_duration - elapsed))
                    } else {
                        Ok(None) // Can request now
                    }
                } else {
                    Ok(None) // Can request now
                }
            }
            None => Ok(None), // Can request now
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(60, Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));

        assert!(limiter.is_allowed("test").await.unwrap());
        assert!(limiter.is_allowed("test").await.unwrap());
        assert!(limiter.is_allowed("test").await.unwrap());
        assert!(!limiter.is_allowed("test").await.unwrap()); // Should be rate limited
    }

    #[tokio::test]
    async fn test_rate_limiter_resets_after_window() {
        let limiter = RateLimiter::new(1, Duration::from_millis(100));

        assert!(limiter.is_allowed("test").await.unwrap());
        assert!(!limiter.is_allowed("test").await.unwrap()); // Rate limited

        sleep(Duration::from_millis(150)).await; // Wait for window to reset

        assert!(limiter.is_allowed("test").await.unwrap()); // Should work again
    }

    #[tokio::test]
    async fn test_rate_limiter_different_identifiers() {
        let limiter = RateLimiter::new(1, Duration::from_secs(1));

        assert!(limiter.is_allowed("test1").await.unwrap());
        assert!(limiter.is_allowed("test2").await.unwrap()); // Different identifier
        assert!(!limiter.is_allowed("test1").await.unwrap()); // First identifier rate limited
        assert!(!limiter.is_allowed("test2").await.unwrap()); // Second identifier rate limited
    }
}
