use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn middleware_url_path_handled_string() {
    let port = setup().await;

    let response = TestRequest::default("/middleware-test", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        "{\"discovery\":\"reached by rhai script !\"}"
    );
}

#[tokio::test]
async fn middleware_url_path_handled_map_file_path() {
    let port = setup().await;

    let response = TestRequest::default("/middleware-test/map/file-path", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        "{\"discovery\":\"reached by rhai script !\"}"
    );
}

#[tokio::test]
async fn middleware_url_path_handled_map_json() {
    let port = setup().await;

    let response = TestRequest::default("/middleware-test/map/json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"greetings\":\"Hello, world.\"}");
}

#[tokio::test]
async fn middleware_url_path_handled_map_text() {
    let port = setup().await;

    let response = TestRequest::default("/middleware-test/map/text", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "Hello, world.");
}

#[tokio::test]
async fn middleware_url_path_missed() {
    let port = setup().await;

    let response = TestRequest::default("/middleware-test/dummy", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "");
}

#[tokio::test]
async fn middleware_body_handled() {
    let port = setup().await;
    let body = "{\"middleware\": \"isHere\"}";

    let response = TestRequest::default("/middleware-test/dummy", port)
        .with_body_as_json(body)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"thisIs\":\"missedByConfigToml\"}");
}

#[tokio::test]
async fn middleware_body_missed() {
    let port = setup().await;
    let body = "{\"middleware\": \"isHere?\"}";

    let response = TestRequest::default("/middleware-test/dummy", port)
        .with_body_as_json(body)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "");
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(root_config_dir::MIDDLEWARE);
    let port: u16 = test_setup.launch().await;
    port
}
