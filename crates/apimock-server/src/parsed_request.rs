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

    Ok(ParsedRequest {
        url_path,
        component_parts,
        body_json,
    })
}

/// Emit a single log line describing the request.
pub fn capture_in_log(request: &ParsedRequest, verbose: VerboseConfig) {
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
            .map(|(name, value)| format!("\n{}: {}", name, value.to_str().unwrap_or("<non-utf8>")))
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

    log::info!("{}", printed);
}
