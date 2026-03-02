//! Debug session instrumentation - appends NDJSON to .cursor/debug-*.log
#![allow(dead_code)]
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_PATH: &str = "/Users/matt.chow/workspace/pmux/.cursor/debug-87bd77.log";

/// Session debug log for current debug run
const SESSION_LOG_PATH: &str = "/Users/matt.chow/workspace/pmux/.cursor/debug-6882f7.log";

static RENDER_COUNT: AtomicU64 = AtomicU64::new(0);
static RENDER_MAX_DURATION_MS: AtomicU64 = AtomicU64::new(0);
static RENDER_LAST_SAMPLE_MS: AtomicU64 = AtomicU64::new(0);

/// Call at end of AppRoot::render with duration. Logs once per second: rate + max_duration_ms.
pub fn dbg_render_sample(duration_ms: u64) {
    RENDER_COUNT.fetch_add(1, Ordering::Relaxed);
    let mut max = RENDER_MAX_DURATION_MS.load(Ordering::Relaxed);
    while duration_ms > max {
        match RENDER_MAX_DURATION_MS.compare_exchange_weak(
            max,
            duration_ms,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(m) => max = m,
        }
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let last = RENDER_LAST_SAMPLE_MS.load(Ordering::Relaxed);
    if now_ms.saturating_sub(last) >= 1000 && RENDER_LAST_SAMPLE_MS.compare_exchange(last, now_ms, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        let count = RENDER_COUNT.swap(0, Ordering::Relaxed);
        let max_d = RENDER_MAX_DURATION_MS.swap(0, Ordering::Relaxed);
        dbg_log(
            "app_root.rs:render",
            "AppRoot render sample (per sec)",
            &serde_json::json!({
                "renders_per_sec": count,
                "max_duration_ms": max_d,
                "over_16ms": max_d > 16,
                "over_8ms": max_d > 8,
            }),
            "H_render_bottleneck",
        );
    }
}

/// Append NDJSON line to session debug log (for bug debugging runs).
pub fn dbg_session_log(location: &str, message: &str, data: &serde_json::Value, hypothesis_id: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let entry = serde_json::json!({
        "sessionId": "6882f7",
        "location": location,
        "message": message,
        "data": data,
        "hypothesisId": hypothesis_id,
        "timestamp": ts
    });
    if let Ok(s) = serde_json::to_string(&entry) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(SESSION_LOG_PATH) {
            let _ = writeln!(f, "{}", s);
        }
    }
}

pub fn dbg_log(location: &str, message: &str, data: &serde_json::Value, hypothesis_id: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let entry = serde_json::json!({
        "sessionId": "87bd77",
        "location": location,
        "message": message,
        "data": data,
        "hypothesisId": hypothesis_id,
        "timestamp": ts
    });
    if let Ok(s) = serde_json::to_string(&entry) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(LOG_PATH) {
            let _ = writeln!(f, "{}", s);
        }
    }
}
