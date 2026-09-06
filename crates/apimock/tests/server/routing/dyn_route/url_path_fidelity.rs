//! RFC 075 — URL path fidelity: percent-decoding (F-03) and uniform
//! case-insensitivity across every segment, not just the filename
//! (F-05). Fixtures live under
//! `examples/config/tests/apimock-dyn-route/url-path-fidelity/`.

use hyper::StatusCode;

use crate::util::{
    http::{
        test_request::TestRequest,
        test_response::{platform_eol, response_body_str},
    },
    test_setup::TestSetup,
};

/// `%20` decodes to a literal space, resolving a filename that could
/// never otherwise be requested.
#[tokio::test]
async fn percent_encoded_space_resolves() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/url-path-fidelity/my%20file.json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_str(response).await;
    assert_eq!(
        body,
        platform_eol("{\n    \"fixture\": \"space in filename\"\n}")
    );
}

/// A non-ASCII filename, requested percent-encoded — unreachable before
/// RFC 075 (percent-decoding never happened at all).
#[tokio::test]
async fn percent_encoded_non_ascii_filename_resolves() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/url-path-fidelity/caf%C3%A9.json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_str(response).await;
    assert_eq!(
        body,
        platform_eol("{\n    \"fixture\": \"non-ascii filename\"\n}")
    );
}

/// The same non-ASCII filename, requested with the literal UTF-8 bytes
/// rather than percent-escaped — both spellings of the same request
/// must resolve to the same file.
#[tokio::test]
async fn literal_utf8_non_ascii_filename_resolves() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/url-path-fidelity/café.json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_str(response).await;
    assert_eq!(
        body,
        platform_eol("{\n    \"fixture\": \"non-ascii filename\"\n}")
    );
}

/// `+` in a path is not a space substitute — that's
/// `application/x-www-form-urlencoded` behaviour, for query strings and
/// form bodies, not paths (RFC 3986). A literal `+` in a filename
/// resolves unencoded.
#[tokio::test]
async fn a_literal_plus_in_a_filename_resolves_unencoded() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/url-path-fidelity/a+b.json", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_str(response).await;
    assert_eq!(
        body,
        platform_eol("{\n    \"fixture\": \"literal plus in filename\"\n}")
    );
}

/// RFC 075 F-05, the cross-platform assertion the tranche 4 handoff
/// names as the actual finding: case-insensitivity extended to *every*
/// segment, not only the final filename. `CaseDir/File.json` exists on
/// disk with that exact case; every case variant of the request below
/// must resolve to it identically — on Linux (case-sensitive
/// filesystem, the platform where this used to 404 before RFC 075) and
/// on macOS/Windows (case-insensitive filesystem, where the *filename*
/// segment already worked before RFC 075 but a differently-cased
/// *directory* segment happened to work too, via the OS rather than
/// apimock). This test doesn't distinguish those two reasons — it
/// only asserts the observable outcome is identical either way, which
/// is what F-05 requires. CI's three-platform matrix is what confirms
/// this actually holds everywhere, not just on this development
/// machine's filesystem.
#[tokio::test]
async fn case_insensitivity_extends_to_every_segment_not_only_the_filename() {
    let port = TestSetup::default().launch().await;

    for path in [
        "/url-path-fidelity/CaseDir/File.json",
        "/url-path-fidelity/casedir/file.json",
        "/url-path-fidelity/CASEDIR/FILE.JSON",
        "/URL-PATH-FIDELITY/CaseDir/File.json",
        "/url-path-fidelity/CaseDir/file",
    ] {
        let response = TestRequest::default(path, port).send().await;
        assert_eq!(response.status(), StatusCode::OK, "request path: {path}");
        let body = response_body_str(response).await;
        assert_eq!(
            body,
            platform_eol("{\n    \"fixture\": \"case-insensitive segment resolution\"\n}"),
            "request path: {path}"
        );
    }
}
