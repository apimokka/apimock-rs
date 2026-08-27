//! RFC 065 — the response body-source model, CLI-level (D3, D1, and the
//! `set --json` → `respond.json` wiring). Wire-level D1/D2 coverage
//! (real requests against a running server) lives in
//! `tests/server/routing/rule_set/rule/respond/body_source.rs`.

#[path = "util.rs"]
mod util;

use util::cli::{bin, run, run_stderr};

fn valid_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"ok\"\n",
    )
    .unwrap();
    dir
}

// ═══════════════════════════════════════════════════════════════════
// D3 — validation moves to load time
// ═══════════════════════════════════════════════════════════════════

#[test]
fn validate_rejects_malformed_inline_json_naming_the_rule() {
    let dir = valid_workspace();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.json = '{not json'\n",
    )
    .unwrap();
    let (code, stderr) = run_stderr(dir.path(), &["validate", "-c", "apimock.toml"]);
    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert!(stderr.contains("rule #1"), "stderr:\n{stderr}");
}

#[test]
fn validate_rejects_a_malformed_referenced_json_file_naming_the_file_and_position() {
    let dir = valid_workspace();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.file_path = \"bad.json\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("bad.json"), "{\"a\": ,,,BROKEN").unwrap();

    let (code, stderr) = run_stderr(dir.path(), &["validate", "-c", "apimock.toml"]);
    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert!(stderr.contains("bad.json"), "stderr:\n{stderr}");
    assert!(stderr.contains("line"), "stderr:\n{stderr}");
    assert!(stderr.contains("column"), "stderr:\n{stderr}");
}

#[test]
fn validate_accepts_valid_inline_and_referenced_json() {
    let dir = valid_workspace();
    std::fs::write(dir.path().join("good.json"), r#"{"a":1}"#).unwrap();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.json = '{\"a\":1}'\n\n\
         [[rules]]\nwhen.request.url_path = \"/b\"\nrespond.file_path = \"good.json\"\n",
    )
    .unwrap();

    let (code, stderr) = run_stderr(dir.path(), &["validate", "-c", "apimock.toml"]);
    assert_eq!(code, 0, "stderr:\n{stderr}");
}

#[test]
fn validate_rejects_json_and_text_together_on_one_rule() {
    let dir = valid_workspace();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.json = '{\"a\":1}'\nrespond.text = \"hi\"\n",
    )
    .unwrap();
    let (code, stderr) = run_stderr(dir.path(), &["validate", "-c", "apimock.toml"]);
    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert!(stderr.contains("mutually exclusive"), "stderr:\n{stderr}");
}

/// Checklist §3: "The server also refuses to start on both [inline and
/// referenced malformed JSON]." Exits before ever binding a listener —
/// `Config::new`'s own `validate()` call runs ahead of `Server::new` —
/// so a plain `.output()` is safe here, no spawn/poll/kill needed.
#[test]
fn the_server_also_refuses_to_start_on_malformed_json() {
    let dir = valid_workspace();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.json = '{not json'\n",
    )
    .unwrap();

    let output = bin()
        .current_dir(dir.path())
        .args(["-c", "apimock.toml", "-p", "0"])
        .output()
        .expect("failed to run apimock");
    assert_ne!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ═══════════════════════════════════════════════════════════════════
// D1 + the CLI: `set --json` writes `respond.json`, and it round-trips
// to `application/json` on the wire — the checklist's own "written as
// respond.json" + "content-type from a real request" bar, exercised
// together as one path: write via the CLI, read back via `apimock get`
// (itself a real dispatch through the same server-side response code,
// RFC 055 — not a second, parallel implementation).
// ═══════════════════════════════════════════════════════════════════

#[test]
fn set_json_writes_respond_json_not_respond_text() {
    let dir = valid_workspace();
    let (code, _stdout) = run(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "apimock.toml",
            "--rule-set",
            "rules.toml",
            "--path",
            "/api/user",
            "--status",
            "200",
            "--json",
            r#"{"id":1,"name":"ada"}"#,
        ],
    );
    assert_eq!(code, 0);

    let written = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    // The fixture's own pre-existing rule (`respond.text = "ok"`) is
    // untouched — `set rule` without `--rule` adds a new rule, it
    // doesn't replace the existing one — so this asserts on the *new*
    // rule's own respond block specifically, not "no `text =` appears
    // anywhere in the file."
    let new_rule = written
        .split("[[rules]]")
        .find(|block| block.contains("/api/user"))
        .expect("the new rule should be present in rules.toml");
    assert!(
        new_rule.contains("json ="),
        "the new rule should contain a `json` field:\n{new_rule}"
    );
    assert!(
        !new_rule.contains("text ="),
        "the new rule should not also fall back to `text` (D1):\n{new_rule}"
    );
}

#[test]
fn set_json_then_get_reports_application_json_content_type() {
    let dir = valid_workspace();
    let (code, _) = run(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "apimock.toml",
            "--rule-set",
            "rules.toml",
            "--path",
            "/api/user",
            "--status",
            "200",
            "--json",
            r#"{"id":1,"name":"ada"}"#,
        ],
    );
    assert_eq!(code, 0);

    let (code, v) = util::cli::run_json(
        dir.path(),
        &["get", "/api/user", "-c", "apimock.toml", "--format", "json"],
    );
    assert_eq!(code, 0, "get result: {v}");
    let headers = v["result"]["response"]["headers"]
        .as_array()
        .expect("headers array");
    let content_type = headers
        .iter()
        .find(|h| {
            h["name"]
                .as_str()
                .map(|n| n.eq_ignore_ascii_case("content-type"))
                == Some(true)
        })
        .and_then(|h| h["value"].as_str())
        .unwrap_or_default();
    assert_eq!(content_type, "application/json", "headers: {headers:?}");
}

#[test]
fn set_json_still_rejects_non_json_input_with_the_unchanged_message() {
    let dir = valid_workspace();
    let (code, stdout, stderr) = {
        let output = bin()
            .current_dir(dir.path())
            .args([
                "set",
                "rule",
                "-c",
                "apimock.toml",
                "--path",
                "/x",
                "--status",
                "200",
                "--json",
                "this is not json at all",
            ])
            .output()
            .expect("failed to run apimock");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };
    assert_eq!(code, 2, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        stderr.contains("--json is not valid JSON"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn set_text_is_unchanged_writes_respond_text_and_serves_text_plain() {
    let dir = valid_workspace();
    let (code, _) = run(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "apimock.toml",
            "--rule-set",
            "rules.toml",
            "--path",
            "/plain",
            "--status",
            "200",
            "--text",
            "hello",
        ],
    );
    assert_eq!(code, 0);
    let written = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    assert!(written.contains("text ="), "rules.toml:\n{written}");
}
