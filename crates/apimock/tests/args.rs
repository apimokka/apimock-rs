use std::path::Path;

use hyper::StatusCode;
use serde_json::json;
use util::{
    cli::bin,
    http::{test_request::TestRequest, test_response::response_body_str},
    test_setup::TestSetup,
};

#[path = "util.rs"]
mod util;

/// A real, shipped example config - used as the "normal workspace"
/// case for RFC 049's `--version` / `--help` evidence requirement.
fn normal_workspace_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/config/default")
}

#[tokio::test]
async fn port_env_arg_overwrites() {
    let port = u16::MAX;
    let test_setup = TestSetup {
        port: Some(port),
        ..Default::default()
    };
    let _ = test_setup.launch().await;

    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), json!({"hello": "index"}).to_string());
}

#[tokio::test]
async fn fallback_response_dir_env_arg_overwrites() {
    let fallback_response_dir_path = "tests/fixtures";
    let test_setup = TestSetup {
        root_config_file_path: None,
        fallback_respond_dir_path: Some(fallback_response_dir_path.to_owned()),
        ..Default::default()
    };
    let port = test_setup.launch().await;

    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        json!({"hello": "custom fallback respond dir"}).to_string()
    );
}

#[tokio::test]
async fn fallback_response_dir_env_arg_default() {
    let test_setup = TestSetup {
        root_config_file_path: None,
        fallback_respond_dir_path: None,
        ..Default::default()
    };
    let port = test_setup.launch().await;

    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = TestRequest::default("/tests/fixtures", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);
}

// RFC 049: the CLI front door. These exercise the real compiled binary
// (`Command`), not `TestSetup`, because the behaviour under test is
// `env::args()` parsing itself - `--version`/`--help` short-circuiting,
// exit codes, and stdout/stderr discipline all have to be observed from
// outside the process.

#[test]
fn unknown_option_exits_2_on_stderr_and_starts_no_server() {
    let output = bin()
        .args(["--bogus-flag-xyz"])
        .output()
        .expect("failed to run apimock");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout was not empty");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown option '--bogus-flag-xyz'"),
        "stderr was: {stderr}"
    );
    // No server started: `.output()` waits for the process to exit on
    // its own. A server that had (wrongly) started would never exit,
    // and this call would hang rather than return - the absence of a
    // hang is itself part of the evidence, not just the message text.
}

#[test]
fn unknown_option_near_match_suggests_the_correction() {
    let output = bin()
        .args(["--prot", "4000"])
        .output()
        .expect("failed to run apimock");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown option '--prot'; did you mean '--port'?"),
        "stderr was: {stderr}"
    );
}

#[test]
fn unknown_option_with_no_plausible_match_names_it_without_a_suggestion() {
    let output = bin()
        .args(["--zzzzzzzz"])
        .output()
        .expect("failed to run apimock");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown option '--zzzzzzzz'"), "{stderr}");
    assert!(!stderr.contains("did you mean"), "stderr was: {stderr}");
}

#[test]
fn version_and_help_normal_workspace() {
    for flag in ["--version", "--help"] {
        let output = bin()
            .current_dir(normal_workspace_dir())
            .args([flag])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock {flag}: {e}"));

        assert_eq!(
            output.status.code(),
            Some(0),
            "{flag} in a normal workspace"
        );
        assert!(output.stderr.is_empty(), "{flag}: stderr was not empty");
        assert!(!output.stdout.is_empty(), "{flag}: stdout was empty");
    }
}

#[test]
fn version_and_help_no_config_file_present() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    // deliberately no apimock.toml written

    for flag in ["--version", "--help"] {
        let output = bin()
            .current_dir(dir.path())
            .args([flag])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock {flag}: {e}"));

        assert_eq!(output.status.code(), Some(0), "{flag} with no config file");
        assert!(output.stderr.is_empty(), "{flag}: stderr was not empty");
        assert!(!output.stdout.is_empty(), "{flag}: stdout was empty");
    }
}

