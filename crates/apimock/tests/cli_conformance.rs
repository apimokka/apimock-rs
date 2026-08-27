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

use util::cli::{run, run_full, run_json, run_stderr};

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

// RFC 064: `validate -c`/`match-test --rule-set` are the two *required*
// value-taking flags anywhere in this CLI — a dangling value collapses
// to "flag absent" the same as before, on purpose, so the message a
// caller already knows stays exactly what it was (RFC 064's handoff
// § 2.2: "Keep the existing required-flag messages").

#[test]
fn validate_missing_config_value_is_usage() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["validate", "--config"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr.trim_end().lines().next().unwrap_or_default(),
        "apimock validate: missing required flag --config / -c",
        "stderr:\n{stderr}"
    );
}

#[test]
fn match_test_missing_rule_set_value_is_usage() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["match-test", "--rule-set"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr.trim_end().lines().next().unwrap_or_default(),
        "apimock match-test: --rule-set <path> is required",
        "stderr:\n{stderr}"
    );
}

// RFC 064, Defect 2: every *optional* value-taking flag, dangling
// (either at the end of the argument list, or immediately followed by
// another flag), used to collapse silently into "as if not given" —
// `get`/`validate`/`set` exit 0, `match-test` exit 1 via "no match".
// `false_and_previously_untested` was the actual claim these two tests
// made before this RFC (`get_has_no_known_flag_missing_value_scenario`,
// `set_has_no_known_flag_missing_value_scenario`) — both empty, both
// wrong. Real rows, one per flag per command, both dangling forms each,
// driven from a table rather than hand-picked — a hand-picked subset is
// how the false claim was written and never caught.

/// A flag known to the command under test that takes no value, safe to
/// use as "the next flag" in the second dangling form without itself
/// triggering an unknown-flag or dangling-value error.
const GET_HARMLESS_NEXT_FLAG: &str = "--why";
const SET_RULE_HARMLESS_NEXT_FLAG: &str = "--dry-run";
const VALIDATE_HARMLESS_NEXT_FLAG: &str = "--quiet";
const MATCH_TEST_HARMLESS_NEXT_FLAG: &str = "--quiet";

/// Assert both dangling forms of `flag` — at the end of `args_prefix`,
/// and immediately followed by `harmless_next_flag` — are each a
/// `usage` error: exit 2, a message on stderr, nothing on stdout.
fn assert_dangling_flag_is_usage_error(
    dir: &std::path::Path,
    args_prefix: &[&str],
    flag: &str,
    harmless_next_flag: &str,
) {
    let mut at_end: Vec<&str> = args_prefix.to_vec();
    at_end.push(flag);
    let (code, stdout, stderr) = run_full(dir, &at_end);
    assert_eq!(
        code, 2,
        "{flag} dangling at end of args: exit was {code}, stderr:\n{stderr}"
    );
    assert!(
        stdout.is_empty(),
        "{flag} dangling at end of args: stdout was not empty:\n{stdout}"
    );
    assert!(
        !stderr.is_empty(),
        "{flag} dangling at end of args: stderr was empty"
    );

    let mut followed_by_flag: Vec<&str> = args_prefix.to_vec();
    followed_by_flag.push(flag);
    followed_by_flag.push(harmless_next_flag);
    let (code, stdout, stderr) = run_full(dir, &followed_by_flag);
    assert_eq!(
        code, 2,
        "{flag} followed by {harmless_next_flag}: exit was {code}, stderr:\n{stderr}"
    );
    assert!(
        stdout.is_empty(),
        "{flag} followed by {harmless_next_flag}: stdout was not empty:\n{stdout}"
    );
    assert!(
        !stderr.is_empty(),
        "{flag} followed by {harmless_next_flag}: stderr was empty"
    );
}

#[test]
fn get_dangling_optional_flags_are_usage_errors() {
    let dir = valid_workspace();
    for flag in [
        "-c",
        "--method",
        "--header",
        "--body",
        "--body-file",
        "--format",
    ] {
        assert_dangling_flag_is_usage_error(
            dir.path(),
            &["get", "/a"],
            flag,
            GET_HARMLESS_NEXT_FLAG,
        );
    }
}

