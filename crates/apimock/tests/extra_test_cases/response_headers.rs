use std::str::FromStr;

use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap, StatusCode,
};

use crate::util::{http::test_request::TestRequest, test_setup::TestSetup};

const REQUIRED_RESPONSE_HEADERS: &[(&str, Option<&str>)] = &[
    ("date", None),
    ("content-length", None),
    ("cache-control", Some("no-store")),
    ("access-control-allow-headers", Some("*")),
    ("access-control-max-age", Some("86400")),
    (
        "access-control-allow-methods",
        Some("GET, POST, PUT, DELETE, OPTIONS"),
    ),
    ("x-content-type-options", Some("nosniff")),
    ("connection", Some("keep-alive")),
];

const REQUIRED_RESPONSE_HEADERS_ON_REQUEST_WITHOUT_AUTH: &[(&str, Option<&str>)] = &[
    ("access-control-allow-origin", Some("*")),
    ("vary", Some("*")),
];

const REQUIRED_RESPONSE_HEADERS_ON_REQUEST_WITH_AUTH: &[(&str, Option<&str>)] = &[
    ("access-control-allow-origin", Some("http://localhost:3001")),
    ("vary", Some("Origin")),
    ("access-control-allow-credentials", Some("true")),
];

#[tokio::test]
async fn http_response_headers_on_request_without_auth() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/root1", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    for (header_key, header_value) in [
        REQUIRED_RESPONSE_HEADERS,
        REQUIRED_RESPONSE_HEADERS_ON_REQUEST_WITHOUT_AUTH,
    ]
    .concat()
    {
        let header = response
            .headers()
            .get(header_key)
            .expect(&format!("failed to get header: {}", header_key));
        if let Some(header_value) = header_value {
            assert_eq!(header, HeaderValue::from_static(header_value),);
        }
    }
}

#[tokio::test]
async fn http_response_headers_on_request_with_auth() {
    let port = TestSetup::default().launch().await;

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
    let response = TestRequest::default("/root1", port)
        .with_headers(&headers)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    for (header_key, header_value) in [
        REQUIRED_RESPONSE_HEADERS,
        REQUIRED_RESPONSE_HEADERS_ON_REQUEST_WITH_AUTH,
    ]
    .concat()
    {
        let header = response
            .headers()
            .get(header_key)
            .expect(&format!("failed to get header: {}", header_key));
        if let Some(header_value) = header_value {
            assert_eq!(header, HeaderValue::from_static(header_value),);
        }
    }
}
