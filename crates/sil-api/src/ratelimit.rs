use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

static LAST_API_CALL: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(|| Mutex::new(None));

/// Enforce a minimal rate-limiting delay (250ms) between external HTTP API requests.
pub fn enforce_api_ratelimit() {
    if let Ok(mut guard) = LAST_API_CALL.lock() {
        if let Some(last) = *guard {
            let elapsed = last.elapsed();
            let min_delay = Duration::from_millis(250);
            if elapsed < min_delay {
                std::thread::sleep(min_delay - elapsed);
            }
        }
        *guard = Some(Instant::now());
    }
}
