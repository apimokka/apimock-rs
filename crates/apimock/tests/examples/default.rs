//! Verifies `crates/apimock/examples/config/default/README.md`.

use hyper::StatusCode;

use crate::{
    common::example_test_setup,
    util::http::{test_request::TestRequest, test_response::response_body_str},
};

async fn setup() -> u16 {
    example_test_setup("config/default").launch().await
}

#[tokio::test]
async fn health_returns_ok() {
    let port = setup().await;
    let response = TestRequest::default("/health", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_body_str(response).await, "ok");
}

#[tokio::test]
async fn greet_returns_hello_world() {
    let port = setup().await;
    let response = TestRequest::default("/greet", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_body_str(response).await, "Hello, world.");
}

#[tokio::test]
async fn hello_is_answered_by_middleware() {
    let port = setup().await;
    let response = TestRequest::default("/hello", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_body_str(response).await, "Hello from middleware!");
}
