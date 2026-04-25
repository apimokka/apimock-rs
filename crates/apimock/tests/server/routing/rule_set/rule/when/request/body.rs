use hyper::StatusCode;
use serde_json::json;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn matches_single_level_1() {
    let port = setup().await;

    let body = json!({"a": "1"});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response1.json"}).to_string()
    );
}

#[tokio::test]
async fn not_matches_single_level_1() {
    let port = setup().await;

    let body = json!({"a": "2"});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_single_level_2() {
    let port = setup().await;

    let body = json!({"b": "1"});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_multiple_levels_1() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": "1"}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response2.json5"}).to_string()
    );
}

#[tokio::test]
async fn not_matches_multiple_levels_1() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": "2"}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_multiple_levels_2() {
    let port = setup().await;

    let body = json!({"a": {"b": ""}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_additional_field_1() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": "1", "d": ""}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response2.json5"}).to_string()
    );
}

#[tokio::test]
async fn matches_additional_field_2() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": "1"}, "d": ""}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response2.json5"}).to_string()
    );
}

#[tokio::test]
async fn matches_multiple_condition_1() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": "1", "d": "0"}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response3.json"}).to_string()
    );
}

#[tokio::test]
async fn not_matches_multiple_condition_1() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": "0", "d": "0"}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_non_string_type_value_1() {
    let port = setup().await;

    let body = "{\"a\":{\"b\":{\"c\":1}}}";
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response2.json5"}).to_string()
    );
}

#[tokio::test]
async fn matches_non_string_type_value_2() {
    let port = setup().await;

    let body = "{\"a\":{\"b\":{\"c\":\"1\",\"d\":0}}}";
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response3.json"}).to_string()
    );
}

#[tokio::test]
async fn json_request_broken_json_body_1() {
    let port = setup().await;

    let body = "{\"a\":{\"b\":{\"c\":\"1\",\"d\":}}}";
    // content-type: application/json
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn not_json_request_broken_json_body_1() {
    let port = setup().await;

    let body = "{\"a\":{\"b\":{\"c\":\"1\",\"d\":}}}";
    // content-type: NOT application/json
    let response = TestRequest::default("/body", port)
        .with_body(body)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_empty_1() {
    let port = setup().await;

    let body = "{\"a\":{\"b\":{\"e\":\"\"}}}";
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response4.json5"}).to_string()
    );
}

#[tokio::test]
async fn not_matches_empty_1() {
    let port = setup().await;

    let body = "{\"a\":{\"b\":{\"e\":0}}}";
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn matches_array_1() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": {"f": ["array"]}}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response5.json"}).to_string()
    );
}

#[tokio::test]
async fn matches_array_2() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": {"f": ["array", "additional-item"]}}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response5.json"}).to_string()
    );
}

#[tokio::test]
async fn matches_array_3() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": {"g": ["1", "2", "3"]}}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_json = response_body_str(response).await;
    assert_eq!(
        body_json,
        json!({"key": "when_request_response6.json5"}).to_string()
    );
}

#[tokio::test]
async fn not_matches_array_1() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": {"f": "array2"}}}});
    let response = TestRequest::default("/body", port)
        .with_body(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_array_2() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": {"f": []}}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_array_3() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": {"f": ""}}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_array_4() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": {"g": ["2"]}}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_matches_array_5() {
    let port = setup().await;

    let body = json!({"a": {"b": {"c": {"g": ["2", "1"]}}}});
    let response = TestRequest::default("/body", port)
        .with_body_as_json(body.to_string().as_str())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::RULE_WHEN_REQUEST_BODY);
    let port = test_setup.launch().await;
    port
}
