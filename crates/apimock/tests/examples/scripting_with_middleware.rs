//! Verifies `crates/apimock/examples/scripting-with-middleware/README.md`.

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

const PROFILE_JSON: &str = r#"{"plan":"pro","source":"middleware-file"}"#;

#[tokio::test]
async fn profile_file_path_map_return() {
    let port = setup().await;
    let response = TestRequest::default("/profile/file-path", port)
        .send()
        .await;
    assert_eq!(response_body_str(response).await, PROFILE_JSON);
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
    assert_eq!(response_body_str(response).await, PROFILE_JSON);
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