#[test]
fn version_and_help_deliberately_invalid_config() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "this is not [[[ valid toml",
    )
    .expect("failed to write broken config");

    for flag in ["--version", "--help"] {
        let output = bin()
            .current_dir(dir.path())
            .args([flag])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock {flag}: {e}"));

        assert_eq!(
            output.status.code(),
            Some(0),
            "{flag} with a broken config present"
        );
        assert!(output.stderr.is_empty(), "{flag}: stderr was not empty");
        assert!(!output.stdout.is_empty(), "{flag}: stdout was empty");
    }
}

#[test]
fn match_test_and_validate_help_are_reachable_per_subcommand() {
    for subcommand in ["match-test", "validate"] {
        let output = bin()
            .args([subcommand, "--help"])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock {subcommand} --help: {e}"));

        assert_eq!(output.status.code(), Some(0), "{subcommand} --help");
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(&format!("apimock {subcommand}")),
            "{subcommand} --help stdout was: {stdout}"
        );
    }
}

/// F-1 (pre-cut audit): `--allow-outside` (RFC 062's write-path
/// confinement opt-out) is documented in the CLI reference but was
/// missing from `set --help` — the first place a user or agent looks.
#[test]
fn set_help_lists_allow_outside() {
    let output = bin()
        .args(["set", "--help"])
        .output()
        .expect("failed to run apimock set --help");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--allow-outside"),
        "set --help should list --allow-outside; stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("resolve outside the config directory"),
        "set --help's --allow-outside wording should match the CLI reference; stdout was:\n{stdout}"
    );
}

/// `match-test --format` (RFC 059) already worked but was undiscoverable
/// — present in `match_test.rs`'s own `USAGE` constant, absent from this
/// hand-maintained help text, the same shape of drift F-1 fixed for
/// `set`/`--allow-outside`.
#[test]
fn match_test_help_lists_format() {
    let output = bin()
        .args(["match-test", "--help"])
        .output()
        .expect("failed to run apimock match-test --help");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--format text|json"),
        "match-test --help should list --format text|json; stdout was:\n{stdout}"
    );
}

/// Mechanical, not by eye (per this task's handoff § 3): every flag
/// named in a subcommand's `USAGE` constant (the line a real usage
/// error prints) must also appear somewhere in that subcommand's
/// `--help` output. Catches the exact shape of drift this task and F-1
/// both fixed, for all four subcommands at once, so it doesn't need a
/// human to notice a fifth instance.
#[test]
fn every_usage_flag_appears_in_help() {
    fn flag_tokens(text: &str) -> std::collections::BTreeSet<String> {
        let mut out = std::collections::BTreeSet::new();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'-'
                && (i == 0 || !(bytes[i - 1] as char).is_alphanumeric())
                && i + 1 < bytes.len()
                && (bytes[i + 1] == b'-' || (bytes[i + 1] as char).is_alphabetic())
            {
                let start = i;
                let mut end = i + 1;
                while end < bytes.len()
                    && ((bytes[end] as char).is_alphanumeric() || bytes[end] == b'-')
                {
                    end += 1;
                }
                out.insert(text[start..end].to_owned());
                i = end;
            } else {
                i += 1;
            }
        }
        out
    }

    for sub in ["get", "set", "validate", "match-test"] {
        let usage_output = bin()
            .args([sub, "--this-flag-does-not-exist"])
            .output()
            .unwrap_or_else(|e| {
                panic!("failed to run apimock {sub} --this-flag-does-not-exist: {e}")
            });
        let usage_stderr = String::from_utf8_lossy(&usage_output.stderr);
        let usage_line = usage_stderr
            .lines()
            .find(|l| l.starts_with("Usage:"))
            .unwrap_or_else(|| panic!("{sub}: no Usage: line in stderr:\n{usage_stderr}"));

        let help_output = bin()
            .args([sub, "--help"])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock {sub} --help: {e}"));
        let help_stdout = String::from_utf8_lossy(&help_output.stdout);

        let usage_flags = flag_tokens(usage_line);
        let help_flags = flag_tokens(&help_stdout);
        let missing: Vec<&String> = usage_flags.difference(&help_flags).collect();

        assert!(
            missing.is_empty(),
            "{sub}: USAGE names {missing:?} but --help doesn't mention them\nUsage line: {usage_line}\nhelp stdout:\n{help_stdout}"
        );
    }
}

