use std::str::FromStr;

use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap, StatusCode,
};
use serde_json::json;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn matches_equal_1() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/equal/1", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "equal"}).to_string());
}

#[tokio::test]
async fn not_matches_equal_1() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/equal/2", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_equal_2() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/equal/", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_not_equal_1() {
    let port: u16 = setup().await;

    let headers: HeaderMap<HeaderValue> = [("user", "not-equal-unique-request")]
        .iter()
        .map(|(k, v)| {
            (
                HeaderName::from_str(k).expect("failed to define header name"),
                HeaderValue::from_static(v),
            )
        })
        .collect();
    let response = TestRequest::default("/rule-op/not-equal/2", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "not-equal"}).to_string());
}

#[tokio::test]
async fn matches_not_equal_2() {
    let port: u16 = setup().await;

    let headers: HeaderMap<HeaderValue> = [("user", "not-equal-unique-request")]
        .iter()
        .map(|(k, v)| {
            (
                HeaderName::from_str(k).expect("failed to define header name"),
                HeaderValue::from_static(v),
            )
        })
        .collect();
    let response = TestRequest::default("/rule-op/not-equal/", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "not-equal"}).to_string());
}

#[tokio::test]
async fn not_matches_not_equal_1() {
    let port: u16 = setup().await;

    let headers: HeaderMap<HeaderValue> = [("user", "not-equal-unique-request")]
        .iter()
        .map(|(k, v)| {
            (
                HeaderName::from_str(k).expect("failed to define header name"),
                HeaderValue::from_static(v),
            )
        })
        .collect();
    let response = TestRequest::default("/rule-op/not-equal/1", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_starts_with_1() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/starts-with/12", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "starts-with"}).to_string());
}

#[tokio::test]
async fn matches_starts_with_2() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/starts-with/123", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "starts-with"}).to_string());
}

#[tokio::test]
async fn not_matches_starts_with_1() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/starts-with/1", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_starts_with_2() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/starts-with/2", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_starts_with_3() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/starts-with/312", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_starts_with_4() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/starts-with/", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_contains_1() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/contains/1", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "contains"}).to_string());
}

#[tokio::test]
async fn matches_contains_2() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/contains/12", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "contains"}).to_string());
}

#[tokio::test]
async fn matches_contains_3() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/a/contains/1/b/", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "contains"}).to_string());
}

#[tokio::test]
async fn not_matches_contains_1() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/contains/2", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_wild_card_1() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/wild-card/123/a/", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "wild-card"}).to_string());
}

#[tokio::test]
async fn matches_wild_card_2() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/wild-card/ABC/12/", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"op": "wild-card"}).to_string());
}

#[tokio::test]
async fn matches_wild_card_3() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/wild-card/123", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_wild_card_2() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/wild-card/123a", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_wild_card_3() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/wild-card/12/a", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_wild_card_4() {
    let port: u16 = setup().await;

    let response = TestRequest::default("/rule-op/wild-card/1234/a", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::RULE_WHEN_REQUEST_RULE_OP);
    let port = test_setup.launch().await;
    port
}
