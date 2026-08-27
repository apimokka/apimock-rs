//! RFC 065 — the response body-source model. Asserted on the wire (a
//! real request against a running server), per the acceptance
//! checklist's own instruction: "the written TOML was already correct
//! before this RFC and wrong after the bug — only a real HTTP response
//! tells the truth."

use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

/// D1 — `json` is served as `application/json`, body byte-identical to
/// what was declared.
#[tokio::test]
async fn json_no_header_serves_application_json() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/json-no-header", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    let body = response_body_str(response).await;
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed, serde_json::json!({"id": 1, "name": "ada"}));
}

/// D2 — `json` + an explicit `content-type` is served as declared, the
/// operator keeping the last word.
#[tokio::test]
async fn json_custom_header_wins_over_the_application_json_default() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/json-custom-header", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/vnd.custom+json"
    );
}

/// D2's "deliberate inversion" row — `json` + `content-type: text/plain`
/// is served as `text/plain`. The override rule doesn't care which
/// direction the operator inverted it.
#[tokio::test]
async fn json_content_type_can_be_inverted_to_text_plain() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/json-inverted-to-text", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain"
    );
}

/// `json` + `status` — the custom status code applies, body still JSON.
#[tokio::test]
async fn json_with_custom_status_code() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/json-with-status", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    let body = response_body_str(response).await;
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed, serde_json::json!({"error": "nope"}));
}

/// D2 — the actual defect: `file_path` (`.json`) + an explicit
/// `content-type` used to be silently overwritten with
/// `application/json`. Now served as declared.
#[tokio::test]
async fn file_path_json_custom_header_wins_over_the_application_json_default() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/file-json-custom-header", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/vnd.custom+json"
    );
}

/// D2 — `file_path` (`.json`), no explicit header, is unaffected:
/// still `application/json`.
#[tokio::test]
async fn file_path_json_no_header_still_serves_application_json() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/file-json-no-header", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
}

/// Deliberate, pinned so it cannot drift: a `text` body that happens to
/// look like JSON is still served as `text/plain` — `json` is a
/// distinct, explicit choice, never inferred from content.
#[tokio::test]
async fn text_that_looks_like_json_still_serves_text_plain() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/text-looks-like-json", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );
    assert_eq!(response_body_str(response).await, r#"{"id":1}"#);
}

/// Not one of RFC 065's own four defects (`file_path` is one body
/// source; extension-derived content-type for a non-JSON, non-text
/// file was already correct) — found while consolidating the override
/// pattern into one shared method: `binary_content_type_response` had
/// the exact same ordering bug as D2's `json_response`, just never
/// named in the handoff's own D2 table (which only lists `.json`,
/// `.html`/`.css`/`.png` *without* an explicit-header row). Pinned here
/// so it can't drift back.
#[tokio::test]
async fn file_path_binary_custom_header_wins_over_the_extension_derived_default() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/file-binary-custom-header", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/vnd.custom-binary"
    );
}

/// `file_path` (`.png`), no explicit header — unaffected, still the
/// extension-derived default.
#[tokio::test]
async fn file_path_binary_no_header_still_serves_the_extension_derived_default() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/file-binary-no-header", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");
}

/// D4 — a CSV file whose rows don't match its header count fails to
/// convert; the client-facing 500 body must not name the server's
/// filesystem path (it used to: `"{file_path}: failed to analyze csv
/// records - {err}"` went straight into the response body).
#[tokio::test]
async fn malformed_csv_500_does_not_leak_the_server_path() {
    let port = setup().await;
    let response = TestRequest::default("/body-source/malformed-csv", port)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_body_str(response).await;
    assert!(
        !body.contains('/') && !body.contains('\\'),
        "500 body should not contain a path-like substring: {body:?}"
    );
    assert!(
        !body.to_lowercase().contains("malformed.csv"),
        "500 body should not name the server's file: {body:?}"
    );
}

async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::RULE_RESPOND_BODY_SOURCE);
    test_setup.launch().await
}
