//! Verifies `crates/apimock/examples/serve-json-resources/README.md`.
//!
//! # RFC 076 updated three of these expectations
//!
//! `users_collection`, `users_member` and `orders_collection` used to
//! assert **minified, alphabetical-key** bodies
//! (`{"email":...,"id":...,"name":...}`) — that was the pre-RFC-076
//! defect itself, pinned as if it were correct. A `.json` `file_path`
//! is now served byte-for-byte (see `file_response.rs`), so the
//! expected bodies below are the fixture files' own bytes — pretty-
//! printed, `id`/`name`/`email` order, exactly as authored in
//! `data/*.json`. **Updated because the bytes are now correct, per RFC
//! 076's own acceptance criterion asking this to be said explicitly.**
//! `products_csv_converts_to_json` is unchanged: CSV conversion is an
//! explicit RFC 076 non-goal, still parsed and reserialised.

use hyper::StatusCode;

use crate::{
    common::example_test_setup,
    util::http::{
        test_request::TestRequest,
        test_response::{platform_eol, response_body_str},
    },
};

async fn setup() -> u16 {
    example_test_setup("serve-json-resources").launch().await
}

#[tokio::test]
async fn users_collection() {
    let port = setup().await;
    let response = TestRequest::default("/users", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(
        response_body_str(response).await,
        platform_eol(
            "[\n  { \"id\": 1, \"name\": \"Ada Lovelace\", \"email\": \"ada@example.com\" },\n  { \"id\": 2, \"name\": \"Grace Hopper\", \"email\": \"grace@example.com\" }\n]\n"
        )
    );
}

#[tokio::test]
async fn users_member() {
    let port = setup().await;
    let response = TestRequest::default("/users/1", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_body_str(response).await,
        platform_eol("{ \"id\": 1, \"name\": \"Ada Lovelace\", \"email\": \"ada@example.com\" }\n")
    );
}

#[tokio::test]
async fn orders_collection() {
    let port = setup().await;
    let response = TestRequest::default("/orders", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_body_str(response).await,
        platform_eol(
            "[\n  { \"id\": 101, \"userId\": 1, \"total\": 42.50 },\n  { \"id\": 102, \"userId\": 2, \"total\": 17.00 }\n]\n"
        )
    );
}

#[tokio::test]
async fn products_csv_converts_to_json() {
    let port = setup().await;
    let response = TestRequest::default("/products", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(
        response_body_str(response).await,
        r#"{"records":[{"id":"1","name":"Widget","price":"9.99"},{"id":"2","name":"Gadget","price":"19.99"}]}"#
    );
}
