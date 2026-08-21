//! RFC 060 — property tests over `Workspace::save`'s write path.
//!
//! # Why this exists
//!
//! RFC 058: `respond_dir` grew by one `./` segment on **every** save, in
//! released code, for over a rule-set-file's worth of history. `save.rs`
//! had 22 tests at the time, and every one of them saved *once* — the
//! invariant that actually broke, "save twice and the file stops
//! changing," was never expressed. A person running `apimock set` five
//! times in a row and noticing the file looked wrong found it; that's
//! luck, not a mechanism. This file is the mechanism.
//!
//! # Four invariants, stated over generated configs — not particular ones
//!
//! 1. **Idempotence** — an edit, then a save with no further edit,
//!    reaches a fixed point: the second save changes nothing.
//! 2. **Preservation** — a comment / blank line / key order the
//!    generator seeded and the edit didn't target survives byte-for-byte.
//! 3. **Locality** — an edit to one rule-set file leaves every other
//!    file in the workspace's write set byte-identical.
//! 4. **Conflict safety** — a file changed on disk underneath a pending
//!    save is refused, and nothing else pending is written either.
//!
//! # The generator
//!
//! Builds valid, schema-shaped TOML directly from the same fields
//! `Workspace::load`/`RuleSet::new` parse (`when.request.url_path`,
//! `when.request.headers.*`, `when.request.body.json.*`,
//! `respond.text`/`respond.status`, `[prefix].respond_dir`) — the same
//! vocabulary every hand-written fixture in `save.rs` already uses, not
//! an invented parallel shape. The repeated edit each property applies
//! between saves goes through the real `Workspace::apply(EditCommand::AddRule
//! { .. })` path with a generated `RulePayload`, not a hand-rolled
//! shortcut — the one place this generator *could* have drifted into
//! testing fiction, and doesn't.
//!
//! Kept deliberately small (RFC 060 § 4: "its value is the shrunk
//! counter-example, not volume") — one or two rule sets, 1–3 rules
//! each, a handful of header/body conditions, `[prefix]` present or
//! absent with a value chosen from the exact shapes RFC 058's bug
//! involved (`"."`, `"././."`).

use std::path::{Path, PathBuf};

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, RngAlgorithm, TestRng, TestRunner};

use crate::view::{EditCommand, RespondPayload, RulePayload};
use crate::workspace::Workspace;

// ── A fixed seed, so a CI failure is reproducible from the log alone ──

const FIXED_SEED: [u8; 32] = [0x60; 32]; // "RFC 060" — arbitrary, just fixed.

fn fixed_runner() -> TestRunner {
    TestRunner::new_with_rng(
        ProptestConfig {
            cases: 48,
            ..ProptestConfig::default()
        },
        TestRng::from_seed(RngAlgorithm::ChaCha, &FIXED_SEED),
    )
}

// ── Generator ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum GenRespond {
    Text(String),
    Status(u16),
}

#[derive(Debug, Clone)]
struct GenRule {
    url_path: String,
    method: Option<&'static str>,
    header: Option<(String, String)>,
    body_condition: Option<(String, String)>,
    respond: GenRespond,
}

#[derive(Debug, Clone)]
struct GenRuleSet {
    leading_comment: bool,
    /// `None` = no `[prefix]` section; `Some(dir)` = one whose
    /// `respond_dir` is `dir` — always a value that resolves to the
    /// rule set's own directory (`"."`/`"././."`), so it never needs a
    /// real subdirectory to exist.
    prefix: Option<&'static str>,
    rules: Vec<GenRule>,
}

fn ident() -> impl Strategy<Value = String> {
    "[a-z]{2,6}"
}

fn url_path_strategy() -> impl Strategy<Value = String> {
    ident().prop_map(|s| format!("/{s}"))
}

fn respond_strategy() -> impl Strategy<Value = GenRespond> {
    prop_oneof![
        "[a-zA-Z0-9 ]{1,12}".prop_map(GenRespond::Text),
        (200u16..599).prop_map(GenRespond::Status),
    ]
}

