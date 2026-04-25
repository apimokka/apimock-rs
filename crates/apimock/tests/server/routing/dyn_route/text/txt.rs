use hyper::StatusCode;

use crate::util::env_args::SetupArgs;

#[tokio::test]
async fn dyn_data_dir_txt() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/text/txt/plain.txt", port).await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "Hello from apimock-rs (API Mock)");
}