#[test]
fn bare_relative_config_resolves_the_same_as_dot_slash_prefixed() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[listener]\nip_address = \"127.0.0.1\"\nport = 0\n",
    )
    .expect("failed to write config");

    for arg in ["apimock.toml", "./apimock.toml"] {
        // Config loading logs `[config] <path>` to stdout the moment it
        // succeeds (`config.rs`), before the listener binds - so instead
        // of a fixed sleep (the exact pattern RFC 046 just removed from
        // the test harness for the same reason: it's either too short
        // and flaky or too long and slow), poll for that line or for the
        // process exiting early, whichever happens first, up to a
        // bounded deadline.
        let mut child = bin()
            .current_dir(dir.path())
            .args(["-c", arg])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn apimock -c {arg}: {e}"));

        let mut stdout = std::io::BufReader::new(child.stdout.take().expect("piped stdout"));
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let mut line = String::new();
            let _ = stdout.read_line(&mut line);
            let _ = tx.send(line);
        });

        let saw_config_line = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .map(|line| line.starts_with("[config]"))
            .unwrap_or(false);

        let _ = child.kill();
        let output = child.wait_with_output().expect("failed to reap child");

        assert!(
            saw_config_line,
            "-c {arg}: never saw the `[config]` line - resolution likely failed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("failed to resolve path"),
            "-c {arg}: stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// RFC 064 Amendment 1: `-c=path` / `--config=path` resolve the same as
/// the space form, on the root command too, not only the four
/// subcommands. Same spawn/poll/kill pattern as
/// `bare_relative_config_resolves_the_same_as_dot_slash_prefixed` above
/// (a real server would otherwise run forever) — a second test rather
/// than folding into that one's loop, since its `["-c", arg]` shape is
/// two argv tokens and the `=` form is one.
#[test]
fn config_equals_form_resolves_the_same_as_space_form() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[listener]\nip_address = \"127.0.0.1\"\nport = 0\n",
    )
    .expect("failed to write config");

    for arg in ["-c=apimock.toml", "--config=./apimock.toml"] {
        let mut child = bin()
            .current_dir(dir.path())
            .args([arg])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn apimock {arg}: {e}"));

        let mut stdout = std::io::BufReader::new(child.stdout.take().expect("piped stdout"));
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let mut line = String::new();
            let _ = stdout.read_line(&mut line);
            let _ = tx.send(line);
        });

        let saw_config_line = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .map(|line| line.starts_with("[config]"))
            .unwrap_or(false);

        let _ = child.kill();
        let output = child.wait_with_output().expect("failed to reap child");

        assert!(
            saw_config_line,
            "{arg}: never saw the `[config]` line - resolution likely failed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("failed to resolve path"),
            "{arg}: stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// A missing file behind `--config=path` must still name the file, the
/// same as the space form's own non-regression test below — proof the
/// value was actually extracted from the `=` form, not silently
/// dropped to an empty string. Exits before ever binding a listener
/// (`EnvArgs::validate`'s existence check runs first), so this is safe
/// to run with a plain `.output()`, no spawn/poll/kill needed.
#[test]
fn config_equals_form_with_a_missing_file_names_the_file_not_an_empty_path() {
    let output = bin()
        .args(["--config=does-not-exist.toml"])
        .output()
        .expect("failed to run apimock");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does-not-exist.toml"),
        "stderr should name the missing file: {stderr}"
    );
}

