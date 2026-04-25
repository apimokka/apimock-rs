use hyper::StatusCode;
use serde_json::json;

use crate::util::{
    http::{test_request::TestRequest, test_response::response_body_str},
    test_setup::TestSetup,
};

#[tokio::test]
async fn match_dyn_route_dir_root_1() {
    let port = TestSetup::default().launch().await;

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
async fn match_dyn_route_dir_root_2() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"hello": "index"}).to_string());
}

#[tokio::test]
async fn match_dyn_route_dir_subdir_1() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/subdir", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"dir": "subdir"}).to_string());
}

#[tokio::test]
async fn match_dyn_route_dir_subdir_2() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/subdir/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"dir": "subdir"}).to_string());
}

#[tokio::test]
async fn match_dyn_route_dir_json5_1() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/subdir", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"dir": "subdir"}).to_string());
}

#[tokio::test]
async fn match_dyn_route_dir_csv_1() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/subdir/deeper", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        json!({"records": [{"dir": "deeper", "note": "csv"}]}).to_string()
    );
}
