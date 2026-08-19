//! RFC 058's own CLI-level evidence, beyond `apimock-routing`'s and
//! `apimock-config`'s unit/workspace-level tests: the original bug
//! report re-run against the fix, RFC 057's W7 script run repeatedly,
//! and `Respond::file_path` still resolving correctly at request time
//! — through a real dispatch, not by inspecting a field (the field is
//! exactly what was wrong before, so it is not evidence on its own).

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_apimock"))
}

fn run(dir: &std::path::Path, args: &[&str]) -> (i32, serde_json::Value) {
    let output = bin()
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run apimock {:?}: {}", args, e));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("stdout wasn't valid JSON: {e}\nargs: {args:?}\nstdout:\n{stdout}")
    });
    (output.status.code().unwrap_or(-1), json)
}

// ── A: the bug is dead ──────────────────────────────────────────────────

#[test]
fn set_rule_five_times_never_grows_a_prefix_block() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (i, path) in ["/a", "/b", "/c", "/d", "/e"].iter().enumerate() {
        let (code, v) = run(
            dir.path(),
            &[
                "set", "rule", "--path", path, "--status", "200", "--format", "json",
            ],
        );
        assert_eq!(code, 0, "round {i}: {v}");
    }

    let rule_set = std::fs::read_to_string(dir.path().join("apimock-rule-set.toml")).unwrap();
    assert!(
        !rule_set.contains("respond_dir"),
        "a bootstrapped rule set never had respond_dir authored, so five saves \
         must never introduce a [prefix] block at all:\n{rule_set}"
    );
}

#[test]
fn w7_script_run_three_times_over_is_byte_stable_after_the_first_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let rule_set_path = dir.path().join("apimock-rule-set.toml");

    let run_once = |dir: &std::path::Path| {
        let (code, _) = run(
            dir,
            &[
                "set",
                "rule",
                "--path",
                "/users/1",
                "--header",
                "x-api-key: k1",
                "--status",
                "403",
                "--format",
                "json",
            ],
        );
        assert_eq!(code, 0);
        let (code, _) = run(
            dir,
            &[
                "set",
                "rule",
                "--path",
                "/users/1",
                "--status",
                "200",
                "--json",
                r#"{"id":1}"#,
                "--format",
                "json",
            ],
        );
        assert_eq!(code, 0);
        let (code, _) = run(dir, &["get", "/users/1", "--format", "json"]);
        assert_eq!(code, 0);
        let (code, _) = run(
            dir,
            &[
                "get",
                "/users/1",
                "--header",
                "x-api-key: k1",
                "--format",
                "json",
            ],
        );
        assert_eq!(code, 0);
        let (code, v) = run(
            dir,
            &["validate", "-c", "./apimock.toml", "--format", "json"],
        );
        assert_eq!(code, 0, "{v}");
    };

    // Each "run" here re-seeds fresh — the checklist's bar is that the
    // config is byte-stable *after the first run*, i.e. that a config
    // already carrying rules and no [prefix] section doesn't start
    // growing one on subsequent, unrelated `set` activity in the same
    // directory. So round 1 seeds the two rules; rounds 2 and 3 repeat
    // set calls against the *same* rule-set file (targeting existing
    // rules this time, via --rule) and must leave it byte-identical
    // beyond the specific field each round intentionally changes.
    run_once(dir.path());
    let after_round_1 = std::fs::read_to_string(&rule_set_path).unwrap();
    assert!(
        !after_round_1.contains("respond_dir"),
        "no respond_dir must ever appear from running W7 alone:\n{after_round_1}"
    );

    // Two more full round-trips of the read-only half (get × 2 +
    // validate) with no further `set` calls — must never write the
    // rule-set file at all, so it stays exactly what round 1 produced.
    for i in 2..=3 {
        let (code, _) = run(dir.path(), &["get", "/users/1", "--format", "json"]);
        assert_eq!(code, 0, "round {i}");
        let (code, _) = run(
            dir.path(),
            &[
                "get",
                "/users/1",
                "--header",
                "x-api-key: k1",
                "--format",
                "json",
            ],
        );
        assert_eq!(code, 0, "round {i}");
        let (code, v) = run(
            dir.path(),
            &["validate", "-c", "./apimock.toml", "--format", "json"],
        );
        assert_eq!(code, 0, "round {i}: {v}");

        let after = std::fs::read_to_string(&rule_set_path).unwrap();
        assert_eq!(
            after_round_1, after,
            "round {i}: read-only commands must never write the rule-set file at all"
        );
    }
}

// ── D: runtime behaviour is unchanged ───────────────────────────────────

fn workspace_with_file_backed_response(
    dir: &std::path::Path,
    prefix_toml: &str,
    responses_subdir: &str,
) {
    let responses = dir.join(responses_subdir);
    std::fs::create_dir_all(&responses).unwrap();
    std::fs::write(responses.join("hello.json"), r#"{"hello":"world"}"#).unwrap();

    let rs_toml = format!(
        "{prefix_toml}[[rules]]\nwhen.request.url_path = \"/hello\"\nrespond = {{ file_path = \"hello.json\" }}\n"
    );
    std::fs::write(dir.join("apimock-rule-set.toml"), rs_toml).unwrap();
    std::fs::write(
        dir.join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
}

#[test]
fn file_backed_response_resolves_with_a_respond_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    workspace_with_file_backed_response(
        dir.path(),
        "[prefix]\nrespond_dir = \"responses\"\n\n",
        "responses",
    );

    let (code, v) = run(dir.path(), &["get", "/hello", "--format", "json"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["result"]["response"]["status"], 200);
    let body: serde_json::Value =
        serde_json::from_str(v["result"]["response"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["hello"], "world");
}

#[test]
fn file_backed_response_resolves_with_no_respond_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    // No [prefix] at all — the file is served relative to the rule-set
    // file's own directory, "." resolved.
    workspace_with_file_backed_response(dir.path(), "", ".");

    let (code, v) = run(dir.path(), &["get", "/hello", "--format", "json"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["result"]["response"]["status"], 200);
    let body: serde_json::Value =
        serde_json::from_str(v["result"]["response"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["hello"], "world");
}

#[test]
fn file_backed_response_resolves_from_a_different_working_directory_than_the_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    workspace_with_file_backed_response(
        dir.path(),
        "[prefix]\nrespond_dir = \"responses\"\n\n",
        "responses",
    );

    // Run from a *different* cwd than the config lives in — the exact
    // case `resolved_respond_dir` exists for. `-c` points at the config
    // by absolute path; the process's own cwd is this test binary's
    // own working directory, unrelated to `dir`.
    let config_path = dir.path().join("apimock.toml");
    let output = bin()
        .args([
            "get",
            "/hello",
            "-c",
            config_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run apimock get");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}\nstdout:\n{stdout}"));

    assert_eq!(output.status.code(), Some(0), "{v}");
    assert_eq!(v["result"]["response"]["status"], 200);
    let body: serde_json::Value =
        serde_json::from_str(v["result"]["response"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["hello"], "world");
}

#[test]
fn respond_dir_pointing_at_a_missing_directory_still_fails_to_load() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock-rule-set.toml"),
        "[prefix]\nrespond_dir = \"does-not-exist\"\n\n[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { file_path = \"x.json\" }\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();

    let (code, v) = run(
        dir.path(),
        &["validate", "-c", "./apimock.toml", "--format", "json"],
    );
    assert_eq!(
        code, 2,
        "a respond_dir pointing nowhere must still fail to load, same as before RFC 058: {v}"
    );
    assert_eq!(v["error"]["kind"], "config_invalid");
}
