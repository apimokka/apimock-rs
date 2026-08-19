//! RFC 055: `apimock get`'s single most important property — its answer
//! agrees with a real, running server on the same config, for a request
//! covering each dispatch stage. Everything else this command does is a
//! proxy for this; this is the test that matters (per the RFC's own
//! Testing section).

use std::process::Command;

use hyper::Method;
use serde_json::Value;

#[path = "util.rs"]
mod util;
use util::{
    http::{test_request::TestRequest, test_response::response_body_str},
    test_setup::TestSetup,
};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_apimock"))
}

/// A rule-set config plus a fallback directory with one file — covers
/// both the rule-set stage and the dyn_route stage from a single
/// workspace, so one server/one `get` invocation set exercises both.
fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \"files\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/users/1\"\nrespond.text = \"user one\"\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("files")).unwrap();
    std::fs::write(
        dir.path().join("files").join("hello.json"),
        "{\"hello\":\"index\"}",
    )
    .unwrap();
    dir
}

fn run_get(dir: &std::path::Path, path: &str, method: &str) -> Value {
    let output = bin()
        .args([
            "get",
            path,
            "-m",
            method,
            "-c",
            "./apimock.toml",
            "--format",
            "json",
        ])
        .current_dir(dir)
        .output()
        .expect("failed to run apimock get");
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("get's stdout wasn't valid JSON: {e}\nstdout was:\n{stdout}"))
}

#[tokio::test]
async fn get_agrees_with_a_running_server_across_every_stage() {
    let dir = workspace();
    let config_path = dir.path().join("apimock.toml").to_str().unwrap().to_owned();

    let test_setup = TestSetup {
        root_config_file_path: Some(config_path),
        ..Default::default()
    };
    let port = test_setup.launch().await;

    // ── Rule-set stage ──────────────────────────────────────────────
    {
        let server_response = TestRequest::default("/users/1", port).send().await;
        let server_status = server_response.status().as_u16();
        let server_body = response_body_str(server_response).await;

        let get_result = run_get(dir.path(), "/users/1", "GET");
        assert_eq!(get_result["result"]["response"]["status"], server_status);
        assert_eq!(get_result["result"]["response"]["body"], server_body);
        assert_eq!(get_result["result"]["stage"], "rule_set");
    }

    // ── dyn_route stage (the case a rules-only implementation gets
    //    wrong — RFC 055's central trap) ─────────────────────────────
    {
        let server_response = TestRequest::default("/hello", port).send().await;
        let server_status = server_response.status().as_u16();
        let server_body = response_body_str(server_response).await;

        let get_result = run_get(dir.path(), "/hello", "GET");
        assert_eq!(get_result["result"]["response"]["status"], server_status);
        assert_eq!(get_result["result"]["response"]["body"], server_body);
        assert_eq!(get_result["result"]["stage"], "dyn_route");
        assert_eq!(server_status, 200, "the file must actually be found");
    }

    // ── dyn_route stage, not found (still a legitimate, agreeing
    //    "result", not a `get`-level failure) ───────────────────────
    {
        let server_response = TestRequest::default("/does-not-exist", port).send().await;
        let server_status = server_response.status().as_u16();

        let get_result = run_get(dir.path(), "/does-not-exist", "GET");
        assert_eq!(get_result["result"]["response"]["status"], server_status);
        assert_eq!(server_status, 404);
    }

    // ── OPTIONS stage ────────────────────────────────────────────────
    {
        let server_response = TestRequest::default("/users/1", port)
            .with_http_method(&Method::OPTIONS)
            .send()
            .await;
        let server_status = server_response.status().as_u16();

        let get_result = run_get(dir.path(), "/users/1", "OPTIONS");
        assert_eq!(get_result["result"]["response"]["status"], server_status);
        assert_eq!(get_result["result"]["stage"], "options");
        assert_eq!(server_status, 204);
    }
}
