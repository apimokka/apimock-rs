use hyper::body::Bytes;
use reqwest::Response;

/// convert response body bytes to string
pub async fn response_body_str(response: Response) -> String {
    let body_bytes = response_body_bytes(response).await;
    let body_str = String::from_utf8(body_bytes.into()).unwrap();
    body_str
}

/// convert response body bytes to string
pub async fn response_body_bytes(response: Response) -> Bytes {
    response
        .bytes()
        .await
        .expect("failed to get response body bytes")
}
