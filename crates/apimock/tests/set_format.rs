//! RFC 057's own evidence beyond the W7 acceptance test
//! (`set_w7_acceptance.rs`): `--dry-run`'s "changes nothing" guarantee,
//! the conflict path, addressing failures, `service.middlewares`
//! staying untouched, and the single highest-risk property this RFC's
//! handoff calls out — no UUID ever reaches `set`'s output, on any
//! path including every error kind.

#[path = "util.rs"]
mod util;

// RFC 059: `bin`/`run`/`run_json`/`run_stderr` used to be defined here —
// this file's own copy was the one the other three duplicated. Now a
// shared harness (`util::cli`, backing `cli_conformance.rs`'s
// cross-command table too); brought into scope under their original
// names so every call site below is unchanged.
use util::cli::{run, run_json, run_stderr};

fn workspace_with_middleware(dir: &std::path::Path) {
    std::fs::write(
        dir.join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nmiddlewares = [\"mw.rhai\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("apimock-rule-set.toml"), "rules = []\n").unwrap();
    std::fs::write(dir.join("mw.rhai"), "fn handle(a, b, c) { () }\n").unwrap();
}

fn sha_bytes(bytes: &[u8]) -> u64 {
    // A cheap, dependency-free checksum is enough here: we only need
    // "did these bytes change at all", not a cryptographic guarantee.
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

// ── C: --dry-run leaves every file byte-identical ──────────────────────

#[test]
fn dry_run_leaves_every_file_byte_identical_and_reports_what_a_real_run_would_do() {
    let dir = tempfile::tempdir().expect("tempdir");
    workspace_with_middleware(dir.path());

    let root_before = std::fs::read(dir.path().join("apimock.toml")).unwrap();
    let rs_before = std::fs::read(dir.path().join("apimock-rule-set.toml")).unwrap();

    let (code, preview) = run_json(
        dir.path(),
        &[
            "set",
            "rule",
            "--path",
            "/x",
            "--status",
            "204",
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0, "{preview}");
    assert_eq!(preview["result"]["dry_run"], true);
    let would_change = preview["result"]["would_change"]
        .as_array()
        .expect("would_change is an array");
    assert!(
        !would_change.is_empty(),
        "a real add should show in preview: {preview}"
    );

    let root_after = std::fs::read(dir.path().join("apimock.toml")).unwrap();
    let rs_after = std::fs::read(dir.path().join("apimock-rule-set.toml")).unwrap();
    assert_eq!(
        sha_bytes(&root_before),
        sha_bytes(&root_after),
        "apimock.toml must be untouched"
    );
    assert_eq!(
        sha_bytes(&rs_before),
        sha_bytes(&rs_after),
        "apimock-rule-set.toml must be untouched"
    );

    // What --dry-run reported must match what a real run then produces.
    let (code, real) = run_json(
        dir.path(),
        &[
            "set", "rule", "--path", "/x", "--status", "204", "--format", "json",
        ],
    );
    assert_eq!(code, 0, "{real}");
    let real_changes = real["result"]["changes"].as_array().expect("changes array");
    assert_eq!(
        would_change.len(),
        real_changes.len(),
        "dry-run's preview must match what the real run then reports"
    );
}

// ── D: the write path — RFC 056's guarantees, re-proved at this surface ──

#[test]
fn a_rule_set_with_comments_survives_a_set() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "# hand-written config\n[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("apimock-rule-set.toml"),
        "# a person's own comment above their rule\n[[rules]]\nwhen.request.url_path = \"/existing\"\nrespond = { text = \"ok\" }\n",
    )
    .unwrap();

    let (code, v) = run_json(
        dir.path(),
        &[
            "set", "rule", "--path", "/new", "--status", "201", "--format", "json",
        ],
    );
    assert_eq!(code, 0, "{v}");

    let after = std::fs::read_to_string(dir.path().join("apimock-rule-set.toml")).unwrap();
    assert!(
        after.contains("# a person's own comment above their rule"),
        "the rule-set file's comment must survive a set:\n{after}"
    );
    assert!(
        after.contains("/existing"),
        "the pre-existing rule must survive:\n{after}"
    );
    assert!(
        after.contains("/new"),
        "the new rule must be present:\n{after}"
    );
}

// A *real* end-to-end conflict needs the target file to change between
// `set`'s own `Workspace::load()` and its `save()` — a window inside a
// single subprocess's run, with nothing in `set` that yields to another
// process during it. Staging that from outside would mean racing a
// second process against a specific instant inside the first one's
// execution: flaky by construction, not a fit for CI. `apimock-config`
// already proves the underlying mechanism end-to-end without that
// problem, from inside the same process
// (`workspace/tests/save.rs::save_refuses_rather_than_overwrites_a_file_changed_on_disk`,
// `::save_reports_a_read_failure_distinctly_from_a_conflict`). What
// `set` adds on top of that — and what's left to prove here — is the
// `SaveError` → `error.kind` mapping: `Conflict` must reach the CLI as
// `"conflict"`, `Read` as `"io"`, not the other way around.
#[test]
fn kind_for_save_error_distinguishes_conflict_from_io() {
    use apimock::cmd::envelope::kind_for_save_error;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.toml");

    let conflict = apimock_config::SaveError::Conflict { path: path.clone() };
    let read = apimock_config::SaveError::Read {
        path,
        source: std::io::Error::other("denied"),
    };

    // Exercised through the public JSON-producing path, the same one
    // `set`'s own error output goes through — not by reaching into
    // `ErrorKind`'s private representation.
    let conflict_json = apimock::cmd::envelope::err(kind_for_save_error(&conflict), "x");
    let read_json = apimock::cmd::envelope::err(kind_for_save_error(&read), "x");
    assert_eq!(conflict_json["error"]["kind"], "conflict");
    assert_eq!(read_json["error"]["kind"], "io");
}

// ── B: addressing failures are clear errors, not panics or silent no-ops ──

#[test]
fn addressing_a_nonexistent_rule_set_for_update_is_a_clear_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("apimock-rule-set.toml"), "rules = []\n").unwrap();

    let (code, v) = run_json(
        dir.path(),
        &[
            "set",
            "rule",
            "--rule-set",
            "nope.toml",
            "--rule",
            "0",
            "--status",
            "200",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "usage");
}

#[test]
fn an_out_of_range_rule_index_is_a_clear_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("apimock-rule-set.toml"),
        "[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n",
    )
    .unwrap();

    let (code, v) = run_json(
        dir.path(),
        &[
            "set",
            "rule",
            "--rule-set",
            "apimock-rule-set.toml",
            "--rule",
            "9",
            "--status",
            "200",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "usage");
}

