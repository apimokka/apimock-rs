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

/// Normalise a URL path so matching doesn't have to worry about slash
/// placement.
///
/// # Why we force exactly one leading slash and no trailing slash
///
/// Rule-set authors write paths both ways — `/api/v1`, `api/v1`,
/// `/api/v1/`. Routing matchers also get requests with and without
/// trailing slashes. Picking one canonical form here means every
/// comparator downstream only has to handle the normalized shape,
/// which eliminates a class of "why isn't my rule matching?" bugs.
pub fn normalize_url_path(url_path: &str, url_path_prefix: Option<&str>) -> String {
    let url_path_prefix = match url_path_prefix {
        Some(prefix) if !prefix.is_empty() => prefix.strip_suffix('/').unwrap_or(prefix),
        _ => "",
    };

    let url_path = url_path.strip_prefix('/').unwrap_or(url_path);

    let merged = format!("{}/{}", url_path_prefix, url_path);

    // Apply the two strips in the same order as the original implementation.
    // Using intermediate `&str` bindings (rather than chaining) keeps the
    // behaviour identical: each `strip_*` is independent of whether the
    // previous one matched.
    let trimmed = merged.strip_suffix('/').unwrap_or(merged.as_str());
    let trimmed = trimmed.strip_prefix('/').unwrap_or(trimmed);

    // Defence in depth against a raw `..` segment (RFC 063): this is
    // not the confinement fix itself — the serve-time canonicalise
    // check is — but it closes the ordinary case before a path even
    // reaches file resolution. A bare token removal rather than a full
    // dot-segment resolution (RFC 3986 §5.2): `..` never counts as
    // legitimate in a rule's own `url_path` either, so dropping it
    // outright, without collapsing whatever segment preceded it, is
    // sufficient here and simpler than a real resolver.
    let stripped: Vec<&str> = trimmed.split('/').filter(|seg| *seg != "..").collect();
    format!("/{}", stripped.join("/"))
}

/// Sleep `milliseconds` ms on the async runtime.
///
/// Used by `respond.delay_response_milliseconds` to simulate slow
/// backends when a mock needs to exercise client timeout behaviour.
pub async fn delay_response(milliseconds: u32) {
    time::sleep(Duration::from_millis(milliseconds.into())).await
}
