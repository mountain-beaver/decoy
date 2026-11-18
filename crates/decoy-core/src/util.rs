//! Small shared helpers: clocks and id generation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn since_epoch() -> std::time::Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

pub fn now_ms() -> i64 {
    since_epoch().as_millis() as i64
}

pub fn now_secs() -> u64 {
    since_epoch().as_secs()
}

/// Unique id: hex of time in nanoseconds plus a process-wide counter.
pub fn new_id() -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}{:04x}", since_epoch().as_nanos(), n & 0xffff)
}
