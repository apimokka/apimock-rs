use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn match_root_1() {
    let port = setup().await;

    let response = TestRequest::default("/url-path/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "url-path matched");
}

#[tokio::test]
async fn match_root_empty_1() {
    let port = setup().await;

    let response = TestRequest::default("/url-path", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "url-path matched");
}

#[tokio::test]
async fn match_subdir_1() {
    let port = setup().await;

    let response = TestRequest::default("/url-path/home", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "url-path.home matched");
}

#[tokio::test]
async fn not_match_subdir_1() {
    let port = setup().await;

    let response = TestRequest::default("/url-path/field", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_match_out_of_prefix_1() {
    let port = setup().await;

    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_match_out_of_prefix_2() {
    let port = setup().await;

    let response = TestRequest::default("", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::RULE_WHEN_REQUEST_URL_PATH);
    let port = test_setup.launch().await;
    port
}
