use hyper::StatusCode;

use crate::{
    constant::DUMMY_BINARY_DATA,
    util::{
        http::{test_request::TestRequest, test_response::response_body_bytes},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn dyn_data_dir_binary_data() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/data/binary.data", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_archive_gzip() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/data/archive.gz", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_archive_tar_gzip() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/data/archive.tar.gz", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}
