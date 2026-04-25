use hyper::StatusCode;
use serde_json::json;
use util::{
    http::{test_request::TestRequest, test_response::response_body_str},
    test_setup::TestSetup,
};

#[path = "util.rs"]
mod util;

#[tokio::test]
async fn port_env_arg_overwrites() {
    let port = u16::MAX;
    let mut test_setup = TestSetup::default();
    test_setup.port = Some(port);
    let _ = test_setup.launch().await;

    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"hello": "index"}).to_string());
}

#[tokio::test]
async fn fallback_response_dir_env_arg_overwrites() {
    let fallback_response_dir_path = "tests/fixtures";
    let mut test_setup = TestSetup::default();
    test_setup.root_config_file_path = None;
    test_setup.fallback_respond_dir_path = Some(fallback_response_dir_path.to_owned());
    let port = test_setup.launch().await;

    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        json!({"hello": "custom fallback respond dir"}).to_string()
    );
}

#[tokio::test]
async fn fallback_response_dir_env_arg_default() {
    let mut test_setup = TestSetup::default();
    test_setup.root_config_file_path = None;
    test_setup.fallback_respond_dir_path = None;
    let port = test_setup.launch().await;

    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = TestRequest::default("/tests/fixtures", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);
}
