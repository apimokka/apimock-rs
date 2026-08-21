//! RFC 059 § 4: `match-test` joins RFC 053's envelope. Before this RFC,
//! `match-test` had no `--format` support at all — the only one of the
//! four commands outside the contract. Additive: text stays the default
//! and byte-identical to before; `--format json` is new.

#[path = "util.rs"]
mod util;

use util::cli::{run, run_json, run_stderr};

fn workspace_with_two_rules() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"ok\"\n\n\
         [[rules]]\nwhen.request.url_path = \"/b\"\nrespond.text = \"ok\"\n",
    )
    .unwrap();
    dir
}

// ── `--format json` emits a valid envelope ──────────────────────────

#[test]
fn format_json_emits_a_valid_envelope_on_a_match() {
    let dir = workspace_with_two_rules();
    let (code, v) = run_json(
        dir.path(),
        &[
            "match-test",
            "--rule-set",
            "rules.toml",
            "--path",
            "/a",
            "--format",
            "json",
        ],
    );

    assert_eq!(code, 0);
    assert!(v.is_object(), "envelope must be an object, not an array");
    assert_eq!(v["schema"], 1);
    assert!(v["apimock"].is_string());
    assert!(v.get("result").is_some(), "v was: {v}");
    assert!(v.get("error").is_none(), "v was: {v}");

    assert_eq!(v["result"]["matched"], true);
    assert_eq!(v["result"]["match_rule_index"], 0);
    assert_eq!(v["result"]["request"]["path"], "/a");
    assert_eq!(v["result"]["request"]["method"], "GET");
    let rules = v["result"]["rules"].as_array().expect("rules is an array");
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0]["rule_index"], 0);
    assert_eq!(rules[0]["matched"], true);
}

#[test]
fn format_json_emits_a_valid_envelope_on_no_match() {
    let dir = workspace_with_two_rules();
    let (code, v) = run_json(
        dir.path(),
        &[
            "match-test",
            "--rule-set",
            "rules.toml",
            "--path",
            "/nowhere",
            "--format",
            "json",
        ],
    );

    // No match -> exit 1, same as text mode, but still a `result`
    // envelope, not an `error` -- "no rule matched" is a legitimate
    // result, not a failure (the same principle RFC 055 established for
    // `get`'s 404s).
    assert_eq!(code, 1);
    assert!(v.get("result").is_some(), "v was: {v}");
    assert!(v.get("error").is_none(), "v was: {v}");
    assert_eq!(v["result"]["matched"], false);
    assert!(v["result"]["match_rule_index"].is_null());
}

// ── Text stays the default, and byte-identical ──────────────────────

#[test]
fn text_is_the_default_and_format_text_matches_it_exactly() {
    let dir = workspace_with_two_rules();
    let implicit = run(
        dir.path(),
        &["match-test", "--rule-set", "rules.toml", "--path", "/a"],
    );
    let explicit = run(
        dir.path(),
        &[
            "match-test",
            "--rule-set",
            "rules.toml",
            "--path",
            "/a",
            "--format",
            "text",
        ],
    );
    assert_eq!(implicit, explicit);
}

#[test]
fn text_output_shape_is_unchanged() {
    let dir = workspace_with_two_rules();
    let (code, stdout) = run(
        dir.path(),
        &["match-test", "--rule-set", "rules.toml", "--path", "/a"],
    );
    assert_eq!(code, 0);
    // The exact shape `print_rule_result`/the final `Result:` line have
    // always printed — RFC 059 only changed how this is computed
    // (`compute_outcome` then `print_text_outcome`), not what it prints.
    assert!(
        stdout.contains("\nRule #1: /a  MATCH ★"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\nRule #2: /b  NO MATCH"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\nResult: MATCH (rule #1)"),
        "stdout:\n{stdout}"
    );
    // `--format json`'s envelope keys must never leak into text mode.
    assert!(!stdout.contains("\"schema\""), "stdout:\n{stdout}");
}

#[test]
fn quiet_suppresses_text_but_not_the_json_envelope() {
    let dir = workspace_with_two_rules();
    let (code, stdout) = run(
        dir.path(),
        &[
            "match-test",
            "--rule-set",
            "rules.toml",
            "--path",
            "/a",
            "--quiet",
        ],
    );
    assert_eq!(code, 0);
    assert!(stdout.is_empty(), "stdout:\n{stdout}");

    let (code, v) = run_json(
        dir.path(),
        &[
            "match-test",
            "--rule-set",
            "rules.toml",
            "--path",
            "/a",
            "--quiet",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0);
    assert!(v.get("result").is_some(), "v was: {v}");
}

#[test]
fn invalid_format_value_is_a_usage_error() {
    let dir = workspace_with_two_rules();
    let (code, stderr) = run_stderr(
        dir.path(),
        &["match-test", "--rule-set", "rules.toml", "--format", "xml"],
    );
    assert_eq!(code, 2);
    assert!(
        stderr.contains("invalid value for --format"),
        "stderr:\n{stderr}"
    );
}