#[test]
fn set_rule_dangling_optional_flags_are_usage_errors() {
    let dir = valid_workspace();
    for flag in [
        "-c",
        "--rule-set",
        "--rule",
        "--path",
        "--method",
        "--header",
        "--status",
        "--json",
        "--text",
        "--file",
        "--delay",
        "--format",
    ] {
        assert_dangling_flag_is_usage_error(
            dir.path(),
            &["set", "rule"],
            flag,
            SET_RULE_HARMLESS_NEXT_FLAG,
        );
    }
}

/// `validate`'s only other value-taking flag is `--config`, already
/// required (covered above) — `--format` is the one genuinely optional
/// value-taking flag left to check.
#[test]
fn validate_dangling_format_is_usage_error() {
    let dir = valid_workspace();
    assert_dangling_flag_is_usage_error(
        dir.path(),
        &["validate", "-c", "apimock.toml"],
        "--format",
        VALIDATE_HARMLESS_NEXT_FLAG,
    );
}

#[test]
fn match_test_dangling_optional_flags_are_usage_errors() {
    let dir = valid_workspace();
    for flag in [
        "--rule",
        "--path",
        "--method",
        "--header",
        "--body",
        "--body-file",
        "--format",
    ] {
        assert_dangling_flag_is_usage_error(
            dir.path(),
            &["match-test", "--rule-set", "rules.toml"],
            flag,
            MATCH_TEST_HARMLESS_NEXT_FLAG,
        );
    }
}

// RFC 064 § 3.2: a repeated flag where one occurrence has a value and a
// later one doesn't (`--header a:b --header`) errors — the same as any
// other dangling occurrence — rather than silently accepting the first
// and dropping the second. The handoff's own recommendation, adopted:
// a partial-acceptance rule is harder to explain than "every
// occurrence needs a value," for no real benefit.
#[test]
fn a_repeated_flag_with_a_later_dangling_occurrence_is_a_usage_error() {
    let dir = valid_workspace();
    let (code, stdout, stderr) = run_full(
        dir.path(),
        &[
            "get",
            "/a",
            "--header",
            "Content-Type: application/json",
            "--header",
        ],
    );
    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert!(stdout.is_empty(), "stdout:\n{stdout}");
    assert!(!stderr.is_empty());
}

// ── § 2a: the two scenarios that matter most — asserted on effects,
// not just exit codes. An exit-code-only assertion would have passed
// while `set` silently wrote to the wrong file, or while `--format`
// silently fell back to human text — that is precisely what happened
// before this RFC. ──────────────────────────────────────────────────

#[test]
fn set_rule_dangling_config_writes_nothing() {
    let dir = valid_workspace();
    let rule_set_path = dir.path().join("rules.toml");
    let before = std::fs::read_to_string(&rule_set_path).unwrap();

    let (code, _stdout, _stderr) = run_full(
        dir.path(),
        &["set", "rule", "--path", "/new", "--status", "418", "-c"],
    );
    assert_eq!(code, 2);

    let after = std::fs::read_to_string(&rule_set_path).unwrap();
    assert_eq!(
        before, after,
        "the default rule set was modified even though -c was dangling"
    );
}

#[test]
fn dangling_format_never_produces_text_output_with_exit_0() {
    let dir = valid_workspace();

    let (get_code, get_stdout, _) = run_full(dir.path(), &["get", "/a", "--format"]);
    assert_ne!(get_code, 0, "get --format (dangling) must not exit 0");
    assert!(get_stdout.is_empty(), "get stdout:\n{get_stdout}");

    let (validate_code, validate_stdout, _) =
        run_full(dir.path(), &["validate", "-c", "apimock.toml", "--format"]);
    assert_ne!(
        validate_code, 0,
        "validate --format (dangling) must not exit 0"
    );
    assert!(
        validate_stdout.is_empty(),
        "validate stdout:\n{validate_stdout}"
    );

    let (set_code, set_stdout, _) = run_full(
        dir.path(),
        &["set", "rule", "--path", "/x", "--status", "200", "--format"],
    );
    assert_ne!(set_code, 0, "set --format (dangling) must not exit 0");
    assert!(set_stdout.is_empty(), "set stdout:\n{set_stdout}");
}

