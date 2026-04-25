use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{http::test_request::TestRequest, test_setup::TestSetup},
};

#[tokio::test]
async fn error401() {
    let port = setup().await;

    let response = TestRequest::default("/error-response/401", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn error403() {
    let port = setup().await;

    let response = TestRequest::default("/error-response/api-403", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(root_config_dir::ERROR_RESPONSE);
    let port = test_setup.launch().await;
    port
}
