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

#[cfg(test)]
mod tests {
    //! RFC 076: this function still parses and reserialises (used by
    //! inline `respond.json` and by `.json5` `file_path`s — converting
    //! JSON5 is the point for both, so neither is served raw). What
    //! must not regress is *key order*: the workspace-wide
    //! `serde_json/preserve_order` feature is what makes that hold —
    //! without it, this test fails by alphabetising `zebra`/`apple`.
    use hyper::HeaderMap;

    use super::json_response;

    #[tokio::test]
    async fn key_order_survives_the_parse_and_reserialise_round_trip() {
        let response = json_response(
            r#"{"zebra":1,"apple":2}"#,
            None,
            None,
            &HeaderMap::new(),
            None,
            &[],
        )
        .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(
            body.as_ref(),
            br#"{"zebra":1,"apple":2}"#,
            "key order must survive the round trip, not be alphabetised"
        );
    }
}
