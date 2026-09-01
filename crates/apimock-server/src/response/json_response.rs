use std::collections::HashMap;

use hyper::{HeaderMap, StatusCode};
use serde_json::Value;

use crate::{
    response::error_response::internal_server_error_response, response_handler::ResponseHandler,
    types::BoxBody,
};

/// JSON response — used both for `respond.json` (RFC 065, `source` is
/// `None`) and for a `file_path` pointing at a `.json`/`.json5` file or
/// a CSV converted to JSON (`source` names the file, for the server
/// log only — see below).
///
/// Parses with the same JSON5 parser `Respond::validate` already
/// checked this content against at load time (`apimock_routing`'s own
/// `json5` dependency), so the `_ =>` branch below is unreachable for
/// `respond.json` and for a `.json`/`.json5` `file_path` in ordinary
/// operation — both are now validated before the server ever starts
/// (RFC 065 D3). It stays reachable for CSV→JSON conversion (not
/// content-validated at load, since the source is CSV, not JSON) and
/// as a defensive fallback if a file changes on disk after startup.
///
/// # Why `source` never reaches the response body (RFC 065 D4)
///
/// This used to build the client-facing message as `"{file_path}:
/// invalid json content"`, putting the server's own filesystem path in
/// front of every client that hit this branch. The path is still
/// useful — to whoever runs the server, not to whoever's making the
/// request — so it goes to the server log via `log::error!` instead;
/// the client gets a message that names the *problem*, not the
/// server's directory layout.
pub fn json_response(
    json_str: &str,
    status_code: Option<&StatusCode>,
    custom_headers: Option<&HashMap<String, Option<String>>>,
    request_headers: &HeaderMap,
    source: Option<&str>,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    match json5::from_str::<Value>(json_str) {
        Ok(content) => {
            let body = content.to_string();
            let mut response_handler = ResponseHandler::default().with_json_body(body.as_str());
            if let Some(status_code) = status_code {
                response_handler = response_handler.with_status(status_code);
            }
            response_handler
                .with_custom_headers(custom_headers)
                .into_response(request_headers, cors_allow_credentials_origins)
        }
        Err(err) => {
            log::error!(
                "invalid json content{}: {}",
                source.map(|s| format!(" ({})", s)).unwrap_or_default(),
                err
            );
            internal_server_error_response(
                "invalid json content",
                request_headers,
                cors_allow_credentials_origins,
            )
        }
    }
}
