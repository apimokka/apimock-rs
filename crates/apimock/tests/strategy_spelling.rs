//! RFC 085 — a strategy is spelled as a bare name or as a table, at both
//! levels, and an unrelated `set` rewrites neither.
//!
//! Before: `uniform_random`, `weighted_random` and `priority` loaded only as
//! tables, so `strategy = "weighted_random"` failed with serde's
//! *"invalid type: unit variant, expected struct variant"*.

#[path = "util.rs"]
mod util;

use std::path::Path;

use util::cli::run_full;

const NAMES: [&str; 5] = [
    "first_match",
    "round_robin",
    "uniform_random",
    "weighted_random",
    "priority",
];

const RULE: &str = "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"a\"\n";

/// `text` with every line ending made `\r\n` or `\n`, whatever a checkout
/// made of it, so each test runs in both on every platform.
fn with_eol(text: &str, crlf: bool) -> String {
    let lf = text.replace("\r\n", "\n");
    if crlf { lf.replace('\n', "\r\n") } else { lf }
}

fn write(dir: &Path, name: &str, text: &str, crlf: bool) -> String {
    let text = with_eol(text, crlf);
    std::fs::write(dir.join(name), &text).unwrap();
    text
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn validate(dir: &Path) -> (i32, String) {
    let (code, _out, err) = run_full(dir, &["validate", "-c", "apimock.toml"]);
    (code, err)
}

/// Option B: every bare name loads, at `[service]` and in a rule set, and
/// `validate` says so. Both levels are proved, not one.
#[test]
fn every_bare_strategy_name_loads_at_both_levels() {
    for name in NAMES {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "apimock.toml",
            &format!(
                "[service]\nstrategy = \"{name}\"\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n"
            ),
            false,
        );
        write(
            dir.path(),
            "rules.toml",
            &format!("strategy = \"{name}\"\n{RULE}"),
            false,
        );
        let (code, err) = validate(dir.path());
        assert_eq!(code, 0, "{name}: {err}");
    }
}

/// The table form still loads, with and without options.
#[test]
fn the_table_form_still_loads() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "apimock.toml",
        "[service]\nstrategy = { priority = { tiebreaker = \"uniform_random\" } }\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
        false,
    );
    write(
        dir.path(),
        "rules.toml",
        &format!("strategy = {{ weighted_random = {{ seed = 7 }} }}\n{RULE}"),
        false,
    );
    let (code, err) = validate(dir.path());
    assert_eq!(code, 0, "{err}");
}

/// A typo is refused with the five names listed, at both levels — not with
/// serde's "unit variant" or "did not match any variant".
#[test]
fn a_typo_is_refused_with_the_names_listed_at_both_levels() {
    const LISTED: &str = "unknown strategy `weigthed_random`, expected one of `first_match`, `round_robin`, `uniform_random`, `weighted_random`, `priority`";

    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "apimock.toml",
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
        false,
    );
    write(
        dir.path(),
        "rules.toml",
        &format!("strategy = \"weigthed_random\"\n{RULE}"),
        false,
    );
    let (code, err) = validate(dir.path());
    assert_eq!(code, 2, "{err}");
    assert!(err.contains(LISTED), "rule-set level:\n{err}");
    assert!(!err.contains("did not match any variant"), "{err}");

    write(dir.path(), "rules.toml", RULE, false);
    write(
        dir.path(),
        "apimock.toml",
        "[service]\nstrategy = \"weigthed_random\"\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
        false,
    );
    let (code, err) = validate(dir.path());
    assert_eq!(code, 2, "{err}");
    assert!(err.contains(LISTED), "service level:\n{err}");
}

/// § 4: an unrelated `set` rewrites neither spelling. A file using the bare
/// name and one using an empty table with a comment are byte-identical after
/// a `set` that changes a different rule: at the rule-set level, where the
/// new rule is appended, and at `[service]` level, where the root is
/// rewritten to gain a rule-set file. In LF and in CRLF.
#[test]
fn an_unrelated_set_rewrites_neither_spelling() {
    for spelling in [
        "\"weighted_random\"",
        "{ weighted_random = {} } # comment",
        "{ weighted_random = { seed = 7 } } # comment",
        "\"priority\"",
        "{ priority = { tiebreaker = \"uniform_random\" } }",
    ] {
        for crlf in [false, true] {
            let ctx = format!("{spelling} crlf={crlf}");

            // Rule-set level.
            let dir = tempfile::tempdir().unwrap();
            write(
                dir.path(),
                "apimock.toml",
                "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
                crlf,
            );
            let original = write(
                dir.path(),
                "rules.toml",
                &format!("strategy = {spelling}\n\n{RULE}"),
                crlf,
            );
            let (code, _out, err) = run_full(
                dir.path(),
                &[
                    "set",
                    "rule",
                    "-c",
                    "apimock.toml",
                    "--rule-set",
                    "rules.toml",
                    "--path",
                    "/new",
                    "--text",
                    "new",
                ],
            );
            assert_eq!(code, 0, "{ctx}: {err}");
            let after = read(dir.path(), "rules.toml");
            assert!(
                after.starts_with(&original),
                "{ctx}: the strategy line changed:\n{after:?}"
            );

            // `[service]` level.
            let dir = tempfile::tempdir().unwrap();
            let before = write(
                dir.path(),
                "apimock.toml",
                &format!(
                    "[service]\nstrategy = {spelling}\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n"
                ),
                crlf,
            );
            write(dir.path(), "rules.toml", RULE, crlf);
            let (code, _out, err) = run_full(
                dir.path(),
                &[
                    "set",
                    "rule",
                    "-c",
                    "apimock.toml",
                    "--rule-set",
                    "new.toml",
                    "--path",
                    "/n",
                    "--text",
                    "n",
                ],
            );
            assert_eq!(code, 0, "{ctx}: {err}");
            assert_eq!(
                read(dir.path(), "apimock.toml"),
                before.replace(
                    "rule_sets = [\"rules.toml\"]",
                    "rule_sets = [\"rules.toml\", \"new.toml\"]"
                ),
                "{ctx}"
            );
        }
    }
}
