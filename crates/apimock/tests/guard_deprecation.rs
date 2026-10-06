//! Task 018 § 2 — a rule-set file containing `[guard]` still loads, and
//! says on stderr that the key is deprecated. Exit code stays 0: a
//! deprecation must not fail a config that works.
//!
//! The warning is asserted against the real binary, because it is written
//! with `eprintln!` and so exists only at the process boundary, not in any
//! return value a library test could inspect.

use std::process::Command;

/// Write a root config and a rule-set file. `rule_set` is the complete text
/// of the rule-set file, so each test controls whether `[guard]` appears.
fn write_workspace(dir: &std::path::Path, rule_set: &str) -> std::path::PathBuf {
    std::fs::write(dir.join("rules.toml"), rule_set).expect("write rule set");
    let config_path = dir.join("apimock.toml");
    std::fs::write(
        &config_path,
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .expect("write root config");
    config_path
}

/// Run `apimock validate` on `config_path` and return the exit code, stdout
/// and stderr, each as text.
fn validate(config_path: &std::path::Path) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_apimock"))
        .args(["validate", "-c", config_path.to_str().unwrap()])
        .output()
        .expect("failed to run apimock validate");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

const RULE: &str = "[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n";

/// The file loads, the warning names the file and the key, and the exit
/// code is 0.
#[test]
fn guard_present_loads_warns_on_stderr_and_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = write_workspace(dir.path(), &format!("[guard]\n{RULE}"));

    let (code, stdout, stderr) = validate(&config);

    assert_eq!(code, 0, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        stderr.contains("apimock: warning: `[guard]` in rule-set file `")
            && stderr.contains("rules.toml`")
            && stderr.contains("has no effect")
            && stderr.contains("will be rejected in a future release"),
        "stderr:\n{stderr}"
    );
}

/// A rule set without `[guard]` produces no guard warning on any stream.
/// Without this, a deprecation that fires on every start would pass the
/// test above unnoticed.
#[test]
fn guard_absent_produces_no_guard_warning() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = write_workspace(dir.path(), RULE);

    let (code, stdout, stderr) = validate(&config);

    assert_eq!(code, 0, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(!stderr.contains("[guard]"), "stderr:\n{stderr}");
    assert!(!stdout.contains("[guard]"), "stdout:\n{stdout}");
}
