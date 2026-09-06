//! Verifies `crates/apimock/examples/scripting-with-middleware/README.md`.
//!
//! # RFC 076 updated `PROFILE_JSON_FILE`
//!
//! `/profile/file-path` and `/profile` both resolve to
//! `data/profile.json` via `FileResponse`'s `file_path` handling, the
//! same path RFC 076 fixed to serve `.json` bytes as written. The old
//! `PROFILE_JSON` constant asserted the pre-fix minified body; the file
//! itself is written with spaces around braces/colons and a trailing
//! newline, which is what a client now actually receives. **Updated
//! because the bytes are now correct**, per RFC 076's own acceptance
//! criterion asking this to be said explicitly.

use hyper::Method;

use crate::{
    common::example_test_setup,
    util::http::{test_request::TestRequest, test_response::response_body_str},
};

async fn setup() -> u16 {
    example_test_setup("scripting-with-middleware")
        .launch()
        .await
}

/// `data/profile.json`'s own bytes — served as written since RFC 076.
const PROFILE_JSON_FILE: &str = "{ \"plan\": \"pro\", \"source\": \"middleware-file\" }\n";

#[tokio::test]
async fn profile_file_path_map_return() {
    let port = setup().await;
    let response = TestRequest::default("/profile/file-path", port)
        .send()
        .await;
    assert_eq!(response_body_str(response).await, PROFILE_JSON_FILE);
}

#[tokio::test]
async fn profile_json_map_return() {
    let port = setup().await;
    let response = TestRequest::default("/profile/json", port).send().await;
    assert_eq!(
        response_body_str(response).await,
        r#"{"plan":"pro","source":"middleware-json"}"#
    );
}

#[tokio::test]
async fn profile_text_map_return() {
    let port = setup().await;
    let response = TestRequest::default("/profile/text", port).send().await;
    assert_eq!(
        response_body_str(response).await,
        "plan: pro (middleware-text)"
    );
}

#[tokio::test]
async fn profile_bare_string_return() {
    let port = setup().await;
    let response = TestRequest::default("/profile", port).send().await;
    assert_eq!(response_body_str(response).await, PROFILE_JSON_FILE);
}

#[tokio::test]
async fn rush_order_handled_by_middleware_body_inspection() {
    let port = setup().await;
    let response = TestRequest::default("/orders", port)
        .with_http_method(&Method::POST)
        .with_body_as_json(r#"{"priority":"rush"}"#)
        .send()
        .await;
    assert_eq!(
        response_body_str(response).await,
        "expedited: this order jumps the queue"
    );
}

#[tokio::test]
async fn standard_order_falls_through_to_rule_set() {
    let port = setup().await;
    let response = TestRequest::default("/orders", port)
        .with_http_method(&Method::POST)
        .with_body_as_json(r#"{"priority":"normal"}"#)
        .send()
        .await;
    assert_eq!(
        response_body_str(response).await,
        "standard: order queued normally"
    );
}

#[tokio::test]
async fn bodyless_order_falls_through_to_rule_set() {
    let port = setup().await;
    let response = TestRequest::default("/orders", port).send().await;
    assert_eq!(
        response_body_str(response).await,
        "standard: order queued normally"
    );
}
