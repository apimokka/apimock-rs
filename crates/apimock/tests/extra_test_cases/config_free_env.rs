use apimock_server::constant::CSV_RECORDS_DEFAULT_KEY;
use hyper::StatusCode;
use serde_json::json;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn config_free_env_root_1() {
    let port = setup().await;

    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_config_free_env_level1_1() {
    let port = setup().await;

    let response = TestRequest::default("/level1", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"level":"1"}).to_string());
}

#[tokio::test]
async fn matches_config_free_env_level1_2() {
    let port = setup().await;

    let response = TestRequest::default("/level1.json", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"level":"1"}).to_string());
}

#[tokio::test]
async fn not_matches_config_free_env_level1_1() {
    let port = setup().await;

    let response = TestRequest::default("/level1.json5", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_config_free_env_level2_1() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"level":"2"}).to_string());
}

#[tokio::test]
async fn matches_config_free_env_level2_2() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2.json5", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"level":"2"}).to_string());
}

#[tokio::test]
async fn not_matches_config_free_env_level2_1() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2.json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_config_free_env_level3_1() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2/level3", port)
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
        json!({CSV_RECORDS_DEFAULT_KEY:[{"level":"3"}]}).to_string()
    );
}

#[tokio::test]
async fn matches_config_free_env_level3_2() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2/level3.csv", port)
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
        json!({CSV_RECORDS_DEFAULT_KEY:[{"level":"3"}]}).to_string()
    );
}

#[tokio::test]
async fn not_matches_config_free_env_level3_1() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2/level3.json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_config_free_env_level4_1() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2/level3/level4.txt", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "level 4");
}

#[tokio::test]
async fn not_matches_config_free_env_level4_1() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2/level3/level4", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_config_free_env_level4_2() {
    let port = setup().await;

    let response = TestRequest::default("/level1/level2/level3/level4.json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// internal setup fn
async fn setup() -> u16 {
    let mut test_setup = TestSetup::default();
    test_setup.current_dir_path = Some(root_config_dir::CONFIG_FREE_ENV.to_owned());
    test_setup.root_config_file_path = None;
    let port = test_setup.launch().await;
    port
}
