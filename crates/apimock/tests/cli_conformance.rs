//! RFC 059: the cross-command conformance table.
//!
//! # Why this file exists
//!
//! `get_format.rs`, `set_format.rs`, `validate_format.rs` and `args.rs`
//! each already tested their own command's own behaviour — but before
//! this RFC, no test anywhere asserted a rule that had to hold *across*
//! all four (the handoff's own § 2: "a rule implemented once and
//! forgotten three times looks locally fine everywhere"). `set` had
//! correct unknown-flag rejection; `get`, `validate` and `match-test`
//! didn't, and nothing failed until someone typed `--strct` in a CI job
//! and watched it exit 0. This file is the mechanism that makes that
//! regression impossible to reintroduce quietly: one command × scenario
//! table, covering every command this RFC's contract applies to.
//!
//! Every row asserts exit code, `error.kind` (where the scenario
//! reaches an envelope), and which stream carries the output — not just
//! the exit code, per the handoff's own acceptance bar. A scenario a
//! command genuinely doesn't have is a `#[test]` that says so and why,
//! not a missing row a reader has to notice on their own.

#[path = "util.rs"]
mod util;

use util::cli::{run_json, run_stderr};

// ── Fixtures ─────────────────────────────────────────────────────────

/// One rule set, one rule, matching `/a` — the "everything is fine"
/// workspace every success-path row below runs against.
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

/// A directory whose `apimock.toml` (or standalone rule-set file, for
/// `match-test`) is present but not valid TOML.
fn malformed_file_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("apimock.toml"), "not valid toml =====\n").unwrap();
    std::fs::write(dir.path().join("rules.toml"), "not valid toml =====\n").unwrap();
    dir
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: unknown flag → usage, exit 2, stderr, near-match suggestion
// ═══════════════════════════════════════════════════════════════════

