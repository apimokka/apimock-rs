//! Shared CLI test harness (RFC 059).
//!
//! # Why this exists
//!
//! Before this, `get_format.rs`, `set_format.rs`, `validate_format.rs`
//! and `args.rs` each drove the compiled binary and captured its output
//! through their own private copy of essentially the same code — some
//! with a `run`/`run_json`/`run_stderr` trio, some with the pattern
//! inlined per test. A cross-command rule (e.g. "every command rejects
//! an unknown flag the same way") had nowhere to live, because there
//! was no shared harness building the assertion against. This module is
//! that shared harness; `cli_conformance.rs` is the cross-command rule
//! it exists to support.

use std::path::Path;
use std::process::Command;

pub fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_apimock"))
}

/// Run `apimock` with `args` from `dir`. Returns the exit code and
/// stdout.
pub fn run(dir: &Path, args: &[&str]) -> (i32, String) {
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

/// Like [`run`], but parses stdout as JSON. Panics with the raw stdout
/// on parse failure, so a broken invocation fails loudly rather than
/// producing a confusing downstream assertion failure.
pub fn run_json(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let (code, stdout) = run(dir, args);
    let json = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("stdout wasn't valid JSON: {e}\nargs: {args:?}\nstdout:\n{stdout}")
    });
    (code, json)
}

/// Argument-parsing failures — including RFC 059's unknown-flag
/// rejection — print plain text to stderr regardless of `--format`,
/// across every command: the rejection happens before any command has
/// parsed far enough to know which format was requested. Use this to
/// assert on that text.
pub fn run_stderr(dir: &Path, args: &[&str]) -> (i32, String) {
    let output = bin()
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run apimock {:?}: {}", args, e));
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}
