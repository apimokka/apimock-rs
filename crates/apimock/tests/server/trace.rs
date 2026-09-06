//! RFC 073 F-08 — every response path emits a trace event describing
//! what it actually did, not `Outcome::Miss { status: 0 }` for
//! everything (the pre-fix bug). `TestSetup` doesn't expose the running
//! server's `TraceEmitter`, so these build an `App` directly (the same
//! pattern `listener/tls.rs`'s S-07 tests use) to subscribe before the
//! first request is sent.

use std::path::Path;

use apimock::{App, EnvArgs};
use apimock_server::trace::Outcome;

use crate::{constant::root_config_dir, util::http::test_request::TestRequest};

async fn setup() -> (
    u16,
    tokio::sync::broadcast::Receiver<apimock_server::trace::MatchTraceEvent>,
) {
    let config_file_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/config/tests")
        .join(root_config_dir::TRACE_OUTCOMES)
        .join("apimock.toml")
        .to_str()
        .expect("config file path")
        .to_owned();

    let mut env_args = EnvArgs::empty();
    env_args.config_file_path = Some(config_file_path);
    env_args.port = Some(0);

    let app = App::new(&env_args, None, true)
        .await
        .expect("App::new for trace outcome fixture");
    let rx = app.server.app_state.tracer.subscribe();

    let listener = app
        .server
        .bind_http()
        .await
        .expect("bind_http")
        .expect("http listener configured");
    let port = listener.local_addr().expect("local_addr").port();
    tokio::spawn(async move {
        app.server.serve_http(listener).await;
    });

    (port, rx)
}

/// A rule-set match emits `Matched` with the correct
/// `(rule_set_index, rule_index)` — asserted for a match in the
/// **first** rule set's **first** rule (`rule-set-a.toml`, index 0/0)
/// so a trivially-always-zero bug wouldn't pass either of these two
/// tests together.
#[tokio::test]
async fn a_rule_set_match_emits_matched_with_the_correct_indices() {
    let (port, mut rx) = setup().await;

    let _response = TestRequest::default("/rule-a", port).send().await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for trace event")
        .expect("trace event received");
    assert!(
        matches!(
            event.outcome,
            Outcome::Matched {
                rule_set_index: 0,
                rule_index: 0,
            }
        ),
        "outcome was: {:?}",
        event.outcome
    );
}

/// The same assertion, for a match in the **second** rule set's
/// **second** rule (`rule-set-b.toml`, index 1/1) — proves both indices
/// are the request's real match, not a coincidentally-correct 0/0.
#[tokio::test]
async fn a_rule_set_match_in_a_later_rule_set_and_rule_reports_both_indices() {
    let (port, mut rx) = setup().await;

    let _response = TestRequest::default("/rule-b", port).send().await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for trace event")
        .expect("trace event received");
    assert!(
        matches!(
            event.outcome,
            Outcome::Matched {
                rule_set_index: 1,
                rule_index: 1,
            }
        ),
        "outcome was: {:?}",
        event.outcome
    );
}

/// A middleware match emits `Middleware` — previously nothing was
/// emitted for this path at all.
#[tokio::test]
async fn a_middleware_match_emits_middleware_with_its_own_file_path() {
    let (port, mut rx) = setup().await;

    let _response = TestRequest::default("/mw", port).send().await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for trace event")
        .expect("trace event received");
    match event.outcome {
        Outcome::Middleware { file_path, status } => {
            assert!(
                file_path.ends_with("apimock-middleware.rhai"),
                "file_path was: {file_path}"
            );
            assert_eq!(status, 200);
        }
        other => panic!("expected Middleware, got: {other:?}"),
    }
}

/// A dyn-route fallback file actually served emits `Fallback` — the
/// resolved file path is the one served, not the request's own URL.
#[tokio::test]
async fn a_dyn_route_fallback_file_emits_fallback_with_its_resolved_path() {
    let (port, mut rx) = setup().await;

    let response = TestRequest::default("/hello", port).send().await;
    assert_eq!(response.status(), hyper::StatusCode::OK);

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for trace event")
        .expect("trace event received");
    match event.outcome {
        Outcome::Fallback { file_path, status } => {
            assert!(
                file_path.ends_with("hello.json"),
                "file_path was: {file_path}"
            );
            assert_eq!(status, 200);
        }
        other => panic!("expected Fallback, got: {other:?}"),
    }
}

/// A genuine miss — nothing matched anywhere — emits `Miss { status:
/// 404 }`, not the pre-fix `Miss { status: 0 }` for every path
/// regardless of what actually happened.
#[tokio::test]
async fn a_genuine_miss_emits_miss_with_the_real_404_status() {
    let (port, mut rx) = setup().await;

    let response = TestRequest::default("/does-not-exist", port).send().await;
    assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for trace event")
        .expect("trace event received");
    assert!(
        matches!(event.outcome, Outcome::Miss { status: 404 }),
        "outcome was: {:?}",
        event.outcome
    );
}
