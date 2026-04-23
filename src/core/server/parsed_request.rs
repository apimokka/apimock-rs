use console::style;
use http_body_util::BodyExt;
use hyper::header::ORIGIN;
use hyper::http::request::Parts;
use hyper::{body::Incoming, Version};
use serde_json::{to_string_pretty, Value};

use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::{
    config::log_config::verbose_config::VerboseConfig,
    util::http::{content_type_is_application_json, normalize_url_path},
};

/// Request metadata and body, decoded once per request.
///
/// # Why we eagerly collect the body
///
/// Routing decisions depend on body contents (rule-set `body.json`
/// conditions, middleware evaluation). Rather than re-collecting the
/// body each time a matcher asks for it, we consume the `Incoming`
/// stream once here and keep the parsed JSON around for the lifetime of
/// the request. This is appropriate for a mock server where payloads
/// are small; a production proxy would want streaming instead.
#[derive(Debug)]
pub struct ParsedRequest {
    pub url_path: String,
    pub component_parts: Parts,
    /// Parsed JSON body, if the request had one that parsed successfully.
    /// `None` here means either "no body" or "body present but not JSON";
    /// the two are indistinguishable at the matcher layer and we don't
    /// currently need to distinguish them.
    pub body_json: Option<Value>,
}

impl ParsedRequest {
    /// Consume an incoming hyper request into a parsed form.
    ///
    /// # Why a non-JSON body is logged but not rejected
    ///
    /// Some rule sets key only on URL path or headers and don't inspect
    /// the body at all. Failing the whole request because an operator
    /// sent a form-encoded payload would be more aggressive than needed;
    /// we log a warning and continue so the URL-path-only rules still
    /// apply. Only *claimed* JSON (`Content-Type: application/json`) that
    /// fails to parse becomes a hard `Err` — that is a real client bug.
    pub async fn from(request: hyper::Request<Incoming>) -> Result<Self, String> {
        let (component_parts, body) = request.into_parts();

        let body_bytes = match body.boxed().collect().await {
            Ok(x) => Some(x.to_bytes()),
            Err(err) => {
                log::warn!("failed to collect request incoming body: {}", err);
                None
            }
        };

        let has_body = body_bytes
            .as_ref()
            .map(|b| !b.is_empty())
            .unwrap_or(false);

        let body_json = if has_body {
            // Safe: `has_body` implies `body_bytes.is_some()`.
            let bytes = body_bytes.as_ref().expect("body_bytes presence checked by has_body");
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
                // declared application/json and body parsed → use it
                (Some(true), Ok(v)) => v,
                // body parsed as JSON even without the declaration → use it
                // (this is a common reality for lazy clients)
                (_, Ok(v)) => {
                    if matches!(content_type_is_application_json(&component_parts.headers), Some(false)) {
                        log::warn!(
                            "request has body but its content-type is not application/json"
                        );
                    } else if content_type_is_application_json(&component_parts.headers).is_none() {
                        log::warn!("request has body but doesn't have content-type");
                    }
                    v
                }
                // body present but not JSON and not claimed as JSON → ignore
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

    /// Emit the request to the log.
    ///
    /// # Why this is a method on `ParsedRequest` and not a logger plugin
    ///
    /// The verbose log contains pretty-printed JSON body, which requires
    /// an allocation. Doing it inside a dedicated method means we can
    /// short-circuit before paying that cost when verbose mode is off —
    /// the default for non-debug use — without the logger trait having
    /// to know anything about request shape.
    pub fn capture_in_log(&self, verbose: VerboseConfig) {
        // server log (timestamp)
        // `unwrap_or_default` here: if the system clock is before 1970
        // we fall back to 0 rather than panicking; this is pure log
        // cosmetics and should never take down the server.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let hours = (now / 3600) % 24;
        let minutes = (now / 60) % 60;
        let seconds = now % 60;
        let timestamp = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

        // request info (url path, origin etc.)
        let version = match self.component_parts.version {
            Version::HTTP_3 => "HTTP/3",
            Version::HTTP_2 => "HTTP/2",
            Version::HTTP_11 => "HTTP/1.1",
            _ => "HTTP/1.0 or earlier, or HTTP/4 or later",
        };

        let origin = self
            .component_parts
            .headers
            .get(ORIGIN)
            .and_then(|v| v.to_str().ok());

        // print
        // - server info and request info
        let mut printed = format!(
            "<- {}\n   [{}]",
            style(self.url_path.as_str()).yellow(),
            self.component_parts.method,
        );
        if let Some(origin) = origin {
            printed.push_str(&format!(" [ORIGIN {}]", origin));
        }
        printed.push_str(&format!(
            " [{}] request received (at {} UTC)",
            version, timestamp
        ));

        // - headers
        if verbose.header || verbose.body {
            printed.push_str("\n");
        }
        if verbose.header {
            // Header values that aren't valid UTF-8 are rare but legal.
            // We render them as `<non-utf8>` rather than panicking — a
            // log line is not worth taking the request down over.
            let headers = self
                .component_parts
                .headers
                .iter()
                .map(|(name, value)| {
                    format!("\n{}: {}", name, value.to_str().unwrap_or("<non-utf8>"))
                })
                .collect::<String>();
            printed.push_str(&format!(
                "   [request.headers]{}\n",
                style(headers).magenta()
            ));
        }

        // - body (url query, json params)
        let mut is_verbose_body = false;
        if verbose.body {
            let query = self.component_parts.uri.query();
            if let Some(query) = query {
                printed.push_str(&format!("   [request.query] {}\n", query));
                is_verbose_body = true;
            }

            if let Some(request_body_json_value) = &self.body_json {
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
            printed.push_str("\n");
        }

        log::info!("{}", printed);
    }
}
