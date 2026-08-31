//! Post-6.0.0 test-integrity handoff § 1.
//!
//! `apimock_routing::Respond::validate` and this crate's own
//! `workspace::validate::respond_node_validation` are **independent
//! implementations of the same rules** — deliberately duplicated (see
//! `validate.rs`'s own module doc comment for why a GUI needs
//! structured issues rather than a `log::error!` + `bool`), and nothing
//! before this test checked that the duplication actually stays in
//! step.
//!
//! It didn't, once: RFC 065 added `json` as a third body source to
//! `Respond::validate`, and `respond_node_validation`'s own copy of the
//! "at least one body source" / "mutually exclusive" checks didn't
//! learn about it — a config using `respond.json` loaded fine but
//! `apimock validate` still reported a false "requires at least one of
//! file_path, text, or status" error on every run. Caught only because
//! the dev team happened to test the *success* path for `json` rather
//! than another failure case.
//!
//! This test runs one shared corpus of `Respond` values through both
//! validators and asserts they reach the same verdict (both accept, or
//! both reject) — not that their messages match, which the handoff
//! explicitly does not ask for and which would make this test brittle
//! against wording changes that carry no behavioural difference.

use std::path::Path;

use apimock_routing::{Respond, RuleSet};

use crate::workspace::validate::respond_node_validation;

/// A `RuleSet` rooted at `dir`, with no `[prefix]` block, so
/// `dir_prefix()` resolves to `dir` itself (`Path::new(dir).join(".")`,
/// which is the same location `.` collapses to) — a file-based
/// `Respond` case writes its file directly into `dir` and references it
/// by bare filename, matching what `Respond::validate(dir_prefix, ..)`
/// is given directly below.
///
/// `RuleSet` is `#[non_exhaustive]` with no cross-crate literal
/// constructor (by design — see its own doc comment), so this goes
/// through the real `RuleSet::new`, reading an actual (trivial, valid)
/// rule-set file from disk rather than trying to fake one.
fn rule_set_in(dir: &Path) -> RuleSet {
    let rs_path = dir.join("apimock-rule-set.toml");
    std::fs::write(
        &rs_path,
        "[[rules]]\nwhen.request.url_path = \"/probe\"\nrespond.text = \"ok\"\n",
    )
    .expect("write rule-set file");
    RuleSet::new(rs_path.to_str().unwrap(), dir.to_str().unwrap(), 0).expect("RuleSet::new")
}

/// One corpus entry: a `Respond` to check, an optional file to write
/// into the case's own directory before checking it (name, content),
/// and the verdict both validators are expected to reach.
struct Case {
    name: &'static str,
    respond: Respond,
    file: Option<(&'static str, &'static str)>,
    expect_ok: bool,
}

fn respond_with(f: impl FnOnce(&mut Respond)) -> Respond {
    let mut r = Respond::default();
    f(&mut r);
    r
}

fn corpus() -> Vec<Case> {
    vec![
        // ── Empty ─────────────────────────────────────────────────
        Case {
            name: "empty respond (no field set) is rejected",
            respond: Respond::default(),
            file: None,
            expect_ok: false,
        },
        // ── Each body source alone ───────────────────────────────
        Case {
            name: "status alone is accepted",
            respond: respond_with(|r| r.status = Some(204)),
            file: None,
            expect_ok: true,
        },
        Case {
            name: "text alone is accepted",
            respond: respond_with(|r| r.text = Some("hi".to_owned())),
            file: None,
            expect_ok: true,
        },
        Case {
            name: "json alone is accepted",
            respond: respond_with(|r| r.json = Some(r#"{"a":1}"#.to_owned())),
            file: None,
            expect_ok: true,
        },
        Case {
            name: "file_path alone (existing file) is accepted",
            respond: respond_with(|r| r.file_path = Some("data.txt".to_owned())),
            file: Some(("data.txt", "hello")),
            expect_ok: true,
        },
        // ── Every pairwise body-source combination ───────────────
        Case {
            name: "json + text together are rejected",
            respond: respond_with(|r| {
                r.json = Some(r#"{"a":1}"#.to_owned());
                r.text = Some("hi".to_owned());
            }),
            file: None,
            expect_ok: false,
        },
        Case {
            name: "json + file_path together are rejected",
            respond: respond_with(|r| {
                r.json = Some(r#"{"a":1}"#.to_owned());
                r.file_path = Some("data.json".to_owned());
            }),
            file: Some(("data.json", r#"{"a":1}"#)),
            expect_ok: false,
        },
        Case {
            name: "file_path + text together are rejected",
            respond: respond_with(|r| {
                r.file_path = Some("data.txt".to_owned());
                r.text = Some("hi".to_owned());
            }),
            file: Some(("data.txt", "hello")),
            expect_ok: false,
        },
        Case {
            name: "file_path + status together are rejected",
            respond: respond_with(|r| {
                r.file_path = Some("data.txt".to_owned());
                r.status = Some(200);
            }),
            file: Some(("data.txt", "hello")),
            expect_ok: false,
        },
        // The one asymmetric pairing: json (unlike file_path) may
        // combine with status. Included because an asymmetric rule is
        // exactly the shape a copy-by-hand can drift on.
        Case {
            name: "json + status together are accepted",
            respond: respond_with(|r| {
                r.json = Some(r#"{"error":"nope"}"#.to_owned());
                r.status = Some(404);
            }),
            file: None,
            expect_ok: true,
        },
        // ── Inline json, valid and malformed ─────────────────────
        Case {
            name: "malformed inline json is rejected",
            respond: respond_with(|r| r.json = Some("{not json".to_owned())),
            file: None,
            expect_ok: false,
        },
        // ── A referenced .json file, valid and malformed ─────────
        Case {
            name: "referenced valid .json file is accepted",
            respond: respond_with(|r| r.file_path = Some("good.json".to_owned())),
            file: Some(("good.json", r#"{"a":1}"#)),
            expect_ok: true,
        },
        Case {
            name: "referenced malformed .json file is rejected",
            respond: respond_with(|r| r.file_path = Some("bad.json".to_owned())),
            file: Some(("bad.json", "{not json")),
            expect_ok: false,
        },
        // A missing file_path — not explicitly named in the handoff's
        // minimum list, but the same class of on-disk check as the two
        // cases above and just as capable of silently diverging.
        Case {
            name: "missing file_path is rejected",
            respond: respond_with(|r| r.file_path = Some("does-not-exist.json".to_owned())),
            file: None,
            expect_ok: false,
        },
    ]
}

#[test]
fn both_validators_agree_on_every_case_in_the_shared_corpus() {
    for case in corpus() {
        let dir = tempfile::tempdir().expect("tempdir");
        if let Some((name, content)) = case.file {
            std::fs::write(dir.path().join(name), content).expect("write case file");
        }
        let rule_set = rule_set_in(dir.path());

        let routing_ok = case
            .respond
            .validate(dir.path().to_str().unwrap(), 0, 0)
            .is_ok();
        let config_ok = respond_node_validation(&case.respond, &rule_set, 0, 0).ok;

        assert_eq!(
            routing_ok, config_ok,
            "{}: Respond::validate says ok={routing_ok}, respond_node_validation says ok={config_ok} — the two validators disagree",
            case.name
        );
        assert_eq!(
            routing_ok, case.expect_ok,
            "{}: both validators agree with each other (ok={routing_ok}) but not with the expected verdict (ok={})",
            case.name, case.expect_ok
        );
    }
}