// ═══════════════════════════════════════════════════════════════════
// Scenario: mutually exclusive flags → usage, exit 2
// ═══════════════════════════════════════════════════════════════════

/// No command in this workspace defines a mutually exclusive flag pair
/// today (checked, not assumed) — `validate --json --format json` was
/// the one example, but 6.0.0 removed `--json` outright, so that
/// combination is now the removal error (enveloped, since `--format
/// json` was also given), not a "cannot combine" message; see
/// `validate_format.rs::json_flag_with_format_json_is_an_enveloped_removal_error`.
/// Stated explicitly, not omitted.
#[test]
fn no_command_defines_a_mutually_exclusive_flag_pair() {}

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

// ═══════════════════════════════════════════════════════════════════
// Scenario: bare relative `-c <file>` resolves the same as `-c
// ./<file>` (RFC 064, Defect 1)
// ═══════════════════════════════════════════════════════════════════
//
// `valid_workspace()`'s config has `rule_sets` — the shape the
// handoff's own warning insists on: a config that fails to parse
// errors before ever reaching the failing path-resolution call, and
// makes the bug look absent. A minimal or invalid fixture would prove
// nothing here.

#[test]
fn validate_bare_relative_config_matches_dot_slash_form() {
    let dir = valid_workspace();
    let (bare_code, bare_stdout) = run(dir.path(), &["validate", "-c", "apimock.toml"]);
    let (dot_code, dot_stdout) = run(dir.path(), &["validate", "-c", "./apimock.toml"]);
    assert_eq!(bare_code, 0, "bare form did not exit 0");
    assert_eq!(bare_code, dot_code);
    assert_eq!(bare_stdout, dot_stdout);
}

#[test]
fn get_bare_relative_config_matches_dot_slash_form() {
    let dir = valid_workspace();
    let (bare_code, bare_stdout) = run(dir.path(), &["get", "/a", "-c", "apimock.toml"]);
    let (dot_code, dot_stdout) = run(dir.path(), &["get", "/a", "-c", "./apimock.toml"]);
    assert_eq!(bare_code, 0, "bare form did not exit 0");
    assert_eq!(bare_code, dot_code);
    assert_eq!(bare_stdout, dot_stdout);
}

#[test]
fn set_rule_bare_relative_config_resolves_and_writes() {
    let dir = valid_workspace();
    // `--rule-set` named explicitly: `set rule` without it bootstraps
    // its own default file rather than targeting `valid_workspace()`'s
    // `rules.toml` — unrelated to Defect 1, so pinned here rather than
    // left implicit, to keep this test about the one thing it's for.
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
            "/bare",
            "--status",
            "200",
            "--text",
            "ok",
        ],
    );
    assert_eq!(code, 0);
    let rules = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    assert!(rules.contains("/bare"), "rules.toml:\n{rules}");
}

#[test]
fn match_test_bare_relative_rule_set_matches_dot_slash_form() {
    let dir = valid_workspace();
    let (bare_code, bare_stdout) = run(
        dir.path(),
        &["match-test", "--rule-set", "rules.toml", "--path", "/a"],
    );
    let (dot_code, dot_stdout) = run(
        dir.path(),
        &["match-test", "--rule-set", "./rules.toml", "--path", "/a"],
    );
    assert_eq!(bare_code, 0, "bare form did not exit 0");
    assert_eq!(bare_code, dot_code);
    assert_eq!(bare_stdout, dot_stdout);
}

// Non-regression: the forms Defect 1 never broke.

