//! Verifies `crates/apimock/examples/match-headers-and-body/README.md`.

use hyper::{
    Method, StatusCode,
    header::{HeaderMap, HeaderValue},
};

use crate::{
    common::example_test_setup,
    util::http::{test_request::TestRequest, test_response::response_body_str},
};

async fn setup() -> u16 {
    example_test_setup("match-headers-and-body").launch().await
}

/// `with_body_as_json` and `with_headers` each *replace*
/// `TestRequest.headers` rather than merge into it, so a request that
/// needs both an API key and a JSON content-type builds one combined
/// `HeaderMap` up front.
fn json_headers(with_api_key: bool) -> HeaderMap<HeaderValue> {
    let mut headers = HeaderMap::new();
    headers.insert(
        hyper::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if with_api_key {
        headers.insert("x-api-key", HeaderValue::from_static("k1"));
    }
    headers
}

async fn post_orders(port: u16, with_api_key: bool, body: &str) -> reqwest::Response {
    TestRequest::default("/orders", port)
        .with_http_method(&Method::POST)
        .with_headers(&json_headers(with_api_key))
        .with_body(body)
        .send()
        .await
}

#[tokio::test]
async fn missing_api_key_is_rejected() {
    let port = setup().await;
    let response = post_orders(port, false, r#"{"total": 10}"#).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response_body_str(response).await,
        "missing x-api-key header"
    );
}

#[tokio::test]
async fn vip_customer_matched_by_nested_body_field() {
    let port = setup().await;
    let response = post_orders(port, true, r#"{"customer":{"tier":"gold"},"total":10}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_body_str(response).await, "VIP customer order");
}

#[tokio::test]
async fn widget_matched_by_array_indexed_body_field() {
    let port = setup().await;
    let response = post_orders(port, true, r#"{"items":[{"sku":"WIDGET-42"}],"total":10}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_body_str(response).await, "widget order");
}

#[tokio::test]
async fn high_value_matched_by_numeric_comparison() {
    let port = setup().await;
    let response = post_orders(port, true, r#"{"total":150}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_body_str(response).await,
        "high-value order, manual review required"
    );
}

#[tokio::test]
async fn fallback_rule_creates_order() {
    let port = setup().await;
    let response = post_orders(port, true, r#"{"total":10}"#).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response_body_str(response).await, "order created");
}
