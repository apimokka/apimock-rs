//! RFC 063, Finding 1 — remotely reachable. `curl --path-as-is
//! 'http://host/../outside.txt'` used to return 200 with a file outside
//! `fallback_respond_dir`. `raw_get_status` reproduces the same
//! unnormalised-path delivery `--path-as-is` gives `curl`; `TestRequest`
//! (backed by `reqwest`) cannot — its URL parser resolves `..` out of
//! the path before the request is ever sent, which would make a test
//! built on it pass regardless of whether the fix exists.

use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{
        http::{
            raw_request::raw_get_status, test_request::TestRequest,
            test_response::response_body_str,
        },
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn a_raw_dot_dot_segment_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/../outside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

#[tokio::test]
async fn two_raw_dot_dot_segments_are_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/../../outside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// A `..` in the middle of the path, past a directory that genuinely
/// exists inside `fallback_respond_dir` — not only a leading `..`.
#[tokio::test]
async fn a_mid_path_dot_dot_segment_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/subdir/../../outside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// The encoded form was already refused before RFC 063 — hyper does not
/// decode `%2e%2e` into `..`. Regression coverage, not part of the
/// fail-first ritual: this one never needed the fix.
#[tokio::test]
async fn an_encoded_dot_dot_segment_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/%2e%2e/outside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// Normal serving inside `fallback_respond_dir` is unaffected.
#[tokio::test]
async fn a_file_actually_inside_the_respond_dir_still_serves() {
    let port = setup().await;

    let response = TestRequest::default("/hello.json", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"key\":\"hello\"}");
}

async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::CONFINEMENT_DYN_ROUTE);
    test_setup.launch().await
}
