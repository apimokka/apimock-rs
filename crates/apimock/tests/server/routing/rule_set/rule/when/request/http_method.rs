use std::str::FromStr;

use hyper::{Method, StatusCode};

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn match_http_method_1() {
    let port = setup().await;
    let response = TestRequest::default("/http-method", port)
        .with_http_method(&Method::from_str("POST").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method POST matched");
}

#[tokio::test]
async fn match_http_method_2() {
    let port = setup().await;
    let response = TestRequest::default("/http-method/get", port)
        .with_http_method(&Method::from_str("GET").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method GET matched");
}

#[tokio::test]
async fn match_http_method_3() {
    let port = setup().await;
    let response = TestRequest::default("/http-method/put", port)
        .with_http_method(&Method::from_str("PUT").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method PUT matched");
}

#[tokio::test]
async fn match_http_method_4() {
    let port = setup().await;
    let response = TestRequest::default("/http-method/delete", port)
        .with_http_method(&Method::from_str("DELETE").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method DELETE matched");
}

#[tokio::test]
async fn not_match_http_method_1() {
    let port = setup().await;
    let response = TestRequest::default("/http-method", port)
        .with_http_method(&Method::from_str("GET").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_match_http_method_2() {
    let port = setup().await;
    let response = TestRequest::default("/http-method", port)
        .with_http_method(&Method::from_str("PUT").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_match_http_method_3() {
    let port = setup().await;
    let response = TestRequest::default("/http-method", port)
        .with_http_method(&Method::from_str("DELETE").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::RULE_WHEN_REQUEST_HTTP_METHOD);
    let port = test_setup.launch().await;
    port
}