// ── E: service.middlewares untouched ────────────────────────────────────

#[test]
fn service_middlewares_is_untouched_even_when_entries_already_exist() {
    let dir = tempfile::tempdir().expect("tempdir");
    workspace_with_middleware(dir.path());
    let before = std::fs::read_to_string(dir.path().join("apimock.toml")).unwrap();

    let (code, v) = run_json(
        dir.path(),
        &[
            "set", "rule", "--path", "/x", "--status", "204", "--format", "json",
        ],
    );
    assert_eq!(code, 0, "{v}");

    let after = std::fs::read_to_string(dir.path().join("apimock.toml")).unwrap();
    let middlewares_line_before = before.lines().find(|l| l.contains("middlewares"));
    let middlewares_line_after = after.lines().find(|l| l.contains("middlewares"));
    assert_eq!(
        middlewares_line_before, middlewares_line_after,
        "the middlewares entry must be untouched:\nbefore:\n{before}\nafter:\n{after}"
    );
}

// ── B: `get --why`'s address composes with `set` verbatim ─────────────

#[test]
fn an_address_from_get_why_feeds_set_unmodified() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("apimock-rule-set.toml"),
        "[[rules]]\nwhen.request.url_path = \"/orders\"\nrespond = { text = \"v1\" }\n",
    )
    .unwrap();

    let (code, why) = run_json(dir.path(), &["get", "/orders", "--format", "json"]);
    assert_eq!(code, 0, "{why}");
    let rule_set_file = why["result"]["matched"]["rule_set_file"]
        .as_str()
        .expect("rule_set_file present")
        .to_owned();
    let rule_index = why["result"]["matched"]["rule_index"]
        .as_u64()
        .expect("rule_index present");

    // Feed the address back into `set` verbatim — no editing.
    let (code, v) = run_json(
        dir.path(),
        &[
            "set",
            "rule",
            "--rule-set",
            &rule_set_file,
            "--rule",
            &rule_index.to_string(),
            "--text",
            "v2",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0, "{v}");

    let after = std::fs::read_to_string(dir.path().join("apimock-rule-set.toml")).unwrap();
    assert!(
        after.contains("v2"),
        "the update must have applied:\n{after}"
    );
    assert!(
        !after.contains("\"v1\""),
        "the old value must be gone:\n{after}"
    );
}

