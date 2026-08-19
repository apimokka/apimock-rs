//! Server-side helpers for `apimock_routing::ParsedRequest`.
//!
//! # Why this file exists after the 5.0 split
//!
//! `ParsedRequest` (the data) now lives in `apimock-routing` so the
//! matcher crate can depend on it without pulling in hyper/body I/O.
//! The two operations that *touch* HTTP — building a `ParsedRequest`
//! from an incoming hyper request, and logging one to stdout — are
//! server-layer activities, so they stay here as free functions.

use apimock_config::config::log_config::verbose_config::VerboseConfig;
use apimock_routing::{ParsedRequest, util::http::normalize_url_path};
use console::style;
use http_body_util::BodyExt;
use hyper::header::ORIGIN;
use hyper::{Version, body::Incoming};
use serde_json::{Value, to_string_pretty};

use std::time::{SystemTime, UNIX_EPOCH};

use crate::http_util::content_type_is_application_json;
use crate::trace::{REDACTED_HEADER_VALUE, TraceConfig};

/// Consume an incoming hyper request into a `ParsedRequest` the matcher
/// can use.
///
/// # Why a non-JSON body is logged but not rejected
///
/// Some rule sets key only on URL path or headers and don't inspect the
/// body at all. Failing the whole request because an operator sent a
/// form-encoded payload would be more aggressive than needed; we log a
/// warning and continue so the URL-path-only rules still apply. Only
/// *claimed* JSON (`Content-Type: application/json`) that fails to
/// parse becomes a hard `Err` — that is a real client bug.
pub async fn parsed_request_from(
    request: hyper::Request<Incoming>,
) -> Result<ParsedRequest, String> {
    let (component_parts, body) = request.into_parts();

    let body_bytes = match body.boxed().collect().await {
        Ok(x) => Some(x.to_bytes()),
        Err(err) => {
            log::warn!("failed to collect request incoming body: {}", err);
            None
        }
    };

    let has_body = body_bytes.as_ref().map(|b| !b.is_empty()).unwrap_or(false);

    let body_json = if has_body {
        let bytes = body_bytes
            .as_ref()
            .expect("body_bytes presence checked by has_body");
        let raw_body_json = serde_json::from_slice::<Option<Value>>(bytes);

        match (
            content_type_is_application_json(&component_parts.headers),
            raw_body_json,
        ) {
            // declared application/json but body didn't parse → hard error
            (Some(true), Err(err)) => {
                return Err(format!(
                    "failed to get json value from request body: {}",
                    err
                ));
            }
            (Some(true), Ok(v)) => v,
            (_, Ok(v)) => {
                if matches!(
                    content_type_is_application_json(&component_parts.headers),
                    Some(false)
                ) {
                    log::warn!("request has body but its content-type is not application/json");
                } else if content_type_is_application_json(&component_parts.headers).is_none() {
                    log::warn!("request has body but doesn't have content-type");
                }
                v
            }
            (_, Err(_)) => None,
        }
    } else {
        None
    };

    let url_path = normalize_url_path(component_parts.uri.path(), None);

    // RFC 050: propagate what's already been measured above (`has_body`,
    // `body_bytes`'s length) rather than computing anything new.
    let body_len = has_body.then(|| {
        body_bytes
            .as_ref()
            .expect("body_bytes presence checked by has_body")
            .len()
    });

    Ok(ParsedRequest::new(url_path, component_parts).with_body(body_json, body_len))
}

/// Emit a single log line describing the request.
///
/// Kept as a public, two-argument function so any caller outside this
/// workspace keeps compiling unchanged (RFC 051 review, R-09 applies to
/// function signatures on `pub fn`s, not only to struct fields). It
/// still redacts — `TraceConfig::default()` carries the default
/// denylist — so an out-of-tree caller gets the security fix for free
/// rather than needing to opt in. In-workspace, use
/// [`capture_in_log_with_trace_config`], which shares the server's own
/// `TraceConfig` instead of a fresh default one.
pub fn capture_in_log(request: &ParsedRequest, verbose: VerboseConfig) {
    capture_in_log_with_trace_config(request, verbose, &TraceConfig::default())
}