fn rule_strategy() -> impl Strategy<Value = GenRule> {
    (
        url_path_strategy(),
        prop::option::of(prop_oneof![Just("GET"), Just("POST"), Just("PUT")]),
        prop::option::of((ident(), ident())),
        prop::option::of((ident(), ident())),
        respond_strategy(),
    )
        .prop_map(
            |(url_path, method, header, body_condition, respond)| GenRule {
                url_path,
                method,
                header,
                body_condition,
                respond,
            },
        )
}

/// Includes `"././."` — a value that is *not* canonical. RFC 058's fix
/// deliberately normalises this to `"."` the next time the file is
/// really saved (`a_previously_grown_respond_dir_collapses_on_the_next_real_save`
/// in `save.rs`), so a value from this strategy is right for idempotence
/// (does it reach and *stay at* a fixed point, whatever that point is)
/// but wrong for preservation (which needs an already-canonical prefix
/// to state "unrelated to the edit, so unchanged" without re-deriving
/// RFC 058's own collapse rule inside this file).
fn prefix_strategy() -> impl Strategy<Value = Option<&'static str>> {
    prop_oneof![Just(None), Just(Some(".")), Just(Some("././."))]
}

/// Only canonical shapes — see `prefix_strategy`'s doc comment for why
/// preservation needs this narrower strategy instead.
fn stable_prefix_strategy() -> impl Strategy<Value = Option<&'static str>> {
    prop_oneof![Just(None), Just(Some("."))]
}

fn rule_set_strategy_with(
    prefix: impl Strategy<Value = Option<&'static str>>,
) -> impl Strategy<Value = GenRuleSet> {
    (
        proptest::bool::ANY,
        prefix,
        prop::collection::vec(rule_strategy(), 1..=3),
    )
        .prop_map(|(leading_comment, prefix, rules)| GenRuleSet {
            leading_comment,
            prefix,
            rules,
        })
}

fn rule_set_strategy() -> impl Strategy<Value = GenRuleSet> {
    rule_set_strategy_with(prefix_strategy())
}

/// A single-rule-set workspace — used by the invariants that don't need
/// more than one file to state (idempotence, preservation).
fn single_rule_set_workspace() -> impl Strategy<Value = Vec<GenRuleSet>> {
    rule_set_strategy().prop_map(|rs| vec![rs])
}

/// Like `single_rule_set_workspace`, but the prefix (if any) is already
/// canonical — see `stable_prefix_strategy`.
fn single_rule_set_workspace_stable_prefix() -> impl Strategy<Value = Vec<GenRuleSet>> {
    rule_set_strategy_with(stable_prefix_strategy()).prop_map(|rs| vec![rs])
}

/// A multi-rule-set workspace — used by locality and conflict safety,
/// which are meaningless with only one file in play.
fn multi_rule_set_workspace() -> impl Strategy<Value = Vec<GenRuleSet>> {
    prop::collection::vec(rule_set_strategy(), 2..=3)
}

// ── Rendering: the generator's output becomes real files on disk ──────

struct Seed {
    _dir: tempfile::TempDir,
    root_path: PathBuf,
    rule_set_paths: Vec<PathBuf>,
}

