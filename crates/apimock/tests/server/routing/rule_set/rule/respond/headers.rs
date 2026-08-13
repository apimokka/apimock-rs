//! RFC 045 Defect 1: `respond.headers` must be honoured on every
//! `respond` shape, and an explicit `content-type` must win over
//! whatever `ResponseHandler` would otherwise infer.

use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{http::test_request::TestRequest, test_setup::TestSetup},
};

#[tokio::test]
async fn file_path_honours_custom_header() {
    let port = setup().await;
    let response = TestRequest::default("/respond-headers/file", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-custom").unwrap(),
        "file-value",
        "file_path responses must keep honouring custom headers (no regression)"
    );
}

#[tokio::test]
async fn text_alone_honours_custom_header_and_explicit_content_type_wins() {
    let port = setup().await;
    let response = TestRequest::default("/respond-headers/text", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("x-custom").unwrap(), "text-value");
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/x-custom",
        "an explicit content-type in respond.headers must win over with_text's text/plain default"
    );
}

#[tokio::test]
async fn text_and_status_honours_custom_header() {
    let port = setup().await;
    let response = TestRequest::default("/respond-headers/text-status", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        response.headers().get("x-custom").unwrap(),
        "text-status-value",
        "respond.headers was previously dropped entirely on text + status responses"
    );
}

#[tokio::test]
async fn status_alone_honours_custom_header() {
    let port = setup().await;
    let response = TestRequest::default("/respond-headers/status", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response.headers().get("x-custom").unwrap(),
        "status-value",
        "respond.headers was previously dropped entirely on status-alone responses"
    );
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(root_config_dir::RULE_RESPOND);
    test_setup.launch().await
}