/// [`capture_in_log`], but redacting per `trace_config` instead of a
/// fresh `TraceConfig::default()` — the one place this and the trace
/// channel (`crate::trace::redact_headers`) can never honour two
/// different denylists, because both read the same `TraceConfig`
/// instance the running server built (RFC 051).
pub(crate) fn capture_in_log_with_trace_config(
    request: &ParsedRequest,
    verbose: VerboseConfig,
    trace_config: &TraceConfig,
) {
    log::info!("{}", render_request_log(request, verbose, trace_config));
}

/// Build the line `capture_in_log` emits, without emitting it — split out
/// so the rendered text (what actually reaches a terminal) can be
/// asserted on directly in tests, rather than intercepting the log
/// backend.
fn render_request_log(
    request: &ParsedRequest,
    verbose: VerboseConfig,
    trace_config: &TraceConfig,
) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;
    let timestamp = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

    let version = match request.component_parts.version {
        Version::HTTP_3 => "HTTP/3",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_11 => "HTTP/1.1",
        _ => "HTTP/1.0 or earlier, or HTTP/4 or later",
    };

    let origin = request
        .component_parts
        .headers
        .get(ORIGIN)
        .and_then(|v| v.to_str().ok());

    let mut printed = format!(
        "<- {}\n   [{}]",
        style(request.url_path.as_str()).yellow(),
        request.component_parts.method,
    );
    if let Some(origin) = origin {
        printed.push_str(&format!(" [ORIGIN {}]", origin));
    }
    printed.push_str(&format!(
        " [{}] request received (at {} UTC)",
        version, timestamp
    ));

    if verbose.header || verbose.body {
        printed.push('\n');
    }
    if verbose.header {
        let headers = request
            .component_parts
            .headers
            .iter()
            .map(|(name, value)| {
                let rendered = if trace_config.is_header_redacted(name.as_str()) {
                    REDACTED_HEADER_VALUE
                } else {
                    value.to_str().unwrap_or("<non-utf8>")
                };
                format!("\n{}: {}", name, rendered)
            })
            .collect::<String>();
        printed.push_str(&format!(
            "   [request.headers]{}\n",
            style(headers).magenta()
        ));
    }

    let mut is_verbose_body = false;
    if verbose.body {
        let query = request.component_parts.uri.query();
        if let Some(query) = query {
            printed.push_str(&format!("   [request.query] {}\n", query));
            is_verbose_body = true;
        }

        if let Some(request_body_json_value) = &request.body_json {
            printed.push_str("   [request.body.json]\n");

            let body_str = match to_string_pretty(request_body_json_value) {
                Ok(x) => x,
                Err(err) => {
                    log::warn!(
                        "failed to prettify JSON: {} ({})",
                        request_body_json_value,
                        err
                    );
                    request_body_json_value.to_string()
                }
            };
            let styled_body_str = body_str
                .split("\n")
                .map(|s| style(s).green().to_string())
                .collect::<Vec<String>>()
                .join("\n");
            printed.push_str(styled_body_str.as_str());

            is_verbose_body = true;
        }
    }
    if verbose.header || is_verbose_body {
        printed.push('\n');
    }

    printed
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::HeaderRedactionMode;

    /// Build a minimal `ParsedRequest` carrying the given headers.
    fn request_with_headers(headers: &[(&str, &str)]) -> ParsedRequest {
        let mut builder = hyper::Request::builder().method("GET").uri("/");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let req = builder.body(()).unwrap();
        let (component_parts, _) = req.into_parts();
        ParsedRequest::new("/".to_owned(), component_parts)
    }

    const VERBOSE_HEADERS_ONLY: VerboseConfig = VerboseConfig::new(true, false);

    /// RFC 051 evidence requirement: with `log.verbose.header` on and no
    /// other configuration (`TraceConfig::default()`), the rendered log
    /// line contains none of the credential values, but a non-credential
    /// header's value still appears.
    #[test]
    fn verbose_header_redacts_credential_headers_by_default() {
        let request = request_with_headers(&[
            ("authorization", "Bearer secret-token"),
            ("cookie", "session=abc123"),
            ("x-api-key", "sk-live-very-secret"),
            ("content-type", "application/json"),
        ]);
        let rendered = render_request_log(&request, VERBOSE_HEADERS_ONLY, &TraceConfig::default());

        assert!(
            !rendered.contains("Bearer secret-token"),
            "rendered was: {rendered}"
        );
        assert!(
            !rendered.contains("session=abc123"),
            "rendered was: {rendered}"
        );
        assert!(
            !rendered.contains("sk-live-very-secret"),
            "rendered was: {rendered}"
        );
        assert!(
            rendered.contains("application/json"),
            "a non-credential header must survive: {rendered}"
        );
    }

    /// Redacted headers stay present, marked with the placeholder — not
    /// silently dropped from the rendered line (RFC 040 Goal 4, reused
    /// here per RFC 051).
    #[test]
    fn verbose_header_redacted_headers_are_marked_not_omitted() {
        let request = request_with_headers(&[("authorization", "Bearer secret-token")]);
        let rendered = render_request_log(&request, VERBOSE_HEADERS_ONLY, &TraceConfig::default());

        assert!(rendered.contains("authorization"), "rendered: {rendered}");
        assert!(
            rendered.contains(REDACTED_HEADER_VALUE),
            "rendered: {rendered}"
        );
    }

    /// A denylist compared case-sensitively is a leak that passes a naive
    /// test — proven here with non-lowercase spellings.
    #[test]
    fn verbose_header_redaction_is_case_insensitive() {
        let request = request_with_headers(&[
            ("Authorization", "Bearer secret-token"),
            ("COOKIE", "session=abc123"),
        ]);
        let rendered = render_request_log(&request, VERBOSE_HEADERS_ONLY, &TraceConfig::default());

        assert!(
            !rendered.contains("Bearer secret-token"),
            "rendered: {rendered}"
        );
        assert!(!rendered.contains("session=abc123"), "rendered: {rendered}");
        assert!(
            rendered.contains(REDACTED_HEADER_VALUE),
            "rendered: {rendered}"
        );
    }

    /// The policy is shared, not copied: an allowlist configured on the
    /// same `TraceConfig` passed to the trace channel also governs verbose
    /// logging, with no separate list to keep in sync.
    #[test]
    fn verbose_header_honours_the_same_trace_config_instance() {
        let config = TraceConfig {
            header_redaction: HeaderRedactionMode::Allowlist,
            header_allowlist: vec!["content-type".into()],
            ..Default::default()
        };
        let request = request_with_headers(&[
            ("content-type", "application/json"),
            ("x-request-id", "not-a-credential"),
        ]);
        let rendered = render_request_log(&request, VERBOSE_HEADERS_ONLY, &config);

        assert!(
            rendered.contains("application/json"),
            "allowlisted header must survive: {rendered}"
        );
        assert!(
            !rendered.contains("not-a-credential"),
            "unlisted header must be redacted under the shared allowlist: {rendered}"
        );
    }

    /// `capture_in_log`'s public, two-argument signature must keep
    /// compiling for any out-of-tree caller (RFC 051 review, § 2) — a
    /// regression guard on the API surface itself. Its body has no
    /// return value to assert redaction on directly; that behaviour is
    /// `render_request_log`'s, exercised by the tests above with the
    /// same `TraceConfig::default()` this delegates to.
    #[test]
    fn capture_in_log_public_two_argument_form_still_compiles_and_runs() {
        let request = request_with_headers(&[("authorization", "Bearer secret-token")]);
        capture_in_log(&request, VERBOSE_HEADERS_ONLY);
    }
}
