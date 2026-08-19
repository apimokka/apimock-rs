//! RFC 057's acceptance test — RFC 048's W7, concretely: a script that
//! runs, non-interactively, in a clean directory, exercising `set` and
//! `get` together. This is the falsification test the RFC names for its
//! own design; everything else `set` does is in service of this.
//!
//! # Two corrections to the RFC's literal script, found by running it
//!
//! RFC 057 § "The acceptance test" writes the two `set rule` calls in
//! this order: the unconditional rule (status 200) first, the
//! header-gated one (status 403) second. Running it that way does not
//! produce the story the script tells. `EditCommand::AddRule` only ever
//! appends (`workspace/edit.rs:288`) — deliberately, so a rule's
//! positional address survives across invocations (RFC 057's handoff
//! § 2 Unresolved 1) — and `Strategy::FirstMatch` picks the first rule
//! in array order whose conditions hold, with no specificity
//! tie-break (`apimock-routing/src/rule_set.rs:181-187`). Appending the
//! unconditional rule first means it answers *every* request to
//! `/users/1`, header or not — the second `get` call in the script
//! would see status 200, not 403.
//!
//! Reordering the two `set rule` calls (specific condition first,
//! general fallback second) produces the intended behaviour using only
//! `AddRule`'s append-only semantics — no reordering command needed.
//! Verified below: the `x-api-key` request gets 403, the plain request
//! gets 200.
//!
//! The RFC's own script also runs `apimock validate` bare, with no
//! `-c`. `validate` has no default-config-path convenience (unlike
//! `get`/`set`) and requires an explicit `--config`/`-c` — confirmed by
//! running it. Its value also needs the `./` prefix: `validate`'s own
//! `-c` doesn't get RFC 049's bare-relative-path normalisation, a
//! pre-existing, already-documented gap (RFC 055's review package § 6).
//! Neither of these is `set`'s to fix; the corrected script below uses
//! `validate -c ./apimock.toml`.
//!
//! RFC 057 § "The acceptance test" says explicitly: "The exact flag
//! spellings are this RFC's to settle; the shape is the commitment."
//! Both corrections keep the shape — two `set rule` calls establishing
//! conditional responses, two `get` calls retrieving them, one
//! `validate` — while making the demonstrated behaviour real.

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

#[test]
fn w7_script_runs_end_to_end_in_a_clean_directory() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Step 1: the specific, header-gated rule — added first so it's
    // tried before the general fallback (FirstMatch is array order).
    let (code, v) = run(
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
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0, "step 1 (set rule, specific): {v}");
    assert!(v.get("result").is_some(), "step 1 must succeed: {v}");

    // Step 2: the general fallback rule.
    let (code, v) = run(
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
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0, "step 2 (set rule, general): {v}");
    assert!(v.get("result").is_some(), "step 2 must succeed: {v}");

    // Step 3: get without the header — must see the general rule.
    let (code, v) = run(dir.path(), &["get", "/users/1", "--format", "json"]);
    assert_eq!(code, 0, "step 3 (get, no header): {v}");
    assert_eq!(v["result"]["response"]["status"], 200);
    assert_eq!(v["result"]["response"]["body"], r#"{"id":1}"#);

    // Step 4: get with the header — must see the specific rule instead.
    let (code, v) = run(
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
    assert_eq!(code, 0, "step 4 (get, with header): {v}");
    assert_eq!(v["result"]["response"]["status"], 403);

    // Step 5: validate the result.
    let (code, v) = run(
        dir.path(),
        &["validate", "-c", "./apimock.toml", "--format", "json"],
    );
    assert_eq!(code, 0, "step 5 (validate): {v}");
    assert_eq!(v["result"]["summary"]["errors"], 0);
    assert_eq!(v["result"]["summary"]["rules"], 2);
}

#[test]
fn w7_script_every_step_exit_code_matches_rfc_049() {
    // The checklist's own bar, isolated from response-content
    // assertions: every step exits 0 (RFC 049's convention — 2 is
    // reserved for a bad invocation or an unloadable config, 1 for
    // everything else after a successful load, neither of which any
    // W7 step should hit).
    let dir = tempfile::tempdir().expect("tempdir");
    let steps: &[&[&str]] = &[
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
        &["get", "/users/1", "--format", "json"],
        &[
            "get",
            "/users/1",
            "--header",
            "x-api-key: k1",
            "--format",
            "json",
        ],
        &["validate", "-c", "./apimock.toml", "--format", "json"],
    ];
    for step in steps {
        let (code, v) = run(dir.path(), step);
        assert_eq!(code, 0, "step {step:?} exited {code}: {v}");
    }
}