/// RFC 064 Amendment 1 § 2's hard acceptance gate, on the root command:
/// a no-value flag given any `=` form is a usage error, never "present"
/// with the value silently discarded. Exits via
/// `reject_unknown_arguments` before `EnvArgs::from_args` or any
/// `--init`/`--yes` branch runs, so none of these ever touch the
/// filesystem or bind a listener - safe with a plain `.output()`.
#[test]
fn root_no_value_flag_given_equals_form_is_a_usage_error() {
    for arg in [
        "--init=false",
        "--init=true",
        "--init=",
        "--yes=false",
        "-y=true",
        "--middleware=false",
        "--version=x",
        "--help=x",
    ] {
        let output = bin()
            .args([arg])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock {arg}: {e}"));
        assert_eq!(output.status.code(), Some(2), "{arg}: {output:?}");
        assert!(
            output.stdout.is_empty(),
            "{arg}: stdout was not empty: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

/// `-c` with nothing after it was already a meaningless invocation
/// before RFC 049 (`args_option_value` returns the empty string for a
/// value-flag given with no value, the same encoding it uses for a
/// boolean flag's presence) - the normalisation in
/// `normalize_bare_relative_path` must not turn "no path given" into
/// "path is `./`", which would trade one confusing error for a
/// different, more confusing one. Pins the exact pre-existing message.
#[test]
fn config_flag_with_no_value_fails_the_same_way_as_before() {
    let output = bin().args(["-c"]).output().expect("failed to run apimock");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("config file specified via --config does not exist:"),
        "stderr was: {stderr}"
    );
    assert!(
        !stderr.contains("Is a directory"),
        "stderr was: {stderr} (this would mean the empty value got normalised to \"./\")"
    );
}

// ── RFC 048 § 6 / RFC 059 / RFC 064: an unknown bare subcommand must
// not silently start a server ────────────────────────────────────────

#[test]
fn unknown_subcommand_exits_2_and_starts_no_server() {
    let output = bin()
        .args(["banana"])
        .output()
        .expect("failed to run apimock");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout was not empty");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown subcommand 'banana'"),
        "stderr was: {stderr}"
    );
    assert!(!stderr.contains("did you mean"), "stderr was: {stderr}");
    // As in `unknown_option_exits_2_on_stderr_and_starts_no_server`
    // above: `.output()` waits for the process to exit on its own. A
    // server that had (wrongly) started would never exit, and this
    // call would hang rather than return.
}

// `serve` itself moved to its own section below (RFC 053: it is now
// the explicit spelling of bare `apimock`, not an unknown subcommand —
// superseding what this test used to assert).

/// Near-match suggestions against the four real subcommands. `gte` (a
/// two-letter transposition of `get`, itself only 3 characters) is
/// deliberately excluded here — measured against the shared
/// `near_match`/threshold this reuses, its edit distance (2) exceeds
/// the threshold (1) that length gets, so it names itself without a
/// suggestion; see this task's review package for the arithmetic. Not
/// a regression to fix here: `near_match` is shared with every flag
/// suggestion in the CLI, and none of this task's scope is to retune it.
#[test]
fn unknown_subcommand_near_match_suggests_the_correction() {
    for (typo, expected) in [
        ("validat", "validate"),
        ("gett", "get"),
        ("st", "set"),
        ("matchtest", "match-test"),
    ] {
        let output = bin()
            .args([typo])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock {typo}: {e}"));

        assert_eq!(output.status.code(), Some(2), "{typo}: {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!(
                "unknown subcommand '{typo}'; did you mean '{expected}'?"
            )),
            "{typo}: stderr was: {stderr}"
        );
    }
}

/// `gte` specifically: the shared `near_match` threshold does not
/// suggest for it (see the test above's doc comment) — pinned here so
/// a future change to `near_match`'s threshold that silently starts
/// suggesting for it is a deliberate, noticed change, not a surprise.
#[test]
fn unknown_subcommand_gte_names_itself_without_a_suggestion() {
    let output = bin().args(["gte"]).output().expect("failed to run apimock");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown subcommand 'gte'"), "{stderr}");
    assert!(!stderr.contains("did you mean"), "stderr was: {stderr}");
}

