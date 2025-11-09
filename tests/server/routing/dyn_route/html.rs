use std::sync::LazyLock;

use hyper::StatusCode;

use crate::util::{
    http::{test_request::TestRequest, test_response::response_body_str},
    test_setup::TestSetup,
};

const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };
static EXPECT: LazyLock<String> = LazyLock::new(|| {
    format!(
        "<!DOCTYPE html>{}Hello from apimock-rs (API Mock)",
        NEW_LINE
    )
});

#[tokio::test]
async fn match_dyn_data_dir_html_1() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/html/index.html", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), *EXPECT);
}

#[tokio::test]
async fn match_dyn_data_dir_html_2() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/html/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), *EXPECT);
}

#[tokio::test]
async fn match_dyn_data_dir_html_3() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/html", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), *EXPECT);
}