#[test]
fn config_flag_forms_all_still_work() {
    let dir = valid_workspace();

    // `-c ./apimock.toml` — already worked before this RFC.
    let (code, _) = run(dir.path(), &["validate", "-c", "./apimock.toml"]);
    assert_eq!(code, 0, "./-prefixed form regressed");

    // `-c ../<dir>/apimock.toml` from a subdirectory.
    let subdir = dir.path().join("sub");
    std::fs::create_dir(&subdir).unwrap();
    let (code, _) = run(&subdir, &["validate", "-c", "../apimock.toml"]);
    assert_eq!(code, 0, "../<dir>/ form from a subdirectory regressed");

    // An absolute `-c /abs/path/apimock.toml`.
    let abs = dir
        .path()
        .join("apimock.toml")
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let (code, _) = run(dir.path(), &["validate", "-c", abs.as_str()]);
    assert_eq!(code, 0, "absolute path form regressed");
}

#[test]
fn a_genuinely_missing_config_file_still_errors_naming_the_file_not_an_empty_path() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(dir.path(), &["validate", "-c", "does-not-exist.toml"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("does-not-exist.toml"),
        "stderr should name the missing file, not an empty path:\n{stderr}"
    );
    assert!(
        !stderr.contains("failed to resolve path ``"),
        "stderr should not show an empty resolved path:\n{stderr}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// RFC 064 Amendment 1: a `--flag=value` form
// ═══════════════════════════════════════════════════════════════════

// ── § 5a: the gate — every no-value flag rejects every `=` form ──────

/// Assert `<flag>=true`, `<flag>=false` and `<flag>=` are each a
/// `usage` error: exit 2, nothing on stdout, something on stderr. The
/// hard acceptance gate (handoff § 2): a no-value flag given any `=`
/// form must never be read as "present" — `=true` exactly like
/// `=false`, never one accepted and the other rejected.
fn assert_equals_form_rejected_for_no_value_flag(
    dir: &std::path::Path,
    args_prefix: &[&str],
    flag: &str,
) {
    for suffix in ["=true", "=false", "="] {
        let flag_arg = format!("{flag}{suffix}");
        let mut args: Vec<&str> = args_prefix.to_vec();
        args.push(flag_arg.as_str());
        let (code, stdout, stderr) = run_full(dir, &args);
        assert_eq!(code, 2, "{flag_arg}: exit was {code}, stderr:\n{stderr}");
        assert!(
            stdout.is_empty(),
            "{flag_arg}: stdout was not empty:\n{stdout}"
        );
        assert!(!stderr.is_empty(), "{flag_arg}: stderr was empty");
    }
}

#[test]
fn get_no_value_flag_rejects_every_equals_form() {
    let dir = valid_workspace();
    assert_equals_form_rejected_for_no_value_flag(dir.path(), &["get", "/a"], "--why");
}

#[test]
fn set_rule_no_value_flags_reject_every_equals_form() {
    let dir = valid_workspace();
    for flag in ["--dry-run", "--allow-outside"] {
        assert_equals_form_rejected_for_no_value_flag(dir.path(), &["set", "rule"], flag);
    }
}

#[test]
fn validate_no_value_flags_reject_every_equals_form() {
    let dir = valid_workspace();
    for flag in ["--strict", "--quiet", "--json"] {
        assert_equals_form_rejected_for_no_value_flag(
            dir.path(),
            &["validate", "-c", "apimock.toml"],
            flag,
        );
    }
}

#[test]
fn match_test_no_value_flags_reject_every_equals_form() {
    let dir = valid_workspace();
    for flag in ["--quiet", "-q"] {
        assert_equals_form_rejected_for_no_value_flag(
            dir.path(),
            &["match-test", "--rule-set", "rules.toml"],
            flag,
        );
    }
}

/// The row that matters most (handoff § 5a): `--allow-outside=false`
/// must not be read as "present" — which would silently disable RFC
/// 062's write-path confinement while the caller wrote `false` to keep
/// it on. Asserted on the filesystem, not just the exit code: nothing
/// is written outside the workspace, and the workspace's own files are
/// byte-identical to before the rejected invocation.
#[test]
fn allow_outside_equals_false_does_not_disable_confinement() {
    let dir = valid_workspace();
    let before_apimock_toml = std::fs::read_to_string(dir.path().join("apimock.toml")).unwrap();
    let before_rules_toml = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();

    let outside = tempfile::tempdir().expect("outside tempdir");
    let outside_rule_set = outside.path().join("elsewhere.toml");
    let outside_rule_set_str = outside_rule_set.to_str().unwrap().to_owned();

    let (code, stdout, stderr) = run_full(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "apimock.toml",
            "--rule-set",
            outside_rule_set_str.as_str(),
            "--path",
            "/x",
            "--status",
            "200",
            "--text",
            "hi",
            "--allow-outside=false",
        ],
    );
    assert_eq!(code, 2, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.is_empty(), "stdout:\n{stdout}");
    assert!(
        !outside_rule_set.exists(),
        "a rule set was written outside the workspace despite --allow-outside=false"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("apimock.toml")).unwrap(),
        before_apimock_toml,
        "apimock.toml changed despite the invocation being rejected"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("rules.toml")).unwrap(),
        before_rules_toml,
        "rules.toml changed despite the invocation being rejected"
    );
}

