use hyper::StatusCode;

use crate::{
    constant::DUMMY_BINARY_DATA,
    util::{
        http::{test_request::TestRequest, test_response::response_body_bytes},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn dyn_data_dir_sound_mp3() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/audio/sound.mp3", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "audio/mpeg"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_sound_wav() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/audio/sound.wav", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "audio/wav");

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_sound_flac() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/audio/sound.flac", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "audio/flac"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_sound_ogg() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/audio/sound.ogg", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "audio/ogg");

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_sound_m4a() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/audio/sound.m4a", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "audio/mp4");

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_sound_aac() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/audio/sound.aac", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "audio/aac");

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_sound_weba() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/audio/sound.weba", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "audio/webm"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}