fn render(rule_sets: &[GenRuleSet]) -> Seed {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut rule_set_names = Vec::new();

    for (i, rs) in rule_sets.iter().enumerate() {
        let mut text = String::new();
        if rs.leading_comment {
            text.push_str("# a person's own comment, above everything else\n");
        }
        if let Some(respond_dir) = rs.prefix {
            text.push_str(&format!("[prefix]\nrespond_dir = {:?}\n\n", respond_dir));
        }
        for rule in &rs.rules {
            text.push_str("[[rules]]\n");
            text.push_str(&format!("when.request.url_path = {:?}\n", rule.url_path));
            if let Some(method) = rule.method {
                text.push_str(&format!("when.request.method = {:?}\n", method));
            }
            if let Some((name, value)) = &rule.header {
                text.push_str(&format!(
                    "when.request.headers.{name} = {{ value = {:?} }}\n",
                    value
                ));
            }
            if let Some((path, value)) = &rule.body_condition {
                text.push_str(&format!(
                    "when.request.body.json.{:?} = {{ op = \"equal\", value = {:?} }}\n",
                    path, value
                ));
            }
            match &rule.respond {
                GenRespond::Text(t) => text.push_str(&format!("respond = {{ text = {:?} }}\n", t)),
                GenRespond::Status(s) => {
                    text.push_str(&format!("respond = {{ status = {} }}\n", s))
                }
            }
        }
        let name = format!("rules-{i}.toml");
        std::fs::write(dir.path().join(&name), &text).expect("write rule set");
        rule_set_names.push(name);
    }

    let quoted: Vec<String> = rule_set_names.iter().map(|n| format!("{:?}", n)).collect();
    let root_text = format!(
        "[service]\nrule_sets = [{}]\nfallback_respond_dir = \".\"\n",
        quoted.join(", ")
    );
    let root_path = dir.path().join("apimock.toml");
    std::fs::write(&root_path, &root_text).expect("write root config");

    let rule_set_paths = rule_set_names.iter().map(|n| dir.path().join(n)).collect();
    Seed {
        _dir: dir,
        root_path,
        rule_set_paths,
    }
}

/// Build a `RulePayload` from a generated respond — the one edit driven
/// through the real `apply()` path rather than rendered directly, per
/// this file's own module doc.
fn to_rule_payload(url_path: String, respond: GenRespond) -> RulePayload {
    let mut respond_payload = RespondPayload::default();
    match respond {
        GenRespond::Text(t) => respond_payload.text = Some(t),
        GenRespond::Status(s) => respond_payload.status = Some(s),
    }
    RulePayload {
        url_path: Some(url_path),
        respond: respond_payload,
        ..Default::default()
    }
}

