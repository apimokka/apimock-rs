//! Task 019 — `apimock set` deletes only what it manages.
//!
//! `set` used to rewrite a config file keeping only the keys the writer
//! models, and delete the rest: `[listener.tls] max_connections`,
//! `[service] max_request_body_bytes`, a rule set's `[default]` — and the
//! command exited 0 and the result still validated. Four of those keys
//! are availability limits, so for an operator who had tightened one,
//! an unrelated `set` loosened it.
//!
//! The library-level matrix (every kind of edit, `Workspace::save`, the
//! docs-driven coverage of the fixture) lives in
//! `crates/apimock-config/src/workspace/tests/preserve.rs`. This file is
//! the same guarantee through the binary, plus the one assertion that is
//! about behaviour and not text: a `[default]` delay still delays.

#[path = "util.rs"]
mod util;

use std::path::Path;
use std::time::{Duration, Instant};

use util::{cli::run_full, http::test_request::TestRequest, test_setup::TestSetup};

const ROOT_TEMPLATE: &str =
    include_str!("../../apimock-config/tests/fixtures/maximal/apimock.toml");
const RULE_SET: &str = include_str!("../../apimock-config/tests/fixtures/maximal/rules.toml");

/// `text` with every line ending made `\r\n` or `\n`, whatever a checkout
/// (git's `autocrlf` on Windows) made of it. The tests that compare whole
/// files run in both, so CRLF is exercised on every platform. The first
/// push of this task failed only on Windows, because it was not.
fn with_eol(text: &str, crlf: bool) -> String {
    let lf = text.replace("\r\n", "\n");
    if crlf { lf.replace('\n', "\r\n") } else { lf }
}

/// Write the maximal fixture into `dir`. TLS paths are made absolute: they
/// resolve against the working directory, not the config.
fn instantiate(dir: &Path, crlf: bool) -> String {
    let root = with_eol(
        &ROOT_TEMPLATE.replace("@DIR@", &dir.to_string_lossy().replace('\\', "/")),
        crlf,
    );
    std::fs::write(dir.join("apimock.toml"), &root).unwrap();
    std::fs::write(dir.join("rules.toml"), with_eol(RULE_SET, crlf)).unwrap();
    std::fs::write(dir.join("cert.pem"), "x\n").unwrap();
    std::fs::write(dir.join("key.pem"), "x\n").unwrap();
    std::fs::write(dir.join("mw.rhai"), "// middleware\n").unwrap();
    std::fs::create_dir_all(dir.join("responses")).unwrap();
    std::fs::write(dir.join("responses/data.csv"), "id,name\n1,a\n").unwrap();
    root
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

/// § 1's root fixture: adding a rule to a *new* rule-set file rewrites the
/// root (it gains the file in `service.rule_sets`). The five keys that used
/// to vanish — and every other byte — are still there; the one line that
/// changes is the one that should. In LF and in CRLF.
#[test]
fn set_into_a_new_rule_set_changes_only_the_rule_sets_line() {
    for crlf in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let before = instantiate(dir.path(), crlf);

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
        assert_eq!(code, 0, "crlf={crlf} stderr:\n{err}");

        assert_eq!(
            read(dir.path(), "apimock.toml"),
            before.replace(
                "rule_sets = [\"rules.toml\"]",
                "rule_sets = [\"rules.toml\", \"new.toml\"]"
            ),
            "crlf={crlf}"
        );
    }
}

/// § 1's rule-set fixture: adding a rule to an existing file appends it and
/// changes nothing before it. `[default]`, `[guard]` and `weight` are in
/// that file. In CRLF the appended rule is CRLF as well, and no bare LF
/// appears: `toml_edit` itself turns CRLF into LF, which this task's writer
/// undoes.
#[test]
fn set_into_an_existing_rule_set_only_appends() {
    for crlf in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        instantiate(dir.path(), crlf);

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
        assert_eq!(code, 0, "crlf={crlf} stderr:\n{err}");

        let after = read(dir.path(), "rules.toml");
        assert!(
            after.starts_with(&with_eol(RULE_SET, crlf)),
            "crlf={crlf}: the existing file changed:\n{after:?}"
        );
        assert!(after.contains("url_path = \"/new\""), "{after}");
        if crlf {
            assert!(
                !after.replace("\r\n", "").contains('\n'),
                "bare LF in {after:?}"
            );
        } else {
            assert!(!after.contains('\r'), "CR in an LF file: {after:?}");
        }
    }
}

