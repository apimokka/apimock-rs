use std::str::FromStr;

use hyper::{
    HeaderMap, StatusCode,
    http::header::{HeaderName, HeaderValue},
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

/// RFC 072 — a header condition must not be satisfied by a value that
/// cannot be read as UTF-8. Before the fix, `headers.rs` returned `true`
/// on the decode failure, so this exact request would have matched the
/// `user.equal "user1"` rule and served its response despite the value
/// not being `"user1"` — it couldn't be, it isn't valid UTF-8 at all.
#[tokio::test]
async fn not_match_headers_key_non_utf8_value_fails_closed() {
    let port = setup().await;

    let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("user"),
        HeaderValue::from_bytes(&[0xFF, 0xFE]).expect("raw bytes are a valid header value"),
    );
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
    test_setup.launch().await
}
