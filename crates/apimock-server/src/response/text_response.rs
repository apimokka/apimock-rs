use hyper::HeaderMap;

use std::collections::HashMap;

use crate::{response_handler::ResponseHandler, types::BoxBody};

/// plain text response
///
/// `custom_headers` is applied *after* `with_text`, via
/// `with_custom_headers` (RFC 065), so an explicit `content-type` in
/// `custom_headers` wins over both `with_text`'s `text/plain` default
/// and `content_type` (itself only an inferred default - e.g. a
/// file-extension guess). This was previously reversed: `with_text` ran
/// last and unconditionally overwrote any explicit `content-type` a
/// caller had set (RFC 045 Defect 1b).
pub fn text_response(
    content: &str,
    content_type: Option<&str>,
    custom_headers: Option<&HashMap<String, Option<String>>>,
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    ResponseHandler::default()
        .with_text(content, content_type)
        .with_custom_headers(custom_headers)
        .into_response(request_headers)
}