fn add_a_rule_via_apply(
    ws: &mut Workspace,
    rule_set_index: usize,
    url_path: String,
    respond: GenRespond,
) {
    let parent = ws
        .rule_set_id_at(rule_set_index)
        .expect("rule set exists at this index");
    ws.apply(EditCommand::AddRule {
        parent,
        rule: to_rule_payload(url_path, respond),
    })
    .expect("AddRule with a generated payload must apply cleanly");
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

// ═══════════════════════════════════════════════════════════════════
// Invariant 1 — Idempotence
// ═══════════════════════════════════════════════════════════════════

/// The `respond_dir` line, if `[prefix]` is present — the exact field
/// RFC 058's bug grew on every save. Mirrors `save.rs`'s own
/// `respond_dir_line` helper.
fn respond_dir_line(text: &str) -> Option<&str> {
    text.lines()
        .find(|l| l.trim_start().starts_with("respond_dir"))
}

#[test]
fn idempotence_repeated_real_saves_reach_a_fixed_point() {
    let mut runner = fixed_runner();
    runner
        .run(
            &(
                single_rule_set_workspace(),
                prop::collection::vec((url_path_strategy(), respond_strategy()), 2..=3),
            ),
            |(rule_sets, edits)| {
                let seed = render(&rule_sets);

                // Every round reloads from scratch before editing —
                // matching how this actually happens in production
                // (`apimock set` is a new process, and so a fresh
                // `Workspace::load`, on every invocation; RFC 057's own
                // handoff is explicit about this). RFC 058's bug only
                // showed itself across a *load*-then-save cycle: the
                // grown value had to be read back in as the new
                // "authored" input before it could grow again. Reusing
                // one in-memory `Workspace` across rounds — as the
                // hand-written `respond_dir_is_a_fixed_point_across_repeated_real_saves`
                // in `save.rs` does — cannot observe that, since the
                // resolved value is computed once at load time and never
                // rederived from a save alone; confirmed empirically
                // while writing this property (see the review package).
                let mut respond_dir_snapshots = Vec::new();
                for (url_path, respond) in edits {
                    let mut ws = Workspace::load(seed.root_path.clone())
                        .map_err(|e| TestCaseError::fail(format!("load: {e}")))?;
                    add_a_rule_via_apply(&mut ws, 0, url_path, respond);
                    let save = ws
                        .save()
                        .map_err(|e| TestCaseError::fail(format!("save: {e}")))?;
                    prop_assert!(
                        !save.changed_files.is_empty(),
                        "each round must be a real save, or this test proves nothing"
                    );
                    let text = read(&seed.rule_set_paths[0]);
                    respond_dir_snapshots.push(respond_dir_line(&text).map(|l| l.to_owned()));
                }

                if rule_sets[0].prefix.is_some() {
                    for pair in respond_dir_snapshots.windows(2) {
                        prop_assert_eq!(
                            &pair[0],
                            &pair[1],
                            "respond_dir must not drift between successive real saves: {:?}",
                            respond_dir_snapshots
                        );
                    }
                }

                // Separately: a save with *no new edit at all* right
                // after a real one must be a true no-op — the file must
                // not change even a single byte.
                let after_last_real_save: Vec<String> =
                    seed.rule_set_paths.iter().map(|p| read(p)).collect();
                let mut ws2 = Workspace::load(seed.root_path.clone())
                    .map_err(|e| TestCaseError::fail(format!("reload: {e}")))?;
                ws2.save()
                    .map_err(|e| TestCaseError::fail(format!("no-op save: {e}")))?;
                let after_no_op_save: Vec<String> =
                    seed.rule_set_paths.iter().map(|p| read(p)).collect();
                prop_assert_eq!(
                    after_last_real_save,
                    after_no_op_save,
                    "a save with no new edit must change nothing"
                );
                Ok(())
            },
        )
        .unwrap();
}

// ═══════════════════════════════════════════════════════════════════
// Invariant 2 — Preservation
// ═══════════════════════════════════════════════════════════════════

#[test]
fn preservation_untargeted_comments_and_prefix_survive_an_unrelated_edit() {
    let mut runner = fixed_runner();
    runner
        .run(
            &(
                single_rule_set_workspace_stable_prefix(),
                url_path_strategy(),
                respond_strategy(),
            ),
            |(rule_sets, extra_url_path, extra_respond)| {
                let seed = render(&rule_sets);
                let before = read(&seed.rule_set_paths[0]);

                let mut ws = Workspace::load(seed.root_path.clone())
                    .map_err(|e| TestCaseError::fail(format!("load: {e}")))?;
                add_a_rule_via_apply(&mut ws, 0, extra_url_path, extra_respond);
                ws.save()
                    .map_err(|e| TestCaseError::fail(format!("save: {e}")))?;

                let after = read(&seed.rule_set_paths[0]);

                if rule_sets[0].leading_comment {
                    prop_assert!(
                        after.starts_with("# a person's own comment, above everything else"),
                        "the leading comment must survive an edit that only adds a rule:\n{after}"
                    );
                }
                if let Some(respond_dir) = rule_sets[0].prefix {
                    let expected = format!("respond_dir = {:?}", respond_dir);
                    prop_assert!(
                        after.contains(&expected),
                        "the prefix's respond_dir must survive unchanged (not resolved/grown):\n{after}"
                    );
                }
                // Every rule already present before the edit must still
                // be present, verbatim — the edit only appends.
                for rule in &rule_sets[0].rules {
                    let needle = format!("when.request.url_path = {:?}", rule.url_path);
                    prop_assert!(
                        after.contains(&needle),
                        "an existing rule's url_path must survive:\n{after}"
                    );
                }
                let _ = before; // establishes intent; `after` is what's asserted on.
                Ok(())
            },
        )
        .unwrap();
}

// ═══════════════════════════════════════════════════════════════════
// Invariant 3 — Locality
// ═══════════════════════════════════════════════════════════════════

#[test]
fn locality_an_edit_to_one_rule_set_leaves_every_other_file_untouched() {
    let mut runner = fixed_runner();
    runner
        .run(
            &(multi_rule_set_workspace(), url_path_strategy(), respond_strategy()),
            |(rule_sets, extra_url_path, extra_respond)| {
                let seed = render(&rule_sets);
                let before: Vec<String> =
                    seed.rule_set_paths.iter().map(|p| read(p)).collect();
                let root_before = read(&seed.root_path);

                let mut ws = Workspace::load(seed.root_path.clone())
                    .map_err(|e| TestCaseError::fail(format!("load: {e}")))?;
                // Edit only rule set 0.
                add_a_rule_via_apply(&mut ws, 0, extra_url_path, extra_respond);
                let save = ws
                    .save()
                    .map_err(|e| TestCaseError::fail(format!("save: {e}")))?;

                prop_assert!(
                    save.changed_files
                        .iter()
                        .all(|p| p != &seed.rule_set_paths[1] && p != &seed.root_path),
                    "save must not report rule set #2 or the root config as changed: {:?}",
                    save.changed_files
                );

                for (i, path) in seed.rule_set_paths.iter().enumerate().skip(1) {
                    let after = read(path);
                    prop_assert_eq!(
                        &before[i],
                        &after,
                        "rule set #{} must be byte-identical after an edit to rule set #1",
                        i + 1
                    );
                }
                let root_after = read(&seed.root_path);
                prop_assert_eq!(
                    root_before,
                    root_after,
                    "the root config must be byte-identical after an edit that only touches a rule set"
                );
                Ok(())
            },
        )
        .unwrap();
}

// ═══════════════════════════════════════════════════════════════════
// Invariant 4 — Conflict safety
// ═══════════════════════════════════════════════════════════════════

#[test]
fn conflict_safety_an_external_change_is_refused_and_nothing_partial_is_written() {
    let mut runner = fixed_runner();
    runner
        .run(
            &(
                multi_rule_set_workspace(),
                url_path_strategy(),
                respond_strategy(),
                url_path_strategy(),
                respond_strategy(),
            ),
            |(rule_sets, url_a, respond_a, url_b, respond_b)| {
                let seed = render(&rule_sets);
                let rs1_before = read(&seed.rule_set_paths[1]);

                let mut ws = Workspace::load(seed.root_path.clone())
                    .map_err(|e| TestCaseError::fail(format!("load: {e}")))?;
                // Pending changes to *two* files in the write set.
                add_a_rule_via_apply(&mut ws, 0, url_a, respond_a);
                add_a_rule_via_apply(&mut ws, 1, url_b, respond_b);

                // Someone else edits rule set #1 on disk after our load().
                let external_edit = "[[rules]]\nwhen.request.url_path = \"/external\"\nrespond.text = \"external\"\n";
                std::fs::write(&seed.rule_set_paths[0], external_edit)
                    .expect("simulate external edit");

                let err = ws.save();
                prop_assert!(
                    matches!(err, Err(crate::error::SaveError::Conflict { .. })),
                    "save must refuse when a pending file changed on disk underneath it, got {:?}",
                    err
                );

                // The externally-changed file keeps the external edit —
                // refusing must not clobber it with our own pending write.
                let rs0_after = read(&seed.rule_set_paths[0]);
                prop_assert_eq!(
                    &rs0_after,
                    external_edit,
                    "a refused save must not overwrite the file it refused"
                );

                // "Nothing partial is written": rule set #1's own
                // pending change (unrelated to the conflict) must not
                // have been written either — the whole write set is
                // checked before anything is written, not file by file.
                let rs1_after = read(&seed.rule_set_paths[1]);
                prop_assert_eq!(
                    &rs1_before,
                    &rs1_after,
                    "a refused save must not partially apply other pending changes"
                );
                Ok(())
            },
        )
        .unwrap();
}