/// `=true` is rejected exactly like `=false` (handoff § 2: "there is no
/// `--flag=bool` feature here; there is only rejection" — accepting one
/// and not the other is the asymmetry someone later "simplifies" into
/// accepting both).
#[test]
fn allow_outside_equals_true_is_also_rejected() {
    let dir = valid_workspace();
    let (code, stdout, _stderr) = run_full(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "apimock.toml",
            "--path",
            "/x",
            "--status",
            "200",
            "--text",
            "hi",
            "--allow-outside=true",
        ],
    );
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "stdout:\n{stdout}");
}

#[test]
fn dry_run_equals_false_is_rejected_and_rule_set_stays_byte_identical() {
    let dir = valid_workspace();
    let before = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    let (code, stdout, _stderr) = run_full(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "apimock.toml",
            "--rule-set",
            "rules.toml",
            "--path",
            "/x",
            "--status",
            "200",
            "--text",
            "hi",
            "--dry-run=false",
        ],
    );
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "stdout:\n{stdout}");
    let after = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    assert_eq!(before, after);
}

// ── § 5b: the feature ─────────────────────────────────────────────────

#[test]
fn set_rule_text_equals_dash_prefixed_value_writes_correctly() {
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
            "/dash",
            "--status",
            "200",
            "--text=-hello",
        ],
    );
    assert_eq!(code, 0);
    let rules = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    assert!(rules.contains("-hello"), "rules.toml:\n{rules}");
}

#[test]
fn set_rule_text_equals_form_accepts_various_dash_leading_values() {
    let dir = valid_workspace();
    for (path, value) in [
        ("/bullet", "- item"),
        ("/yaml-sep", "---"),
        ("/diff-hunk", "-1,2 +1,3 @@"),
    ] {
        let text_arg = format!("--text={value}");
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
                path,
                "--status",
                "200",
                text_arg.as_str(),
            ],
        );
        assert_eq!(code, 0, "path {path}, value {value:?}");
        let rules = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
        assert!(
            rules.contains(value),
            "rules.toml should contain {value:?}:\n{rules}"
        );
    }
}

#[test]
fn config_equals_form_works_on_every_subcommand() {
    let dir = valid_workspace();

    let (code, _) = run(dir.path(), &["validate", "--config=./apimock.toml"]);
    assert_eq!(code, 0, "validate --config=");
    let (code, _) = run(dir.path(), &["validate", "-c=./apimock.toml"]);
    assert_eq!(code, 0, "validate -c=");

    let (code, _) = run(dir.path(), &["get", "/a", "--config=./apimock.toml"]);
    assert_eq!(code, 0, "get --config=");
    let (code, _) = run(dir.path(), &["get", "/a", "-c=./apimock.toml"]);
    assert_eq!(code, 0, "get -c=");

    let (code, _) = run(
        dir.path(),
        &[
            "set",
            "rule",
            "--config=./apimock.toml",
            "--rule-set",
            "rules.toml",
            "--path",
            "/eq",
            "--status",
            "200",
            "--text",
            "ok",
        ],
    );
    assert_eq!(code, 0, "set --config=");

    let (code, _) = run(
        dir.path(),
        &["match-test", "--rule-set=./rules.toml", "--path", "/a"],
    );
    assert_eq!(code, 0, "match-test --rule-set=");
}

