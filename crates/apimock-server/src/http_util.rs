//! HTTP utilities used only by the server crate.
//!
//! `normalize_url_path` previously lived alongside these but moved to
//! `apimock-routing::util::http` in 5.0 — the matcher needs it; these
//! two helpers are HTTP-layer only.

use hyper::{
    HeaderMap,
    header::{CONTENT_TYPE, HeaderValue},
};
use tokio::time;

use std::time::Duration;

/// Inspect `Content-Type` to decide whether a request body should be
/// parsed as JSON.
///
/// Returns:
/// - `Some(true)`  — header is present and starts with `application/json`
///   (supports `application/json; charset=utf-8` and similar).
/// - `Some(false)` — header is present but is something else.
/// - `None`        — header is absent, so we can't tell.
///
/// The three-valued return is deliberate: callers treat "absent" and
/// "present-but-wrong" differently (the first is a common shortcut, the
/// second is a likely client bug).
pub fn content_type_is_application_json(headers: &HeaderMap<HeaderValue>) -> Option<bool> {
    let content_type = headers.get(CONTENT_TYPE)?;

    let Ok(content_type) = content_type.to_str() else {
        return Some(false);
    };

    Some(
        content_type
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("application/json"),
    )
}

/// Sleep `milliseconds` ms on the async runtime.
///
/// Used by `respond.delay_response_milliseconds` to simulate slow
/// backends when a mock needs to exercise client timeout behaviour.
pub async fn delay_response(milliseconds: u32) {
    time::sleep(Duration::from_millis(milliseconds.into())).await
}