/// The other direction. `set` replacing a rule's `text` with `json` must
/// still remove `text`: preserving what the writer does not manage must not
/// stop it deleting what it does.
#[test]
fn a_managed_key_that_set_replaces_is_removed() {
    let dir = tempfile::tempdir().unwrap();
    instantiate(dir.path(), false);

    let (code, _out, err) = run_full(
        dir.path(),
        &[
            "set",
            "rule",
            "-c",
            "apimock.toml",
            "--rule-set",
            "rules.toml",
            "--rule",
            "0",
            "--json",
            "{\"a\":1}",
        ],
    );
    assert_eq!(code, 0, "stderr:\n{err}");

    let after = read(dir.path(), "rules.toml");
    assert!(
        !after.contains("respond.text = \"slow\""),
        "text should be gone:\n{after}"
    );
    assert!(after.contains("json"), "{after}");
    assert!(after.contains("weight = 3 # weight-one"), "{after}");
    assert!(after.contains("[default] # default-comment"), "{after}");
}

/// The worst form of the defect, found while auditing this task: a
/// strategy written as a table (`weighted_random = { seed = 7 }`) was
/// rewritten to the bare string `"weighted_random"`, which does not load
/// ("invalid type: unit variant, expected struct variant"). `set` exited 0
/// and left a config the server then refused to start from. The line must
/// be left as written, and the result must still validate.
#[test]
fn set_does_not_turn_a_table_strategy_into_a_config_that_will_not_load() {
    let dir = tempfile::tempdir().unwrap();
    instantiate(dir.path(), false);
    // The fixture's root enables TLS with placeholder PEMs, which `validate`
    // accepts (it checks existence only), so the fixture is validated as is.
    let (code, _out, err) = run_full(dir.path(), &["validate", "-c", "apimock.toml"]);
    assert_eq!(code, 0, "the fixture must validate before the edit:\n{err}");

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
    assert_eq!(code, 0, "stderr:\n{err}");

    assert!(
        read(dir.path(), "rules.toml")
            .replace("\r\n", "\n")
            .starts_with(
                "# rules header comment\nstrategy = { weighted_random = { seed = 7 } } # strategy-comment\n"
            )
    );
    let (code, _out, err) = run_full(dir.path(), &["validate", "-c", "apimock.toml"]);
    assert_eq!(code, 0, "`set` left a config that no longer loads:\n{err}");
}

/// Task 018's deprecation warning must still fire once on `set`, with the
/// `[guard]` line now preserved rather than deleted.
#[test]
fn the_guard_warning_fires_once_and_the_guard_line_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    instantiate(dir.path(), false);

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
    assert_eq!(code, 0, "stderr:\n{err}");

    assert_eq!(err.matches("`[guard]`").count(), 1, "stderr:\n{err}");
    assert!(read(dir.path(), "rules.toml").contains("[guard] # guard-comment"));
}

/// Behaviour, not text. `[default] delay_response_milliseconds` is a
/// working feature; the architect measured `/slow` at 1.502s before an
/// unrelated `set` and 0.0005s after. Asserted here on the served response.
#[tokio::test]
async fn a_default_delay_still_delays_after_an_unrelated_set() {
    let dir = tempfile::tempdir().unwrap();
    instantiate(dir.path(), false);
    // The fixture's root enables TLS with placeholder PEMs; serve this
    // rule set from a plain root instead.
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    let config = dir.path().join("apimock.toml");

    async fn slow_response_time(config: &Path) -> Duration {
        let setup = TestSetup {
            root_config_file_path: Some(config.to_string_lossy().into_owned()),
            port: None,
            fallback_respond_dir_path: None,
            current_dir_path: None,
        };
        let port = setup.launch().await;
        let started = Instant::now();
        let response = TestRequest::default("/api/slow", port).send().await;
        assert_eq!(response.status(), 200);
        started.elapsed()
    }

    // Before: the delay is real, so the measurement means something.
    let before = slow_response_time(&config).await;
    assert!(before >= Duration::from_millis(1400), "before: {before:?}");

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
    assert_eq!(code, 0, "stderr:\n{err}");
    assert!(read(dir.path(), "rules.toml").contains("delay_response_milliseconds = 1500"));

    let after = slow_response_time(&config).await;
    assert!(
        after >= Duration::from_millis(1400),
        "the default delay was lost by `set`: /api/slow answered in {after:?}"
    );
}
