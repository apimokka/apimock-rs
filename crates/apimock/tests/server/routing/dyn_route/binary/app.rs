use hyper::StatusCode;

use crate::{
    constant::DUMMY_BINARY_DATA,
    util::{
        http::{test_request::TestRequest, test_response::response_body_bytes},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn dyn_data_dir_doc_pdf() {
    let port = TestSetup::default().launch().await;
    let response = TestRequest::default("/binary/app/doc.pdf", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/pdf"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}

#[tokio::test]
async fn dyn_data_dir_archive_zip() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/app/archive.zip", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/zip"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(body_str.as_ref(), DUMMY_BINARY_DATA);
}
