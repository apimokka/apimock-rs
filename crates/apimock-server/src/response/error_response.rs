use hyper::{HeaderMap, StatusCode};

use super::status_code_response::{status_code_response, status_code_response_with_message};
use crate::types::BoxBody;

/// error response on http BAD_REQUEST (400)
///
/// # Uncalled today (RFC 079 M-04d, audit F-09) — re-verified, still
/// # true
///
/// A prior handoff expected RFC 068 (tranche 1, body-size limits) to
/// give this its first caller; it didn't — RFC 068 added the separate
/// `payload_too_large_response` (413) for its own overflow case instead
/// of reusing this one. Re-checked for this tranche:
/// `grep -rn bad_request_response crates/ --include='*.rs'` still
/// returns only this definition. F-09 still wants a caller for it, so
/// it stays — a dead function about to be used is correct to keep; the
/// note is what says *why* it's unused, rather than leaving a reader to
/// wonder or assume it's reachable.
///
/// A plausible caller exists and was noticed while checking this
/// (deliberately **not** implemented here — RFC 079's own scope is "no
/// behaviour change anywhere", and wiring this up would be one):
/// `parsed_request.rs`'s `parsed_request_from` currently answers a
/// malformed JSON body claiming `Content-Type: application/json` with
/// `ParsedRequestError::Other`, which `server.rs` maps to a bare 500 —
/// a client mistake reported as a server failure. `bad_request_response`
/// is the obvious fix, but it's a real behaviour change (500 → 400 for
/// that one case) that belongs to whichever RFC picks it up next, not
/// to this hygiene pass.
pub fn bad_request_response(
    message: &str,
    request_headers: &HeaderMap,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    status_code_response_with_message(
        &StatusCode::BAD_REQUEST,
        message,
        None,
        request_headers,
        cors_allow_credentials_origins,
    )
}

/// error response on http PAYLOAD_TOO_LARGE (413) — RFC 068 S-02: a
/// request body over the configured limit, refused before it is
/// buffered.
pub fn payload_too_large_response(
    message: &str,
    request_headers: &HeaderMap,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    status_code_response_with_message(
        &StatusCode::PAYLOAD_TOO_LARGE,
        message,
        None,
        request_headers,
        cors_allow_credentials_origins,
    )
}

/// error response on http NOT_FOUND (404)
pub fn not_found_response(
    request_headers: &HeaderMap,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    status_code_response(
        &StatusCode::NOT_FOUND,
        None,
        request_headers,
        cors_allow_credentials_origins,
    )
}

/// error response on http INTERNAL_SERVER_ERROR (500)
pub fn internal_server_error_response(
    message: &str,
    request_headers: &HeaderMap,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    status_code_response_with_message(
        &StatusCode::INTERNAL_SERVER_ERROR,
        message,
        None,
        request_headers,
        cors_allow_credentials_origins,
    )
}
