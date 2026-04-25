use hyper::StatusCode;
use serde_json::json;

use crate::util::{
    http::{test_request::TestRequest, test_response::response_body_str},
    test_setup::TestSetup,
};

#[tokio::test]
async fn matches_dyn_route_json_root_json_ext_none() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/root1", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"name": "root1.json"}).to_string());
}

#[tokio::test]
async fn matches_dyn_route_json_root_json_ext_json() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/root1.json", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"name": "root1.json"}).to_string());
}

#[tokio::test]
async fn matches_dyn_route_json_root_json_ext_json5() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/root1.json5", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        json!({"name": "root1.json5"}).to_string()
    );
}

#[tokio::test]
async fn matches_dyn_route_json_root_json5() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/root1.json5", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        json!({"name": "root1.json5"}).to_string()
    );
}

#[tokio::test]
async fn matches_dyn_route_json_root_multiple_1() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/root2.json5", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"name": "root2.json"}).to_string());
}

#[tokio::test]
async fn not_matches_dyn_route_json_root_multiple_1() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/root2.json", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_dyn_route_json_subdir() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/json/subdir.json", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        json!({"name": "subdir.json"}).to_string()
    );
}

#[tokio::test]
async fn matches_dyn_route_json_depth() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/json/another-dir/depth.json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"name": "depth.json"}).to_string());
}
