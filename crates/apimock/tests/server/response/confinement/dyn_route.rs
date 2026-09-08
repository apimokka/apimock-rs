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
            raw_request::raw_get_status,
            test_request::TestRequest,
            test_response::{platform_eol, response_body_str},
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

/// Task 016 — double-encoded: `%252f` decodes *once* to the literal
/// text `%2f`, not to `/`. Refused today for exactly that reason:
/// decoding runs a single pass, so this segment never becomes `..`
/// followed by a real separator, and reaches path resolution as one
/// opaque, meaningless segment. Regression coverage for a future
/// change that decoded twice, or decoded after normalising instead of
/// before — either would let this traverse, and nothing today would
/// say so.
#[tokio::test]
async fn a_double_encoded_dot_dot_slash_segment_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/..%252foutside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// Task 016 — backslash: `%5c` decodes to `\`, an ordinary filename
/// character on Linux but a path separator on Windows, where this
/// decodes to `/..\outside.txt`. On a case-insensitive, backslash-aware
/// filesystem, RFC 075's own `normalize_url_path` (which only ever
/// splits on `/`) very likely treats `..\outside.txt` as one opaque
/// segment — meaning confinement (`crates/apimock-server/src/response/confine.rs`,
/// `canonicalize()` + `starts_with(base)`) is the *only* layer refusing
/// this on Windows, not defence in depth. This is exactly the situation
/// RFC 075's own handoff warned about: relying on the second layer
/// alone is how the original advisory (GHSA-72g6-wgrg-vhm7) happened.
/// The Windows CI leg running this is the actual point of this test —
/// a Linux-only green here would prove nothing about the platform
/// where the risk is real.
#[tokio::test]
async fn a_backslash_encoded_dot_dot_segment_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/%2e%2e%5coutside.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND.as_u16());
}

/// Task 016 — overlong UTF-8: `%c0%af` is an invalid, overlong encoding
/// of `/` (a two-byte sequence for a code point that fits in one byte).
/// Some decoders have historically accepted overlong forms regardless
/// of the encoding being technically malformed — this pins that
/// apimock's own decoder does not.
#[tokio::test]
async fn an_overlong_utf8_encoded_slash_is_refused() {
    let port = setup().await;

    let status = raw_get_status("127.0.0.1", port, "/..%c0%afoutside.txt").await;

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
    assert_eq!(body_str.as_str(), platform_eol("{\"key\": \"hello\"}\n"));
}

async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::CONFINEMENT_DYN_ROUTE);
    test_setup.launch().await
}
