use hyper::StatusCode;

use crate::{
    constant::DUMMY_BINARY_DATA,
    util::{
        http::{test_request::TestRequest, test_response::response_body_bytes},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn dyn_data_dir_video_mp4() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/video/video.mp4", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "video/mp4");

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_video_m4v() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/video/video.m4v", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "video/mp4");

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_video_mpeg() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/video/video.mpeg", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "video/mpeg"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_video_mpg() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/video/video.mpg", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "video/mpeg"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_video_avi() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/video/video.avi", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "video/x-msvideo"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_video_mov() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/video/video.mov", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "video/quicktime"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_video_webm() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/video/video.webm", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "video/webm"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_video_ogv() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/video/video.ogv", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "video/ogg");

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}
