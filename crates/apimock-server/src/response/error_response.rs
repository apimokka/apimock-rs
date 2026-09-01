use hyper::{HeaderMap, StatusCode};

use super::status_code_response::{status_code_response, status_code_response_with_message};
use crate::types::BoxBody;

/// error response on http BAD_REQUEST (400)
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