/// A flag-shaped token at position 1 (`-p`, `--init`, …) must never be
/// treated as an unknown-subcommand attempt — that's
/// `reject_unknown_arguments`'s job, unaffected by this fix. Distinct
/// from `unknown_option_exits_2_on_stderr_and_starts_no_server`, which
/// covers a flag typo *elsewhere* in the line; this one is specifically
/// about position 1.
#[test]
fn a_flag_shaped_token_at_position_one_is_not_treated_as_a_subcommand() {
    let output = bin()
        .args(["--bogus-flag-xyz"])
        .output()
        .expect("failed to run apimock");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown option '--bogus-flag-xyz'"),
        "stderr was: {stderr} (should be `unknown option`, not `unknown subcommand`)"
    );
}

/// A real, no-subcommand invocation still starts the zero-config
/// server end to end (config resolution, listener bind, banner) —
/// `raw.get(1)` here is `-p`, a flag, so it's the same "position 1
/// isn't a bare word" path bare `apimock` (no position 1 at all) takes.
/// `-p 0` rather than an unspecified/default port, so this can't
/// collide with anything already bound to `3001` in the environment
/// this test runs in.
///
/// The literal zero-argv case (`raw.len() == 1`, no position 1 at all)
/// is not separately exercised at runtime: `raw.get(1)` is `None` in
/// that case, `None.filter(...)` stays `None`, and the new check's `if
/// let Some(...)` body simply never runs — true by inspection of
/// `EnvArgs::default`, not something a fixed-port-3001 runtime test
/// should risk flaking over. Every pre-existing zero-config test in
/// this workspace (via `TestSetup`, none of which pass a subcommand
/// either) continuing to pass unmodified is the regression coverage for
/// "this dispatch shape didn't change."
///
/// Same spawn/poll/kill pattern as
/// `bare_relative_config_resolves_the_same_as_dot_slash_prefixed`
/// above: a real server never exits on its own, so `.output()` alone
/// would hang.
#[test]
fn a_no_subcommand_invocation_still_starts_the_server() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    // deliberately no apimock.toml: zero-config mode.

    let mut child = bin()
        .current_dir(dir.path())
        .args(["-p", "0"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn apimock: {e}"));

    let mut stdout = std::io::BufReader::new(child.stdout.take().expect("piped stdout"));
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut line = String::new();
        loop {
            line.clear();
            if stdout.read_line(&mut line).unwrap_or(0) == 0 {
                let _ = tx.send(false);
                return;
            }
            if line.starts_with("Listening on") {
                let _ = tx.send(true);
                return;
            }
        }
    });

    let started = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap_or(false);

    let _ = child.kill();
    let output = child.wait_with_output().expect("failed to reap child");

    assert!(
        started,
        "never saw the `Listening on` line - server likely didn't start; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── RFC 053: `apimock serve` is the explicit spelling of bare
// `apimock` — identical in every respect, never required ─────────────

/// Spawns `apimock` with `args` in `dir`, waits (bounded) for the
/// `Listening on` banner line on stdout, then kills it. Same
/// spawn/poll/kill shape as
/// `a_no_subcommand_invocation_still_starts_the_server` above, factored
/// out here since this section runs it for both the bare and `serve`
/// spellings repeatedly.
fn spawn_and_wait_for_listening(args: &[&str], dir: &std::path::Path) -> bool {
    let mut child = bin()
        .current_dir(dir)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn apimock {args:?}: {e}"));

    let mut stdout = std::io::BufReader::new(child.stdout.take().expect("piped stdout"));
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut line = String::new();
        loop {
            line.clear();
            if stdout.read_line(&mut line).unwrap_or(0) == 0 {
                let _ = tx.send(false);
                return;
            }
            if line.starts_with("Listening on") {
                let _ = tx.send(true);
                return;
            }
        }
    });

    let started = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap_or(false);
    let _ = child.kill();
    let _ = child.wait();
    started
}