#[test]
fn get_unknown_flag_is_usage_with_near_match() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["get", "/a", "--whyy"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("did you mean '--why'?"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn validate_unknown_flag_is_usage_with_near_match() {
    // The RFC's own motivating case: `--strct`, a typo of `--strict`,
    // used to exit 0 and print "Validation passed".
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["validate", "-c", "./apimock.toml", "--strct"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("did you mean '--strict'?"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn match_test_unknown_flag_is_usage_with_near_match() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(
        dir.path(),
        &["match-test", "--rule-set", "rules.toml", "--rulee", "1"],
    );
    assert_eq!(code, 2);
    assert!(
        stderr.contains("did you mean '--rule'?"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn set_unknown_flag_is_usage_with_near_match() {
    // `set` already had this (`set_format.rs`'s own
    // `an_unknown_flag_is_rejected_and_writes_nothing`); included here
    // too so this table is genuinely complete for all four commands,
    // not three plus a pointer elsewhere for the fourth.
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["set", "rule", "--statuss", "200"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("did you mean '--status'?"),
        "stderr:\n{stderr}"
    );
}

/// RFC 062's `--allow-outside` opt-out is a known flag `set` recognises,
/// same as every other one above — a typo of it gets the same
/// near-match treatment, proving it's part of the vocabulary this
/// table's mechanism actually covers, not a flag bolted on beside it.
#[test]
fn set_allow_outside_typo_is_usage_with_near_match() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["set", "rule", "--allow-outsid"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("did you mean '--allow-outside'?"),
        "stderr:\n{stderr}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: known flag, missing value → usage, exit 2
// ═══════════════════════════════════════════════════════════════════

#[test]
fn validate_missing_config_value_is_usage() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["validate", "--config"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("missing required flag --config"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn match_test_missing_rule_set_value_is_usage() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["match-test", "--rule-set"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("--rule-set"), "stderr:\n{stderr}");
}

/// `get`'s only required argument is the positional `<path>`, not a
/// value-taking flag — checked (`missing_path_is_a_usage_error` in
/// `get_format.rs` covers the actual missing-argument case); there is
/// no known *flag* whose missing value is independently a usage error.
/// Stated explicitly, not omitted.
#[test]
fn get_has_no_known_flag_missing_value_scenario() {}

/// Every flag `set rule` takes is optional, and a value-taking one
/// given without its value degrades to "as if not given" rather than
/// erroring (e.g. `--rule` with no value behaves like `--rule` absent,
/// switching an update into an add) — a real, separate gap, but not
/// this RFC's unknown-flag scope, and not a scenario this table can
/// assert without inventing new validation `set` doesn't have. Stated
/// explicitly, not omitted.
#[test]
fn set_has_no_known_flag_missing_value_scenario() {}

// ═══════════════════════════════════════════════════════════════════
// Scenario: mutually exclusive flags → usage, exit 2
// ═══════════════════════════════════════════════════════════════════

/// `validate --json --format json` is the one mutually-exclusive flag
/// pair in this workspace today, already exercised end to end by
/// `validate_format.rs::json_and_format_together_is_a_usage_error` — not
/// duplicated here. `get`, `set` and `match-test` define no mutually
/// exclusive flag pair (checked, not assumed): nothing to assert for
/// them under this scenario. Stated explicitly, not omitted.
#[test]
fn only_validate_has_a_mutually_exclusive_flag_pair() {}

// ═══════════════════════════════════════════════════════════════════
// Scenario: config missing → config_unreadable, exit 2
// ═══════════════════════════════════════════════════════════════════

#[test]
fn get_config_missing_is_config_unreadable() {
    let dir = valid_workspace();
    let (code, v) = run_json(
        dir.path(),
        &["get", "/a", "-c", "does-not-exist.toml", "--format", "json"],
    );
    assert_eq!(code, 2);
    assert_eq!(v["error"]["kind"], "config_unreadable");
}

#[test]
fn validate_config_missing_is_config_unreadable() {
    let dir = valid_workspace();
    let (code, v) = run_json(
        dir.path(),
        &["validate", "-c", "does-not-exist.toml", "--format", "json"],
    );
    assert_eq!(code, 2);
    assert_eq!(v["error"]["kind"], "config_unreadable");
}

#[test]
fn match_test_rule_set_missing_is_config_unreadable() {
    let dir = valid_workspace();
    let (code, v) = run_json(
        dir.path(),
        &[
            "match-test",
            "--rule-set",
            "does-not-exist.toml",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 2);
    assert_eq!(v["error"]["kind"], "config_unreadable");
}

/// `set rule` with no config present **bootstraps a starter file
/// instead of erroring** (documented in `set.rs`'s own module doc) —
/// this is `set`'s deliberate zero-config UX, not the failure mode this
/// scenario describes. Genuinely absent, not omitted; `set_format.rs`
/// has its own bootstrap-behaviour coverage.
#[test]
fn set_config_missing_bootstraps_rather_than_erroring() {}

// ═══════════════════════════════════════════════════════════════════
// Scenario: config malformed → config_invalid, exit 2
// ═══════════════════════════════════════════════════════════════════

#[test]
fn get_config_malformed_is_config_invalid() {
    let dir = malformed_file_workspace();
    let (code, v) = run_json(dir.path(), &["get", "/a", "--format", "json"]);
    assert_eq!(code, 2);
    assert_eq!(v["error"]["kind"], "config_invalid");
}

#[test]
fn validate_config_malformed_is_config_invalid() {
    let dir = malformed_file_workspace();
    let (code, v) = run_json(
        dir.path(),
        &["validate", "-c", "./apimock.toml", "--format", "json"],
    );
    assert_eq!(code, 2);
    assert_eq!(v["error"]["kind"], "config_invalid");
}

#[test]
fn match_test_rule_set_malformed_is_config_invalid() {
    let dir = malformed_file_workspace();
    let (code, v) = run_json(
        dir.path(),
        &["match-test", "--rule-set", "rules.toml", "--format", "json"],
    );
    assert_eq!(code, 2);
    assert_eq!(v["error"]["kind"], "config_invalid");
}

#[test]
fn set_config_malformed_is_config_invalid() {
    let dir = malformed_file_workspace();
    let (code, v) = run_json(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "./apimock.toml",
            "--path",
            "/x",
            "--status",
            "200",
            "--text",
            "hi",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 2);
    assert_eq!(v["error"]["kind"], "config_invalid");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: success → exit 0, stdout only
// ═══════════════════════════════════════════════════════════════════

#[test]
fn get_success_is_exit_0_with_a_result_envelope() {
    let dir = valid_workspace();
    let (code, v) = run_json(dir.path(), &["get", "/a", "--format", "json"]);
    assert_eq!(code, 0);
    assert!(v.get("result").is_some(), "v was: {v}");
    assert!(v.get("error").is_none(), "v was: {v}");
}

#[test]
fn validate_success_is_exit_0_with_a_result_envelope() {
    let dir = valid_workspace();
    let (code, v) = run_json(
        dir.path(),
        &["validate", "-c", "./apimock.toml", "--format", "json"],
    );
    assert_eq!(code, 0);
    assert!(v.get("result").is_some(), "v was: {v}");
    assert!(v.get("error").is_none(), "v was: {v}");
}

#[test]
fn match_test_success_is_exit_0_with_a_result_envelope() {
    let dir = valid_workspace();
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
    assert!(v.get("result").is_some(), "v was: {v}");
    assert!(v.get("error").is_none(), "v was: {v}");
    assert_eq!(v["result"]["matched"], true);
}

#[test]
fn set_success_is_exit_0_with_a_result_envelope() {
    let dir = valid_workspace();
    let (code, v) = run_json(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "./apimock.toml",
            "--path",
            "/y",
            "--status",
            "200",
            "--text",
            "hi",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0);
    assert!(v.get("result").is_some(), "v was: {v}");
    assert!(v.get("error").is_none(), "v was: {v}");
}
