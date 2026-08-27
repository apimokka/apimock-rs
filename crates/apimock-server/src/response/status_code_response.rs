use hyper::{HeaderMap, StatusCode};

use std::collections::HashMap;

use crate::{response_handler::ResponseHandler, types::BoxBody};

/// custom status code response (body is empty)
///
/// `headers` are applied last, after the status is set, so an explicit
/// header always wins over anything `ResponseHandler` would otherwise
/// infer (RFC 045: an explicitly configured header wins over an
/// inferred default).
pub fn status_code_response(
    status_code: &StatusCode,
    headers: Option<&HashMap<String, Option<String>>>,
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    ResponseHandler::default()
        .with_status(status_code)
        .with_custom_headers(headers)
        .into_response(request_headers)
}

/// custom status code response with message in body
///
/// See [`status_code_response`] for why `headers` is applied after
/// `with_text` — an explicit `content-type` in `headers` must win over
/// the `text/plain` default `with_text` sets.
pub fn status_code_response_with_message(
    status_code: &StatusCode,
    message: &str,
    headers: Option<&HashMap<String, Option<String>>>,
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    ResponseHandler::default()
        .with_status(status_code)
        .with_text(message, None)
        .with_custom_headers(headers)
        .into_response(request_headers)
}
