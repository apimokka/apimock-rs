//! Verifies `crates/apimock/examples/serve-json-resources/README.md`.

use hyper::StatusCode;

use crate::{
    common::example_test_setup,
    util::http::{test_request::TestRequest, test_response::response_body_str},
};

async fn setup() -> u16 {
    example_test_setup("serve-json-resources").launch().await
}

#[tokio::test]
async fn users_collection() {
    let port = setup().await;
    let response = TestRequest::default("/users", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(
        response_body_str(response).await,
        r#"[{"email":"ada@example.com","id":1,"name":"Ada Lovelace"},{"email":"grace@example.com","id":2,"name":"Grace Hopper"}]"#
    );
}

#[tokio::test]
async fn users_member() {
    let port = setup().await;
    let response = TestRequest::default("/users/1", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_body_str(response).await,
        r#"{"email":"ada@example.com","id":1,"name":"Ada Lovelace"}"#
    );
}

#[tokio::test]
async fn orders_collection() {
    let port = setup().await;
    let response = TestRequest::default("/orders", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_body_str(response).await,
        r#"[{"id":101,"total":42.5,"userId":1},{"id":102,"total":17.0,"userId":2}]"#
    );
}

#[tokio::test]
async fn products_csv_converts_to_json() {
    let port = setup().await;
    let response = TestRequest::default("/products", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(
        response_body_str(response).await,
        r#"{"records":[{"id":"1","name":"Widget","price":"9.99"},{"id":"2","name":"Gadget","price":"19.99"}]}"#
    );
}