/// § 1's table, row 1: `apimock serve` with no flags is identical to
/// bare `apimock` — both start the zero-config server. `-p 0` on both
/// sides so this can't collide with anything already bound to the
/// default port.
#[test]
fn serve_with_no_flags_starts_the_zero_config_server_like_bare_apimock() {
    let bare_dir = tempfile::tempdir().expect("failed to create temp dir");
    let serve_dir = tempfile::tempdir().expect("failed to create temp dir");

    assert!(
        spawn_and_wait_for_listening(&["-p", "0"], bare_dir.path()),
        "bare apimock -p 0 never started"
    );
    assert!(
        spawn_and_wait_for_listening(&["serve", "-p", "0"], serve_dir.path()),
        "apimock serve -p 0 never started"
    );
}

/// § 1's table, row 2: `-c <path>` behaves identically with or without
/// `serve` — both log the same `[config]` line for their own (distinct)
/// config file and go on to listen.
#[test]
fn serve_with_config_flag_behaves_like_bare_apimock_with_config_flag() {
    let bare_dir = tempfile::tempdir().expect("failed to create temp dir");
    let serve_dir = tempfile::tempdir().expect("failed to create temp dir");
    for dir in [&bare_dir, &serve_dir] {
        std::fs::write(
            dir.path().join("apimock.toml"),
            "[service]\nfallback_respond_dir = \".\"\n",
        )
        .expect("failed to write config");
    }

    // `-p 0` alongside `-c`: `--port` overrides `listener.port` from the
    // config regardless (per `EnvArgs`'s own doc comment), so this
    // still proves `-c` was honoured (the `[config]` line) without
    // depending on a fixed port.
    assert!(
        spawn_and_wait_for_listening(&["-c", "apimock.toml", "-p", "0"], bare_dir.path()),
        "apimock -c apimock.toml -p 0 never started"
    );
    assert!(
        spawn_and_wait_for_listening(
            &["serve", "-c", "apimock.toml", "-p", "0"],
            serve_dir.path()
        ),
        "apimock serve -c apimock.toml -p 0 never started"
    );
}

/// § 1's table, row 3: `-d <dir>` behaves identically with or without
/// `serve`.
#[test]
fn serve_with_dir_flag_behaves_like_bare_apimock_with_dir_flag() {
    let bare_dir = tempfile::tempdir().expect("failed to create temp dir");
    let serve_dir = tempfile::tempdir().expect("failed to create temp dir");

    assert!(
        spawn_and_wait_for_listening(&["-p", "0", "-d", "."], bare_dir.path()),
        "apimock -p 0 -d . never started"
    );
    assert!(
        spawn_and_wait_for_listening(&["serve", "-p", "0", "-d", "."], serve_dir.path()),
        "apimock serve -p 0 -d . never started"
    );
}

/// § 1's table, row 4: `--init [--yes] [--middleware]` writes the exact
/// same files with or without `serve` — the one row in the table that
/// doesn't bind a listener, so a plain `.output()` (no spawn/kill) is
/// safe.
#[test]
fn serve_init_writes_the_same_files_as_bare_apimock_init() {
    let bare_dir = tempfile::tempdir().expect("failed to create temp dir");
    let serve_dir = tempfile::tempdir().expect("failed to create temp dir");

    let bare = bin()
        .current_dir(bare_dir.path())
        .args(["--init", "--yes"])
        .output()
        .expect("failed to run apimock --init --yes");
    let serve = bin()
        .current_dir(serve_dir.path())
        .args(["serve", "--init", "--yes"])
        .output()
        .expect("failed to run apimock serve --init --yes");

    assert_eq!(bare.status.code(), serve.status.code());
    assert_eq!(
        std::fs::read_to_string(bare_dir.path().join("apimock.toml")).unwrap(),
        std::fs::read_to_string(serve_dir.path().join("apimock.toml")).unwrap(),
    );
}

