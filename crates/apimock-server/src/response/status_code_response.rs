use hyper::{HeaderMap, StatusCode};

use crate::{response_handler::ResponseHandler, types::BoxBody};

/// custom status code response (body is empty)
pub fn status_code_response(
    status_code: &StatusCode,
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    ResponseHandler::default()
        .with_status(status_code)
        .into_response(request_headers)
}

/// custom status code response with message in body
pub fn status_code_response_with_message(
    status_code: &StatusCode,
    message: &str,
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    ResponseHandler::default()
        .with_status(status_code)
        .with_text(message, None)
        .into_response(request_headers)
}
