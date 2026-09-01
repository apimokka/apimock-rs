//! RFC 067 — credentialed CORS reflection only for an allowed origin.
//!
//! `crates/apimock/tests/extra_test_cases/response_headers.rs` already
//! covers rows 1 and 2 (no-auth, and a loopback origin with auth — the
//! same behaviour before and after this RFC, both re-run unchanged as
//! part of this branch's gate suite). This module covers what changed:
//! row 2's *non-loopback, explicitly-listed* case, and row 3 (the new
//! row — an unlisted origin degrades instead of reflecting).
//!
//! # Before this fix (reproduced live, not by this suite)
//!
//! ```text
//! $ curl -H 'Origin: https://evil.example' -H 'Cookie: session=abc' …
//! access-control-allow-credentials: true
//! access-control-allow-origin: https://evil.example
//! ```
//! Any origin, no allowlist, not configurable — exactly RFC 067's own
//! Motivation section, reproduced against this branch's own baseline
//! commit before writing the fix below.

use std::time::Duration;

use hyper::{
    HeaderMap, StatusCode,
    header::{HeaderName, HeaderValue},
};

use apimock::{App, EnvArgs};
use tokio::net::TcpListener;

use crate::util::http::test_request::TestRequest;

/// Launch an HTTP server with `[service].cors_allow_credentials_origins`
/// set to `listed_origins`, no rule sets (dyn_route's 404 path still
/// carries the CORS headers this suite checks, same as every other
/// response path).
async fn launch_with_cors_origins(listed_origins: &[&str]) -> u16 {
    let dir = tempfile::tempdir().expect("tempdir");
    let port_probe = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let port = port_probe.local_addr().expect("local_addr").port();
    drop(port_probe);

    let origins_toml_array = listed_origins
        .iter()
        .map(|o| format!("\"{o}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let toml_path = dir.path().join("apimock.toml");
    std::fs::write(
        &toml_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = {port}\n\
             [service]\n\
             rule_sets = []\n\
             fallback_respond_dir = \".\"\n\
             cors_allow_credentials_origins = [{origins_toml_array}]\n"
        ),
    )
    .expect("write apimock.toml");

    let mut env_args = EnvArgs::empty();
    env_args.config_file_path = Some(toml_path.to_string_lossy().into_owned());
    env_args.port = Some(port);

    let app = App::new(&env_args, None, true)
        .await
        .expect("App::new for CORS test fixture");
    let listener = app
        .server
        .bind_http()
        .await
        .expect("bind_http")
        .expect("http listener configured");
    tokio::spawn(async move {
        app.server.serve_http(listener).await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    port
}

fn headers_with_origin_and_credential(origin: &str) -> HeaderMap<HeaderValue> {
    [
        ("Origin", origin),
        ("Authorization", "Bearer eyJhbxxx.xxx.xxx"),
    ]
    .into_iter()
    .map(|(k, v)| {
        (
            HeaderName::from_bytes(k.as_bytes()).unwrap(),
            HeaderValue::from_str(v).unwrap(),
        )
    })
    .collect()
}

/// RFC 067 § Design table, row: credentialed, origin **in** the list —
/// reflect + credentials, unchanged from before this RFC. The loopback
/// case of this same row is `extra_test_cases/response_headers.rs`'s
/// `http_response_headers_on_request_with_auth` (localhost, allowed
/// implicitly); this is the *non-loopback, explicitly listed* case,
/// which needs a configured origin to test at all.
#[tokio::test]
async fn credentialed_request_from_a_listed_non_loopback_origin_gets_reflected_with_credentials() {
    let port = launch_with_cors_origins(&["https://trusted.example"]).await;

    let response = TestRequest::default("/", port)
        .with_headers(&headers_with_origin_and_credential(
            "https://trusted.example",
        ))
        .send()
        .await;

    assert_eq!(
        response.headers().get("access-control-allow-origin"),
        Some(&HeaderValue::from_static("https://trusted.example")),
    );
    assert_eq!(
        response.headers().get("access-control-allow-credentials"),
        Some(&HeaderValue::from_static("true")),
    );
    assert_eq!(
        response.headers().get("vary"),
        Some(&HeaderValue::from_static("Origin")),
        "Vary: Origin must be present whenever the origin is reflected"
    );
}

/// RFC 067 § Design table, the new row: credentialed, origin **not**
/// in the list — degrades to the safe non-credentialed path. The
/// response is still served (asserted via the 200 status): this is a
/// browser-enforced read restriction, not a server-side refusal.
#[tokio::test]
async fn credentialed_request_from_an_unlisted_origin_gets_no_credentials() {
    let port = launch_with_cors_origins(&["https://trusted.example"]).await;

    let response = TestRequest::default("/", port)
        .with_headers(&headers_with_origin_and_credential("https://evil.example"))
        .send()
        .await;

    assert_eq!(
        response.headers().get("access-control-allow-origin"),
        Some(&HeaderValue::from_static("*")),
        "an unlisted origin must not have its Origin reflected"
    );
    assert_eq!(
        response.headers().get("access-control-allow-credentials"),
        None,
        "an unlisted origin must not get Access-Control-Allow-Credentials at all"
    );
    assert_eq!(
        response.headers().get("vary"),
        Some(&HeaderValue::from_static("*")),
        "the degraded path is indistinguishable from the no-auth path's Vary value"
    );
}

/// RFC 067 § Design "the convenience question": a loopback origin
/// works with credentials and **no configuration** — `127.0.0.1`
/// variant. `extra_test_cases/response_headers.rs` already covers the
/// `localhost` variant; both prefixes are checked independently by
/// `is_implicit_loopback_origin`, so both are worth a real request.
#[tokio::test]
async fn loopback_127_0_0_1_origin_works_with_credentials_and_no_configuration() {
    let port = launch_with_cors_origins(&[]).await;

    let response = TestRequest::default("/", port)
        .with_headers(&headers_with_origin_and_credential("http://127.0.0.1:5173"))
        .send()
        .await;

    assert_eq!(
        response.headers().get("access-control-allow-origin"),
        Some(&HeaderValue::from_static("http://127.0.0.1:5173")),
    );
    assert_eq!(
        response.headers().get("access-control-allow-credentials"),
        Some(&HeaderValue::from_static("true")),
    );
}

/// A non-loopback origin that merely *starts with* the loopback prefix
/// must not be treated as loopback — the exact bug shape
/// `is_implicit_loopback_origin`'s own doc comment calls out.
#[tokio::test]
async fn a_lookalike_origin_is_not_treated_as_loopback() {
    let port = launch_with_cors_origins(&[]).await;

    let response = TestRequest::default("/", port)
        .with_headers(&headers_with_origin_and_credential(
            "http://localhost.evil.example",
        ))
        .send()
        .await;

    assert_eq!(
        response.headers().get("access-control-allow-origin"),
        Some(&HeaderValue::from_static("*")),
        "a lookalike origin (matches the loopback prefix but isn't loopback) must be refused \
         credentials"
    );
    assert_eq!(
        response.headers().get("access-control-allow-credentials"),
        None,
    );
}

/// RFC 067 acceptance: "the response is still served" for an unlisted
/// credentialed origin — refusing the cross-origin *read* is the
/// browser's job, not a server-side error.
#[tokio::test]
async fn the_response_is_still_served_for_an_unlisted_origin() {
    let port = launch_with_cors_origins(&[]).await;

    let response = TestRequest::default("/nonexistent-path-for-this-fixture", port)
        .with_headers(&headers_with_origin_and_credential("https://evil.example"))
        .send()
        .await;

    // This fixture has no rule sets and nothing at that path, so a 404
    // is the expected *content* of the response — the point is that a
    // real response came back at all, not that it errored out.
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
