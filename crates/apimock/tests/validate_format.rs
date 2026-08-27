//! RFC 054 → 6.0.0: `apimock validate --json` (removed) vs
//! `--format json` (RFC 053's envelope) vs `--format text` (today's
//! default). Exercises the real compiled binary — the whole point under
//! test is what actually reaches stdout/stderr, and those two streams
//! have to be captured separately to prove `--json`'s removal error
//! stays off stdout.
//!
//! # Why there is no "loads cleanly, but reports diagnostics" fixture
//!
//! There isn't one to build: `Workspace::load` (via `Config::new` ->
//! `ServiceConfig::validate`) hard-rejects every condition that would
//! otherwise become a `Severity::Error` diagnostic in
//! `Workspace::validate()`'s report — the exact same checks run twice,
//! once as a bool gate that aborts loading, once as the diagnostic
//! walker `apimock validate` prints from. And nothing anywhere
//! constructs a `Severity::Warning` or `Severity::Info` diagnostic
//! (confirmed by grep). So `report.diagnostics` is always empty by the
//! time `apimock validate` reaches it through the CLI — the only two
//! reachable outcomes are "loaded, zero diagnostics" and "failed to
//! load". Reported in the review package; not this RFC's job to fix
//! (its Non-goals explicitly protect `validate`'s existing diagnostics
//! behaviour). Tests below cover the states that are actually reachable
//! today, and say so where a requirement can't be demonstrated.

#[path = "util.rs"]
mod util;

// RFC 059: shared harness (`util::cli`, backing `cli_conformance.rs`'s
// cross-command table too) — this file used to define its own `bin()`.
// A handful of tests below still build on `bin()` directly rather than
// `run`/`run_json`/`run_stderr`: they need *both* streams from the same
// invocation (e.g. "stderr carries the warning, stdout doesn't"), which
// none of the single-stream helpers captures.
use util::cli::{bin, run, run_json, run_stderr};

/// A config with zero diagnostics: 1 rule set, 1 rule, a `text` respond
/// (so nothing to validate against the filesystem).
fn clean_config_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/ok\"\nrespond.text = \"ok\"\n",
    )
    .unwrap();
    dir
}

/// A config that fails to *load* — one rule with an empty `respond`
/// (none of `file_path`/`text`/`status` set). `ServiceConfig::validate`
/// rejects this before `Workspace::load` ever returns `Ok`, so this is
/// a `ConfigError::Validation` / exit-2 case, not a diagnostics-array
/// case — see the module doc comment.
fn structurally_invalid_config_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond = {}\n",
    )
    .unwrap();
    dir
}

// ── `--json` is removed (RFC 054 → 6.0.0, RFC 048 § 7) ─────────────────

#[test]
fn json_flag_is_a_removal_error_stdout_stays_empty() {
    let dir = clean_config_dir();
    let with_json = bin()
        .current_dir(dir.path())
        .args(["validate", "--config", "./apimock.toml", "--json"])
        .output()
        .expect("failed to run apimock validate");

    assert_eq!(with_json.status.code(), Some(2));
    assert!(
        with_json.stdout.is_empty(),
        "stdout was: {}",
        String::from_utf8_lossy(&with_json.stdout)
    );

    let stderr = String::from_utf8_lossy(&with_json.stderr);
    assert!(
        stderr.contains("--json was removed in 6.0.0"),
        "stderr was:\n{stderr}"
    );
    assert!(stderr.contains("--format json"), "stderr was:\n{stderr}");
}

/// Unconditional: `--json`'s presence is fatal on its own, regardless of
/// whether the rest of the invocation is otherwise valid — proven here
/// against both a clean and a load-failing config, so the removal error
/// isn't accidentally contingent on `Workspace::load` succeeding.
#[test]
fn json_flag_is_rejected_before_the_config_is_even_loaded() {
    for dir in [clean_config_dir(), structurally_invalid_config_dir()] {
        let (code, stderr) = run_stderr(
            dir.path(),
            &["validate", "--config", "./apimock.toml", "--json"],
        );

        assert_eq!(code, 2);
        assert!(
            stderr.contains("--json was removed in 6.0.0"),
            "stderr was:\n{stderr}"
        );
    }
}

/// `--json` together with `--format json`: the removal error itself is
/// enveloped (RFC 053, `error.kind: "usage"`), rather than plain text a
/// caller who already asked for the envelope couldn't parse.
#[test]
fn json_flag_with_format_json_is_an_enveloped_removal_error() {
    let dir = clean_config_dir();
    let (code, v) = run_json(
        dir.path(),
        &[
            "validate",
            "--config",
            "./apimock.toml",
            "--json",
            "--format",
            "json",
        ],
    );

    assert_eq!(code, 2);
    assert!(v.get("error").is_some(), "v was: {v}");
    assert!(v.get("result").is_none(), "v was: {v}");
    assert_eq!(v["error"]["kind"], "usage");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("--format json"),
        "v was: {v}"
    );
}

