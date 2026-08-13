//! API request retry policies and backoff handlers.

use std::thread;
use std::time::Duration;

use crate::error::ApiError;

/// Policy configuration for API retries.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including initial try).
    pub max_attempts: usize,
    /// Initial delay before first retry.
    pub base: Duration,
    /// Backoff multiplier.
    pub factor: u32,
    /// Maximum cap for sleep duration.
    pub cap: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base: Duration::from_millis(250),
            factor: 2,
            cap: Duration::from_secs(2),
        }
    }
}

/// Returns `true` if `err` is a transient failure eligible for retry.
pub fn should_retry(err: &ApiError) -> bool {
    match err {
        ApiError::RateLimited(_) => true,
        ApiError::NetworkError(_) => true,
        ApiError::NotFound(_) | ApiError::ParseError(_) | ApiError::InvalidIdentifier(_) => false,
    }
}

/// Trait to allow mock/instant sleepers in unit tests.
pub trait Sleeper: Send + Sync {
    /// Sleep for `duration`.
    fn sleep(&self, duration: Duration);
}

/// Real wall-clock sleeper using `std::thread::sleep`.
pub struct StdSleeper;

impl Sleeper for StdSleeper {
    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
    }
}

/// Instant sleeper for testing retries without delay.
#[cfg(test)]
pub struct InstantSleeper;

#[cfg(test)]
impl Sleeper for InstantSleeper {
    fn sleep(&self, _duration: Duration) {}
}

/// Execute closure `f` with the given retry `policy` and `sleeper`.
pub fn with_retry_sleeper<T, F>(
    policy: &RetryPolicy,
    sleeper: &dyn Sleeper,
    mut f: F,
) -> Result<T, ApiError>
where
    F: FnMut() -> Result<T, ApiError>,
{
    let mut attempt = 1;
    let mut delay = policy.base;

    loop {
        match f() {
            Ok(val) => return Ok(val),
            Err(err) => {
                if attempt >= policy.max_attempts || !should_retry(&err) {
                    return Err(err);
                }
                sleeper.sleep(delay);
                attempt += 1;
                delay = (delay * policy.factor).min(policy.cap);
            }
        }
    }
}

/// Execute closure `f` with default retry policy and wall-clock sleeper.
pub fn with_retry<T, F>(f: F) -> Result<T, ApiError>
where
    F: FnMut() -> Result<T, ApiError>,
{
    let policy = RetryPolicy::default();
    with_retry_sleeper(&policy, &StdSleeper, f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_should_retry_classifier() {
        assert!(should_retry(&ApiError::RateLimited("429".into())));
        assert!(should_retry(&ApiError::NetworkError("500".into())));
        assert!(!should_retry(&ApiError::NotFound("404".into())));
        assert!(!should_retry(&ApiError::ParseError("bad json".into())));
        assert!(!should_retry(&ApiError::InvalidIdentifier(
            "invalid".into()
        )));
    }

    #[test]
    fn test_retry_succeeds_after_failures() {
        let attempts = AtomicUsize::new(0);
        let policy = RetryPolicy::default();
        let sleeper = InstantSleeper;

        let res = with_retry_sleeper(&policy, &sleeper, || {
            let current = attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if current < 3 {
                Err(ApiError::RateLimited("429".into()))
            } else {
                Ok("success")
            }
        });

        assert_eq!(res.unwrap(), "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_non_retryable_error_fails_immediately() {
        let attempts = AtomicUsize::new(0);
        let policy = RetryPolicy::default();
        let sleeper = InstantSleeper;

        let res: Result<(), ApiError> = with_retry_sleeper(&policy, &sleeper, || {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(ApiError::NotFound("404".into()))
        });

        assert!(res.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}