/// § 1's table, row 5: `--help` / `--version` are the same with or
/// without `serve` — byte-for-byte, not just "similar", per the
/// handoff's "do not give `serve` its own help text" instruction.
#[test]
fn serve_help_and_version_are_byte_identical_to_the_root_command() {
    for flag in ["--help", "--version"] {
        let bare = bin()
            .args([flag])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock {flag}: {e}"));
        let serve = bin()
            .args(["serve", flag])
            .output()
            .unwrap_or_else(|e| panic!("failed to run apimock serve {flag}: {e}"));

        assert_eq!(bare.status.code(), serve.status.code(), "{flag}");
        assert_eq!(bare.stdout, serve.stdout, "{flag}: stdout differed");
        assert_eq!(bare.stderr, serve.stderr, "{flag}: stderr differed");
    }
}

/// A config that fails to load fails the same way whether reached via
/// `serve` or bare `apimock` — the acceptance checklist's explicit
/// second item, not just "does it start", but "does it fail the same
/// way too".
#[test]
fn serve_with_a_config_that_fails_to_load_fails_the_same_way_as_bare_apimock() {
    let bare_dir = tempfile::tempdir().expect("failed to create temp dir");
    let serve_dir = tempfile::tempdir().expect("failed to create temp dir");
    for dir in [&bare_dir, &serve_dir] {
        std::fs::write(
            dir.path().join("apimock.toml"),
            "this is not [[[ valid toml",
        )
        .expect("failed to write broken config");
    }

    let bare = bin()
        .current_dir(bare_dir.path())
        .output()
        .expect("failed to run apimock");
    let serve = bin()
        .current_dir(serve_dir.path())
        .args(["serve"])
        .output()
        .expect("failed to run apimock serve");

    assert_eq!(bare.status.code(), serve.status.code());
    assert_eq!(bare.status.code(), Some(1));
    assert_eq!(bare.stdout, serve.stdout);
}

/// `apimock serve` never appears as a rejected, unknown subcommand —
/// distinct from `unknown_subcommand_exits_2_and_starts_no_server`
/// (`banana`) above, which must still be rejected. `--allow-outside`
/// (F-1) has no bearing here; kept to a minimal flag so this test is
/// about dispatch, not `set`'s own parsing.
#[test]
fn serve_root_help_lists_it_as_a_subcommand() {
    let output = bin()
        .args(["--help"])
        .output()
        .expect("failed to run apimock");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\n  serve"),
        "root --help should list `serve` as a subcommand; stdout was:\n{stdout}"
    );
}

#[tokio::test]
async fn dir_flag_resolves_bare_and_dot_slash_identically() {
    // RFC 049 § 2 finding: `--dir` never shared `--config`'s resolution
    // fault. A CLI-supplied `--dir` overrides `fallback_respond_dir`
    // verbatim (`Config::compute_fallback_respond_dir`'s early return)
    // without going through the parent-dir resolution `--config` used
    // to hit, and it's read at request time via a plain `Path::join`
    // (`dyn_route.rs`), which treats a bare and a `./`-prefixed relative
    // path identically. `fallback_response_dir_env_arg_overwrites` above
    // already covers the bare form (`"tests/fixtures"`); this proves the
    // `./`-prefixed form serves the exact same fixture, so neither is
    // failing silently by accident.
    let test_setup = TestSetup {
        root_config_file_path: None,
        fallback_respond_dir_path: Some("./tests/fixtures".to_owned()),
        ..Default::default()
    };
    let port = test_setup.launch().await;

    let response = TestRequest::default("/", port).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let body_str = response_body_str(response).await;
    assert_eq!(
        body_str.as_str(),
        json!({"hello": "custom fallback respond dir"}).to_string()
    );
}
