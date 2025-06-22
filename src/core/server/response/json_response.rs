use std::collections::HashMap;

use hyper::HeaderMap;
use serde_json::Value;

use crate::core::server::{
    response::error_response::internal_server_error_response, response_handler::ResponseHandler,
    types::BoxBody,
};

/// json response
pub fn json_response(
    json_str: &str,
    custom_headers: Option<&HashMap<String, Option<String>>>,
    request_headers: &HeaderMap,
    file_path: &str,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    match json5::from_str::<Value>(json_str) {
        Ok(content) => {
            let body = content.to_string();

            let mut response_handler = ResponseHandler::default();

            if let Some(custom_headers) = custom_headers.clone() {
                response_handler = response_handler.with_headers(custom_headers.to_owned());
            }

            response_handler
                .with_json_body(body.as_str())
                .into_response(request_headers)
        }
        _ => internal_server_error_response(
            &format!("{}: invalid json content", file_path),
            request_headers,
        ),
    }
}
