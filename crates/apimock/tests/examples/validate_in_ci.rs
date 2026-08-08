//! Verifies `crates/apimock/examples/validate-in-ci/README.md`.
//!
//! `apimock::cmd::validate::run` returns an exit code (safe to call
//! in-process). `apimock::cmd::match_test::run` calls
//! `std::process::exit` internally (see its own doc comment) - calling
//! it in-process would kill the test binary, so `match-test` is
//! exercised via the real compiled binary instead, through
//! `CARGO_BIN_EXE_apimock`.

use std::{path::Path, process::Command};

fn example_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/validate-in-ci")
}

#[tokio::test]
async fn validate_passes_on_a_clean_config() {
    let config_path = example_dir().join("apimock.toml");
    let config_path = config_path
        .to_str()
        .expect("path is valid UTF-8")
        .to_owned();

    let exit_code = apimock::cmd::validate::run(&["--config".to_owned(), config_path]);
    assert_eq!(exit_code, 0);
}

/// `apimock::cmd::validate::run` (above) can't have its stdout
/// captured in-process, so the `--json` shape documented in the
/// README is checked against the real binary instead, the same way
/// `match-test` is below.
///
/// `--config ./apimock.toml`, not the bare filename: a bare relative
/// `--config` fails to resolve even though the file exists (see
/// ESCALATION-003 in the RFC 036 review package) - not a mistake to
/// repeat here.
#[tokio::test]
async fn validate_json_flag_emits_diagnostics_array() {
    let output = Command::new(env!("CARGO_BIN_EXE_apimock"))
        .current_dir(example_dir())
        .args(["validate", "--config", "./apimock.toml", "--json"])
        .output()
        .expect("failed to run apimock validate");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[]"), "stdout was:\n{stdout}");
    assert!(stdout.contains("Validation passed (2 rules across 1 rule set(s))."));
}

#[tokio::test]
async fn match_test_gold_tier_matches_rule_one() {
    let output = Command::new(env!("CARGO_BIN_EXE_apimock"))
        .current_dir(example_dir())
        .args([
            "match-test",
            "--rule-set",
            "apimock-rule-set.toml",
            "--path",
            "/orders",
            "--method",
            "POST",
            "--body",
            r#"{"customer":{"tier":"gold"}}"#,
        ])
        .output()
        .expect("failed to run apimock match-test");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Result: MATCH (rule #1)"),
        "stdout was:\n{stdout}"
    );
}

#[tokio::test]
async fn match_test_silver_tier_falls_through_to_rule_two() {
    let output = Command::new(env!("CARGO_BIN_EXE_apimock"))
        .current_dir(example_dir())
        .args([
            "match-test",
            "--rule-set",
            "apimock-rule-set.toml",
            "--path",
            "/orders",
            "--method",
            "POST",
            "--body",
            r#"{"customer":{"tier":"silver"}}"#,
        ])
        .output()
        .expect("failed to run apimock match-test");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Result: MATCH (rule #2)"),
        "stdout was:\n{stdout}"
    );
}

#[tokio::test]
async fn match_test_get_matches_nothing() {
    let output = Command::new(env!("CARGO_BIN_EXE_apimock"))
        .current_dir(example_dir())
        .args([
            "match-test",
            "--rule-set",
            "apimock-rule-set.toml",
            "--path",
            "/orders",
            "--method",
            "GET",
        ])
        .output()
        .expect("failed to run apimock match-test");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Result: NO MATCH"), "stdout was:\n{stdout}");
}