// ── `--format json` — RFC 053's envelope ───────────────────────────────

#[test]
fn format_json_emits_a_valid_envelope_on_a_clean_config() {
    let dir = clean_config_dir();
    let output = bin()
        .current_dir(dir.path())
        .args(["validate", "--config", "./apimock.toml", "--format", "json"])
        .output()
        .expect("failed to run apimock validate");

    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stderr.is_empty(),
        "no --json, so no removal error; stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("stdout must be a single parseable JSON value: {e}\nstdout was:\n{stdout}")
    });
    assert!(v.is_object(), "envelope must be an object, not an array");
    assert_eq!(v["schema"], 1);
    assert!(v["apimock"].is_string());
    assert!(v.get("result").is_some(), "v was: {v}");
    assert!(v.get("error").is_none(), "v was: {v}");
    assert_eq!(v["result"]["summary"]["errors"], 0);
    assert_eq!(v["result"]["diagnostics"].as_array().unwrap().len(), 0);
}

#[test]
fn format_json_reports_config_load_failure_as_an_error_envelope() {
    let (code, v) = run_json(
        std::path::Path::new("."),
        &[
            "validate",
            "--config",
            "/nonexistent/apimock.toml",
            "--format",
            "json",
        ],
    );

    assert_eq!(code, 2);
    assert!(v.get("error").is_some(), "v was: {v}");
    assert!(v.get("result").is_none(), "v was: {v}");
    // Missing file -> unreadable, not invalid: distinct from the next test.
    assert_eq!(v["error"]["kind"], "config_unreadable");
}

/// A config that *exists* but fails `ServiceConfig::validate`'s bool
/// gate gets `config_invalid`, not `config_unreadable` — the two
/// `ErrorKind`s this RFC's implementation actually distinguishes,
/// rather than labelling every load failure the same way.
#[test]
fn format_json_distinguishes_invalid_from_unreadable() {
    let dir = structurally_invalid_config_dir();
    let (code, v) = run_json(
        dir.path(),
        &["validate", "--config", "./apimock.toml", "--format", "json"],
    );

    assert_eq!(code, 2);
    assert_eq!(v["error"]["kind"], "config_invalid");
}

// ── `--format text` matches today's default ────────────────────────────

#[test]
fn format_text_matches_the_implicit_default() {
    let dir = clean_config_dir();
    let explicit = bin()
        .current_dir(dir.path())
        .args(["validate", "--config", "./apimock.toml", "--format", "text"])
        .output()
        .expect("failed to run apimock validate");
    let implicit = bin()
        .current_dir(dir.path())
        .args(["validate", "--config", "./apimock.toml"])
        .output()
        .expect("failed to run apimock validate");

    assert_eq!(explicit.status.code(), implicit.status.code());
    assert_eq!(explicit.stdout, implicit.stdout);
    assert_eq!(explicit.stderr, implicit.stderr);
}

// ── Usage errors ────────────────────────────────────────────────────────
//
// `--json` combined with `--format json` is covered above
// (`json_flag_with_format_json_is_an_enveloped_removal_error`) — it's
// the removal error, enveloped, not a separate "cannot combine" message;
// `--json` alone is fatal regardless of what else is on the line.

#[test]
fn invalid_format_value_is_a_usage_error() {
    let dir = clean_config_dir();
    let (code, stderr) = run_stderr(
        dir.path(),
        &["validate", "--config", "./apimock.toml", "--format", "xml"],
    );

    assert_eq!(code, 2);
    assert!(
        stderr.contains("invalid value for --format"),
        "stderr was:\n{stderr}"
    );
}

// ── Exit codes unchanged across --strict / --quiet ─────────────────────

#[test]
fn exit_codes_unchanged_with_strict_and_quiet() {
    let clean = clean_config_dir();
    let invalid = structurally_invalid_config_dir();

    let cases: &[(&std::path::Path, &[&str], i32)] = &[
        (clean.path(), &["--config", "./apimock.toml"], 0),
        (clean.path(), &["--config", "./apimock.toml", "--strict"], 0),
        (clean.path(), &["--config", "./apimock.toml", "--quiet"], 0),
        (invalid.path(), &["--config", "./apimock.toml"], 2),
        (
            invalid.path(),
            &["--config", "./apimock.toml", "--strict"],
            2,
        ),
        (
            invalid.path(),
            &["--config", "./apimock.toml", "--quiet"],
            2,
        ),
    ];

    for (dir, extra_args, expected) in cases {
        let mut args = vec!["validate"];
        args.extend_from_slice(extra_args);
        let (code, _stdout) = run(dir, &args);
        assert_eq!(code, *expected, "args {:?} in {:?}", args, dir);
    }
}
