//! Verifies `crates/apimock/examples/status-codes-and-errors/README.md`.

use hyper::{Method, StatusCode};

use crate::{
    common::example_test_setup,
    util::http::{test_request::TestRequest, test_response::response_body_str},
};

async fn setup() -> u16 {
    example_test_setup("status-codes-and-errors").launch().await
}

#[tokio::test]
async fn validation_failure_400() {
    let port = setup().await;
    let response = TestRequest::default("/widgets/create", port)
        .with_http_method(&Method::POST)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response_body_str(response).await, "missing field: name");
}

#[tokio::test]
async fn missing_auth_401() {
    let port = setup().await;
    let response = TestRequest::default("/widgets/private", port).send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response_body_str(response).await, "authentication required");
}

#[tokio::test]
async fn forbidden_403() {
    let port = setup().await;
    let response = TestRequest::default("/widgets/1", port)
        .with_http_method(&Method::DELETE)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_body_str(response).await,
        "insufficient permissions"
    );
}

#[tokio::test]
async fn not_found_404() {
    let port = setup().await;
    let response = TestRequest::default("/widgets/999", port).send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response_body_str(response).await, "widget not found");
}

#[tokio::test]
async fn rate_limited_429() {
    let port = setup().await;
    let response = TestRequest::default("/widgets/rate-limited", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response_body_str(response).await,
        "rate limit exceeded, retry after 30s"
    );
}

#[tokio::test]
async fn server_error_500() {
    let port = setup().await;
    let response = TestRequest::default("/widgets/boom", port).send().await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_body_str(response).await,
        "internal error, try again later"
    );
}

#[tokio::test]
async fn no_content_204_with_empty_body() {
    let port = setup().await;
    let response = TestRequest::default("/widgets/2", port)
        .with_http_method(&Method::DELETE)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(response_body_str(response).await, "");
}