// ── B: no UUID appears in any set output, on any path ──────────────────

/// A UUID is 32 hex digits split 8-4-4-4-12 by hyphens. Scan every
/// hyphen-delimited token in `text` (splitting first on whitespace/`"`/`,`
/// so a UUID embedded in a larger JSON string is still isolated) for
/// that shape.
fn assert_no_uuid(label: &str, text: &str) {
    let looks_like_uuid = |s: &str| -> bool {
        let parts: Vec<&str> = s.split('-').collect();
        parts.len() == 5
            && parts[0].len() == 8
            && parts[1].len() == 4
            && parts[2].len() == 4
            && parts[3].len() == 4
            && parts[4].len() == 12
            && parts
                .iter()
                .all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
    };
    for token in text.split(|c: char| c.is_whitespace() || c == '"' || c == ',') {
        assert!(
            !looks_like_uuid(token),
            "{label}: output looks like it contains a UUID: '{token}'\nfull output:\n{text}"
        );
    }
}

#[test]
fn no_uuid_anywhere_in_set_output_success_dry_run_or_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("apimock-rule-set.toml"), "rules = []\n").unwrap();

    // Success path.
    let (_, out) = run(
        dir.path(),
        &[
            "set", "rule", "--path", "/a", "--status", "200", "--format", "json",
        ],
    );
    assert_no_uuid("success", &out);

    // --dry-run path.
    let (_, out) = run(
        dir.path(),
        &[
            "set",
            "rule",
            "--path",
            "/b",
            "--status",
            "200",
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_no_uuid("dry-run", &out);

    // Error path: usage error (bad rule index).
    let (_, out) = run(
        dir.path(),
        &[
            "set",
            "rule",
            "--rule-set",
            "apimock-rule-set.toml",
            "--rule",
            "99",
            "--status",
            "200",
            "--format",
            "json",
        ],
    );
    assert_no_uuid("usage error", &out);

    // Error path: config couldn't be loaded.
    let bad_dir = tempfile::tempdir().unwrap();
    std::fs::write(bad_dir.path().join("apimock.toml"), "not valid toml =====").unwrap();
    let (_, out) = run(
        bad_dir.path(),
        &[
            "set", "rule", "--path", "/x", "--status", "200", "--format", "json",
        ],
    );
    assert_no_uuid("config-load error", &out);
}

// ── REVIEW-001 § 3: a malformed or empty invocation must be a usage
// error and must write nothing — not a degenerate rule that then
// fails to load. ─────────────────────────────────────────────────────

fn dir_is_empty(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir).unwrap().next().is_none()
}

