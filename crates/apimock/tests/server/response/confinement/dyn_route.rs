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

/// RFC 075: before percent-decoding existed, this was refused only
/// because `%2e%2e` was never turned into `..` at all — hyper's raw
/// path passes it through as a literal, meaningless segment, and
/// nothing on disk is named `%2e%2e`. Now that decoding runs (ordered
/// *before* dot-segment normalisation, per RFC 075's own security-
/// critical requirement), this is refused for the intended reason:
/// decoding turns it into `..`, and the same token-removal the
/// plain-text form already gets strips it. Assert on the response, not
/// the resolved path — per the tranche 4 handoff's explicit
/// instruction that a resolved-path assertion can pass while the
/// response still leaks.
#[tokio::test]
async fn an_encoded_dot_dot_segment_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/%2e%2e/outside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// RFC 075 § 1: the whole `../` sequence percent-encoded, not just the
/// two dots — `%2f` must decode to `/` before dot-segment stripping
/// runs, or this reaches path resolution as one meaningless segment
/// instead of the traversal attempt it's disguising.
#[tokio::test]
async fn a_fully_encoded_dot_dot_slash_segment_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/%2e%2e%2foutside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// RFC 075 § 1: a literal `..` with only the trailing slash encoded —
/// the partial-encoding case the handoff names explicitly.
#[tokio::test]
async fn a_dot_dot_segment_with_an_encoded_trailing_slash_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/..%2foutside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// RFC 075 § 1: mixed-case percent-encoding (`%2E` vs `%2e`) — hex
/// digits in a percent-escape are case-insensitive by RFC 3986, so
/// decoding must not depend on the escape's letter case.
#[tokio::test]
async fn a_mixed_case_encoded_dot_dot_segment_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/%2E%2E/outside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// Normal serving inside `fallback_respond_dir` is unaffected.
#[tokio::test]
async fn a_file_actually_inside_the_respond_dir_still_serves() {
    let port = setup().await;

    let response = TestRequest::default("/hello.json", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = response_body_str(response).await;
    // RFC 076: `serve/hello.json`'s own bytes (one space after the
    // colon, trailing newline) — served byte-for-byte, not minified.
    // Updated because the bytes are now correct.
    assert_eq!(body_str.as_str(), "{\"key\": \"hello\"}\n");
}

async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::CONFINEMENT_DYN_ROUTE);
    test_setup.launch().await
}
