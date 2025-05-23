use hyper::StatusCode;

use crate::util::{http_response_default, response_body_str, setup};

#[tokio::test]
async fn dyn_data_dir_js() {
    let port = setup().await;
    let response = http_response_default("/text/js/scripts.js", port).await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/javascript"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        "function hello() {\n    console.log(\"Hello from API mock (apimock-rs)\")\n}"
    );
}
