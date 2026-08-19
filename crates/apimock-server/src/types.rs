use http_body_util::BodyExt;
use hyper::body::Bytes;

use std::convert::Infallible;

pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

/// A response fully read into memory — status, headers, body — rather
/// than the streaming `hyper::Response<BoxBody>` every dispatch function
/// returns.
///
/// # Why this exists (RFC 055)
///
/// `apimock get` answers a question; it does not write to a socket.
/// Reusing `rule_set_response`/`dyn_route_content`/`respond_response`
/// unchanged (the RFC's own "one implementation of matching" principle)
/// still leaves a `BoxBody` on the way out, and a CLI command has
/// nothing to stream it *to* — it needs the bytes, once, to print or
/// serialise. This is the minimal additive surface that gap needed:
/// no new dispatch logic, just a collector for what dispatch already
/// produces.
pub struct CollectedResponse {
    pub status: hyper::StatusCode,
    /// Header order preserved as received; values that aren't valid
    /// UTF-8 are dropped, matching how `RequestSummary`'s own header
    /// collection (`trace.rs`) already treats non-UTF-8 values.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl CollectedResponse {
    /// Consume a streaming response and read it fully into memory.
    pub async fn collect(response: hyper::Response<BoxBody>) -> Self {
        let (parts, body) = response.into_parts();
        // `BoxBody`'s error type is `Infallible` (see the alias above) —
        // collection cannot fail.
        let bytes = body
            .collect()
            .await
            .expect("BoxBody's error type is Infallible")
            .to_bytes();
        let headers = parts
            .headers
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_owned())))
            .collect();
        Self {
            status: parts.status,
            headers,
            body: bytes.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Full;

    #[tokio::test]
    async fn collect_reads_status_headers_and_body() {
        let response = hyper::Response::builder()
            .status(201)
            .header("x-custom", "yes")
            .body(
                Full::new(Bytes::from_static(b"hello"))
                    .map_err(|e| match e {})
                    .boxed(),
            )
            .unwrap();

        let collected = CollectedResponse::collect(response).await;
        assert_eq!(collected.status, hyper::StatusCode::CREATED);
        assert_eq!(collected.body, b"hello");
        assert!(
            collected
                .headers
                .iter()
                .any(|(k, v)| k == "x-custom" && v == "yes")
        );
    }
}