#[test]
fn an_unknown_flag_is_rejected_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");

    let (code, stderr) = run_stderr(dir.path(), &["set", "rule", "--bogus"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(
        stderr.contains("unrecognized argument '--bogus'"),
        "{stderr}"
    );
    assert!(
        dir_is_empty(dir.path()),
        "an unknown flag must write nothing"
    );
}

#[test]
fn a_bare_add_invocation_with_no_respond_content_is_rejected_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");

    // REVIEW-001 § 3's own repro: `set rule --bogus` and bare `set
    // rule` both used to report success and write a degenerate rule.
    let (code, stderr) = run_stderr(dir.path(), &["set", "rule"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("nothing to respond with"), "{stderr}");
    assert!(
        dir_is_empty(dir.path()),
        "an add with nothing to respond with must write nothing"
    );

    // A match condition alone (no respond content) is the same defect:
    // it would still write a rule whose `respond` fails validation.
    let (code, stderr) = run_stderr(dir.path(), &["set", "rule", "--path", "/x"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(dir_is_empty(dir.path()));
}

#[test]
fn a_bare_update_invocation_with_nothing_to_change_is_rejected_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("apimock-rule-set.toml"),
        "[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n",
    )
    .unwrap();
    let before = std::fs::read_to_string(dir.path().join("apimock-rule-set.toml")).unwrap();

    let (code, stderr) = run_stderr(dir.path(), &["set", "rule", "--rule", "0"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("nothing to change"), "{stderr}");
    let after = std::fs::read_to_string(dir.path().join("apimock-rule-set.toml")).unwrap();
    assert_eq!(
        before, after,
        "an update with nothing to change must write nothing"
    );
}

// ── REVIEW-001 § 4: --dry-run never writes, even to bootstrap ─────────

#[test]
fn dry_run_in_an_empty_directory_refuses_rather_than_bootstraps() {
    let dir = tempfile::tempdir().expect("tempdir");

    let (code, v) = run_json(
        dir.path(),
        &[
            "set",
            "rule",
            "--path",
            "/x",
            "--status",
            "200",
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "usage");
    assert!(
        dir_is_empty(dir.path()),
        "--dry-run must not bootstrap a config when none exists"
    );
}

#[test]
fn dry_run_refuses_to_bootstrap_a_new_rule_set_even_with_an_existing_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();

    let (code, v) = run_json(
        dir.path(),
        &[
            "set",
            "rule",
            "--rule-set",
            "new-rules.toml",
            "--path",
            "/x",
            "--status",
            "200",
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "usage");
    assert!(
        !dir.path().join("new-rules.toml").exists(),
        "--dry-run must not create a new rule-set file either"
    );
}

// ── Found while fixing § 3: UpdateRule with no respond flags used to
// wipe the existing rule's respond to all-None (RulePayload.respond
// has no "None = preserve" semantic) — the same failure class,
// reachable via a different invocation shape. ─────────────────────────

#[test]
fn updating_only_the_when_clause_preserves_the_existing_respond() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("apimock-rule-set.toml"),
        "[[rules]]\nwhen.request.url_path = \"/old\"\nrespond = { status = 200, text = \"hi\" }\n",
    )
    .unwrap();

    let (code, v) = run_json(
        dir.path(),
        &[
            "set", "rule", "--rule", "0", "--path", "/new", "--format", "json",
        ],
    );
    assert_eq!(code, 0, "{v}");

    let after = std::fs::read_to_string(dir.path().join("apimock-rule-set.toml")).unwrap();
    assert!(
        after.contains("/new"),
        "the path change must apply:\n{after}"
    );
    assert!(
        after.contains("200") && after.contains("hi"),
        "the untouched respond must survive the update:\n{after}"
    );

    let (code, v) = run_json(
        dir.path(),
        &["validate", "-c", "./apimock.toml", "--format", "json"],
    );
    assert_eq!(code, 0, "the config must still load after the update: {v}");
    assert_eq!(v["result"]["summary"]["errors"], 0);
}

// ── RFC 062: `--rule-set` confined to the config's own directory tree ──

fn workspace_with_empty_rule_sets(dir: &std::path::Path) {
    std::fs::write(
        dir.join("apimock.toml"),
        "[service]\nrule_sets = []\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
}

#[test]
fn rule_set_outside_the_tree_by_relative_path_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workdir = dir.path().join("workdir");
    std::fs::create_dir(&workdir).unwrap();
    workspace_with_empty_rule_sets(&workdir);
    let config_before = std::fs::read_to_string(workdir.join("apimock.toml")).unwrap();

    let (code, stderr) = run_stderr(
        &workdir,
        &[
            "set",
            "rule",
            "--rule-set",
            "../escaped.toml",
            "--path",
            "/x",
            "--status",
            "200",
            "--text",
            "hi",
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    assert!(
        stderr.contains("resolves outside the config directory"),
        "{stderr}"
    );
    assert!(
        !dir.path().join("escaped.toml").exists(),
        "nothing must be written outside the tree"
    );
    let config_after = std::fs::read_to_string(workdir.join("apimock.toml")).unwrap();
    assert_eq!(
        config_before, config_after,
        "nothing new must be written inside the tree either"
    );
    assert_eq!(
        std::fs::read_dir(&workdir).unwrap().count(),
        1,
        "no new file must appear inside the tree"
    );
}

#[test]
fn rule_set_outside_the_tree_by_absolute_path_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    workspace_with_empty_rule_sets(dir.path());
    let outside = tempfile::tempdir().expect("second tempdir, standing in for 'outside'");
    let target = outside.path().join("abs-escape.toml");

    let (code, stderr) = run_stderr(
        dir.path(),
        &[
            "set",
            "rule",
            "--rule-set",
            target.to_str().unwrap(),
            "--path",
            "/x",
            "--status",
            "200",
            "--text",
            "hi",
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    assert!(
        stderr.contains("resolves outside the config directory"),
        "{stderr}"
    );
    assert!(!target.exists(), "nothing must be written outside the tree");
}

#[test]
fn allow_outside_permits_a_rule_set_outside_the_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workdir = dir.path().join("workdir");
    std::fs::create_dir(&workdir).unwrap();
    workspace_with_empty_rule_sets(&workdir);

    let (code, v) = run_json(
        &workdir,
        &[
            "set",
            "rule",
            "--rule-set",
            "../escaped.toml",
            "--allow-outside",
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
    assert_eq!(code, 0, "{v}");
    assert!(
        dir.path().join("escaped.toml").exists(),
        "--allow-outside must permit the write it names"
    );
}

#[test]
fn an_existing_non_rule_set_toml_in_the_tree_is_still_refused_and_unchanged() {
    // RFC 062 § 2's third probe, re-asserted after the change: this is
    // unrelated to confinement (the target is inside the tree) — it's
    // the pre-existing "doesn't parse as a rule set" refusal, and it
    // must behave exactly as before.
    let dir = tempfile::tempdir().expect("tempdir");
    workspace_with_empty_rule_sets(dir.path());
    let not_a_rule_set = "not = \"a rule set\"\n";
    std::fs::write(dir.path().join("not-a-ruleset.toml"), not_a_rule_set).unwrap();

    let (code, stderr) = run_stderr(
        dir.path(),
        &[
            "set",
            "rule",
            "--rule-set",
            "not-a-ruleset.toml",
            "--path",
            "/x",
            "--status",
            "200",
            "--text",
            "hi",
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    let after = std::fs::read_to_string(dir.path().join("not-a-ruleset.toml")).unwrap();
    assert_eq!(
        not_a_rule_set, after,
        "a target that doesn't parse as a rule set must be left untouched"
    );
}

#[test]
fn bootstrapping_in_a_fresh_empty_directory_still_works_under_confinement() {
    // The regression the handoff flags as most likely to bite: a naive
    // confinement check that requires the target to already exist would
    // break `set`'s own bootstrap-on-first-use behaviour.
    let dir = tempfile::tempdir().expect("tempdir");

    let (code, v) = run_json(
        dir.path(),
        &[
            "set", "rule", "--path", "/x", "--status", "200", "--text", "hi", "--format", "json",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert!(dir.path().join("apimock.toml").exists());
    assert!(dir.path().join("apimock-rule-set.toml").exists());
}