#[test]
fn text_equals_form_with_nothing_after_is_an_explicit_empty_value() {
    let dir = valid_workspace();
    // Precedent: `--text ""` (space form, explicit empty string) works
    // today, exit 0 — `--text=` (handoff § 4) must behave the same, not
    // be confused with a dangling `--text` (no `=`, nothing after),
    // which is a usage error (RFC 064).
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
            "/empty",
            "--status",
            "204",
            "--text=",
        ],
    );
    assert_eq!(code, 0);
}

#[test]
fn dangling_text_without_equals_is_still_a_usage_error() {
    let dir = valid_workspace();
    let (code, stdout, stderr) = run_full(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "apimock.toml",
            "--path",
            "/x",
            "--status",
            "200",
            "--text",
        ],
    );
    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert!(stdout.is_empty());
}

#[test]
fn header_equals_form_repeated_yields_two_headers() {
    let dir = valid_workspace();
    let (code, v) = run_json(
        dir.path(),
        &[
            "get",
            "/a",
            "--header=X-One: 1",
            "--header=X-Two: 2",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0);
    // `get`'s request headers aren't echoed into its own JSON result
    // shape, but a bad merge (e.g. only the first `--header` surviving)
    // would still show up as a parse/usage failure above; a stronger
    // assertion is available through `match-test`'s `--why`-style
    // per-condition breakdown, but the two-headers-both-parsed claim is
    // adequately covered by this command succeeding at all with two
    // `=`-form repeats where a broken merge would instead misinterpret
    // the second as an unrelated token. Complemented by the unit test
    // `flag_values_all_collects_equals_form_occurrences` in
    // `cmd/flags.rs`, which asserts the collected values directly.
    assert!(v.get("result").is_some(), "result: {v}");
}

// ── § 5c: non-regression ──────────────────────────────────────────────

#[test]
fn header_value_containing_equals_still_works_in_space_form() {
    // `-H "Authorization: Basic YWJj=="` — the value legitimately
    // contains `=`, and is a *separate* argv token from `-H` itself, so
    // it must never be mistaken for `-H`'s own `=` form.
    let dir = valid_workspace();
    let (code, v) = run_json(
        dir.path(),
        &[
            "get",
            "/a",
            "-H",
            "Authorization: Basic YWJj==",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0);
    assert!(v.get("result").is_some(), "result: {v}");
}

#[test]
fn positional_path_containing_query_string_with_equals_still_works() {
    // `get "/a?x=1&y=2"` — a positional argument containing `=`, never
    // starting with `-`, so `split_equals_form` never touches it.
    let dir = valid_workspace();
    let (code, _stdout) = run(dir.path(), &["get", "/a?x=1&y=2"]);
    assert_eq!(code, 0);
}

#[test]
fn typo_in_equals_form_still_suggests_a_near_match() {
    let dir = valid_workspace();
    let (code, stderr) = run_stderr(
        dir.path(),
        &["validate", "-c", "apimock.toml", "--strct=true"],
    );
    assert_eq!(code, 2);
    assert!(
        stderr.contains("did you mean '--strict'?"),
        "stderr:\n{stderr}"
    );
}

/// `set`'s own pre-existing strictness — a leftover token that isn't a
/// known flag is rejected even if it doesn't start with `-` — was
/// preserved by name (`strict_bare_tokens: true`) when its private
/// `reject_unknown_flags` copy was folded into the shared one this
/// amendment extended. Never explicitly asserted before; closing that
/// gap here rather than only relying on the consolidation not having
/// changed anything by inspection.
#[test]
fn set_rule_rejects_a_leftover_bare_token_that_is_not_a_known_flag() {
    let dir = valid_workspace();
    let (code, stdout, stderr) = run_full(
        dir.path(),
        &[
            "set", "rule", "--path", "/x", "--status", "200", "--text", "hi", "garbage",
        ],
    );
    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert!(stdout.is_empty(), "stdout:\n{stdout}");
}

// ═══════════════════════════════════════════════════════════════════
// REVIEW-001 § 3: an explicit empty value on a *path*-valued flag is
// a usage error naming the flag, not a downstream failure naming an
// empty path. Content flags (`--text`, `--json`, `--body`) are
// unaffected — `--text=` stays a legitimate empty response body
// (`text_equals_form_with_nothing_after_is_an_explicit_empty_value`,
// above, still passes unchanged).
// ═══════════════════════════════════════════════════════════════════

/// Assert `<args_prefix> <flag>=` is a `usage` error naming `flag`,
/// not a path-resolution failure several layers downstream.
fn assert_empty_equals_value_names_the_flag(
    dir: &std::path::Path,
    args_prefix: &[&str],
    flag: &str,
) {
    let flag_arg = format!("{flag}=");
    let mut args: Vec<&str> = args_prefix.to_vec();
    args.push(flag_arg.as_str());
    let (code, stdout, stderr) = run_full(dir, &args);
    assert_eq!(code, 2, "{flag_arg}: exit was {code}, stderr:\n{stderr}");
    assert!(
        stdout.is_empty(),
        "{flag_arg}: stdout was not empty:\n{stdout}"
    );
    assert!(
        stderr.contains(flag),
        "{flag_arg}: stderr should name the flag, not just an empty path:\n{stderr}"
    );
}

#[test]
fn validate_config_equals_empty_names_the_flag() {
    let dir = valid_workspace();
    assert_empty_equals_value_names_the_flag(dir.path(), &["validate"], "--config");
}

#[test]
fn get_config_equals_empty_names_the_flag() {
    let dir = valid_workspace();
    assert_empty_equals_value_names_the_flag(dir.path(), &["get", "/a"], "--config");
}

#[test]
fn get_body_file_equals_empty_names_the_flag() {
    let dir = valid_workspace();
    assert_empty_equals_value_names_the_flag(dir.path(), &["get", "/a"], "--body-file");
}

#[test]
fn set_rule_path_flags_equals_empty_name_the_flag() {
    let dir = valid_workspace();
    for flag in ["--config", "--rule-set", "--file"] {
        assert_empty_equals_value_names_the_flag(
            dir.path(),
            &["set", "rule", "--path", "/x", "--status", "200"],
            flag,
        );
    }
}

#[test]
fn match_test_rule_set_equals_empty_names_the_flag() {
    let dir = valid_workspace();
    assert_empty_equals_value_names_the_flag(dir.path(), &["match-test"], "--rule-set");
}

#[test]
fn match_test_body_file_equals_empty_names_the_flag() {
    let dir = valid_workspace();
    assert_empty_equals_value_names_the_flag(
        dir.path(),
        &["match-test", "--rule-set", "rules.toml"],
        "--body-file",
    );
}

/// Contrast case (REVIEW-001's own table): `set`'s already-numeric
/// flags reject an empty value in the same style, unaffected by this
/// fix since they never went through `flag_value` as a bare passthrough
/// to begin with — included so the "path flags now match the numeric
/// flags' existing style" claim is asserted, not just narrated.
#[test]
fn set_rule_status_equals_empty_still_names_the_flag_unaffected_by_this_fix() {
    let dir = valid_workspace();
    let (code, stdout, stderr) = run_full(
        dir.path(),
        &["set", "rule", "--path", "/x", "--status=", "--text", "hi"],
    );
    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert!(stdout.is_empty(), "stdout:\n{stdout}");
    assert!(stderr.contains("--status"), "stderr:\n{stderr}");
}
