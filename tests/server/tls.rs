use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn tls_server_tls_client() {
    let port = tls_setup().await;
    let response = TestRequest::default("/", port).with_https().send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"hello\":\"index\"}");
}

#[tokio::test]
#[should_panic = "library: \"SSL routines\", function: \"tls_get_more_records\""]
async fn nontls_server_tls_client() {
    let port = default_setup().await;
    let _ = TestRequest::default("/", port).with_https().send().await;
}

#[tokio::test]
#[should_panic = "hyper::Error(Parse(Version)"]
async fn tls_server_nontls_client() {
    let port = tls_setup().await;
    let _ = TestRequest::default("/", port).send().await;
}

#[tokio::test]
async fn nontls_server_nontls_client() {
    let port = default_setup().await;
    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"hello\":\"index\"}");
}

/// internal setup fn on https support config
async fn tls_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(root_config_dir::TLS);
    let port = test_setup.launch().await;
    port
}

/// internal setup fn on default config
async fn default_setup() -> u16 {
    let test_setup = TestSetup::default();
    let port = test_setup.launch().await;
    port
}
