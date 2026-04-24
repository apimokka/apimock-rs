use hyper::HeaderMap;

use std::collections::HashMap;

use crate::{response_handler::ResponseHandler, types::BoxBody};

/// plain text response
pub fn text_response(
    content: &str,
    content_type: Option<&str>,
    custom_headers: Option<&HashMap<String, Option<String>>>,
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let mut response_handler = ResponseHandler::default();
    if let Some(custom_headers) = custom_headers {
        response_handler = response_handler.with_headers(custom_headers.to_owned());
    }
    if let Some(content_type) = content_type {
        response_handler = response_handler.with_header("content-type", Some(content_type));
    }
    response_handler
        .with_headers(match custom_headers {
            Some(x) => x.to_owned(),
            None => HashMap::new(),
        })
        .with_text(content, content_type)
        .into_response(request_headers)
}
