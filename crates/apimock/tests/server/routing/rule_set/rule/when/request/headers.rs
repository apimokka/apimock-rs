use std::str::FromStr;

use hyper::{
    http::header::{HeaderName, HeaderValue},
    HeaderMap, StatusCode,
};

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn match_headers_key_1() {
    let port = setup().await;

    let headers: HeaderMap<HeaderValue> = [("user", "user1")]
        .iter()
        .map(|(k, v)| {
            (
                HeaderName::from_str(k).expect("failed to define header name"),
                HeaderValue::from_static(v),
            )
        })
        .collect();
    let response = TestRequest::default("/headers", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "headers user.equal matched");
}

#[tokio::test]
async fn match_headers_key_2() {
    let port = setup().await;

    let headers: HeaderMap<HeaderValue> = [("User", "user1")]
        .iter()
        .map(|(k, v)| {
            (
                HeaderName::from_str(k).expect("failed to define header name"),
                HeaderValue::from_static(v),
            )
        })
        .collect();
    let response = TestRequest::default("/headers", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "headers user.equal matched");
}

#[tokio::test]
async fn match_headers_key_3() {
    let port = setup().await;

    let headers: HeaderMap<HeaderValue> = [("uSER", "user1")]
        .iter()
        .map(|(k, v)| {
            (
                HeaderName::from_str(k).expect("failed to define header name"),
                HeaderValue::from_static(v),
            )
        })
        .collect();
    let response = TestRequest::default("/headers", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "headers user.equal matched");
}

#[tokio::test]
async fn match_headers_key_4() {
    let port = setup().await;

    let headers: HeaderMap<HeaderValue> = [("USER", "user1")]
        .iter()
        .map(|(k, v)| {
            (
                HeaderName::from_str(k).expect("failed to define header name"),
                HeaderValue::from_static(v),
            )
        })
        .collect();
    let response = TestRequest::default("/headers", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "headers user.equal matched");
}

#[tokio::test]
async fn match_headers_key_5() {
    let port = setup().await;

    let headers: HeaderMap<HeaderValue> = [
        ("Origin", "http://localhost:3001"),
        ("Authorization", "Bearer eyJhbxxx.xxx.xxx"),
    ]
    .iter()
    .map(|(k, v)| {
        (
            HeaderName::from_str(k).expect("failed to define header name"),
            HeaderValue::from_static(v),
        )
    })
    .collect();
    let response = TestRequest::default("/headers", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "headers authorization.contains matched");
}

#[tokio::test]
async fn not_match_headers_key_1() {
    let port = setup().await;

    let headers: HeaderMap<HeaderValue> = [("user", "user2")]
        .iter()
        .map(|(k, v)| (HeaderName::from_static(k), HeaderValue::from_static(v)))
        .collect();
    let response = TestRequest::default("/headers", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::RULE_WHEN_REQUEST_HEADERS);
    let port = test_setup.launch().await;
    port
}
