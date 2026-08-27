//! Verifies `crates/apimock/examples/agent-bootstrap/README.md`.
//!
//! Unlike every other example under this directory, `agent-bootstrap`
//! has no checked-in `apimock.toml`/`apimock-rule-set.toml` — the
//! walkthrough's whole point is that `set` creates them. So this test
//! runs the five-step sequence in a fresh temp directory, the way
//! `set_w7_acceptance.rs` (RFC 057's own W7 acceptance test, which
//! this example is derived from) already does, rather than pointing at
//! a fixed `example_dir()` like the other `tests/examples/*.rs` files.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_apimock"))
}

fn run(dir: &std::path::Path, args: &[&str]) -> (i32, String) {
    let output = bin()
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run apimock {:?}: {}", args, e));
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

#[tokio::test]
async fn the_readme_walkthrough_runs_exactly_as_documented() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Step 1: the specific, header-gated rule — added first so it's
    // tried before the general fallback (FirstMatch is file order).
    let (code, stdout) = run(
        dir.path(),
        &[
            "set",
            "rule",
            "--path",
            "/users/1",
            "--header",
            "x-api-key: k1",
            "--status",
            "403",
        ],
    );
    assert_eq!(code, 0, "step 1: {stdout}");
    assert!(
        stdout.contains("rule set: apimock-rule-set.toml (new rule)"),
        "step 1 stdout: {stdout}"
    );

    // Step 2: the general fallback rule.
    let (code, stdout) = run(
        dir.path(),
        &[
            "set",
            "rule",
            "--path",
            "/users/1",
            "--status",
            "200",
            "--json",
            r#"{"id":1}"#,
        ],
    );
    assert_eq!(code, 0, "step 2: {stdout}");
    assert!(
        stdout.contains("added rule #2 in rule set #1"),
        "step 2 stdout: {stdout}"
    );

    // Step 3: get without the header — the README's own claim is that
    // this answers from the *second* rule, not the first.
    let (code, stdout) = run(dir.path(), &["get", "/users/1"]);
    assert_eq!(code, 0, "step 3: {stdout}");
    assert!(stdout.contains("Status: 200"), "step 3 stdout: {stdout}");
    assert!(
        stdout.contains("content-type: application/json"),
        "step 3 stdout: {stdout}"
    );
    assert!(stdout.contains(r#"{"id":1}"#), "step 3 stdout: {stdout}");
    assert!(
        stdout.contains("Answered: rule set #1, rule #2"),
        "step 3 stdout: {stdout}"
    );

    // Step 4: get with the header — must see the specific rule instead.
    let (code, stdout) = run(
        dir.path(),
        &["get", "/users/1", "--header", "x-api-key: k1"],
    );
    assert_eq!(code, 0, "step 4: {stdout}");
    assert!(stdout.contains("Status: 403"), "step 4 stdout: {stdout}");
    assert!(
        stdout.contains("Answered: rule set #1, rule #1"),
        "step 4 stdout: {stdout}"
    );

    // Step 5: validate — the README's own claim is that a bare
    // relative `-c` (no `./`) resolves correctly (RFC 064).
    let (code, stdout) = run(dir.path(), &["validate", "-c", "apimock.toml"]);
    assert_eq!(code, 0, "step 5: {stdout}");
    assert!(
        stdout.contains("Validation passed (2 rules across 1 rule set(s))."),
        "step 5 stdout: {stdout}"
    );

    // What got written — the README's own final rule-set listing.
    let written = std::fs::read_to_string(dir.path().join("apimock-rule-set.toml")).unwrap();
    assert!(written.contains("status = 403"), "rules.toml:\n{written}");
    assert!(
        written.contains(r#"json = '{"id":1}'"#),
        "rules.toml:\n{written}"
    );
    assert!(written.contains("status = 200"), "rules.toml:\n{written}");
    assert!(
        !written.contains("text ="),
        "the second rule must be respond.json, not respond.text (D1):\n{written}"
    );
}
