//! Task 019 — `Workspace::save` deletes only what the writer manages.
//!
//! Every test here starts from the same config: `tests/fixtures/maximal/`,
//! a root file and a rule-set file that between them set every key the
//! documentation lists, each with a comment on it. A save that changes
//! one unrelated thing must leave every other byte alone.
//!
//! # How the fixture is kept in step with the schema
//!
//! A fixture nobody updates would recreate the defect: it would cover the
//! keys that existed when it was written and say nothing about the ones
//! added since. Three things stop that.
//!
//! 1. `the_fixture_sets_every_key_the_docs_list` reads the two reference
//!    pages, takes every field they put in a table, and fails if the
//!    fixture does not set it. Documenting a key without adding it here
//!    fails this test, so the key is covered the moment it is documented.
//! 2. `the_fixture_loads` runs it through the loader, which refuses an
//!    unknown key (`deny_unknown_fields`), so the fixture cannot carry a
//!    key the schema does not have.
//! 3. The writer's ownership tree is checked against what the writer emits
//!    for this same fixture (`toml_writer::tests`).
//!
//! The guarantee itself does not rest on the fixture. A key the writer
//! does not list is never deleted, so a key added tomorrow and not yet in
//! the fixture is still preserved; what the fixture and the tests
//! guard is that the mechanism keeps doing that for every key known today.

use std::path::{Path, PathBuf};

use crate::{
    view::{
        ConfigFileKind, EditCommand, EditValue, NodeId, NodeKind, RespondPayload, RootSettingKey,
        RulePayload,
    },
    workspace::Workspace,
};

const ROOT_TEMPLATE: &str = include_str!("../../../tests/fixtures/maximal/apimock.toml");
const RULE_SET: &str = include_str!("../../../tests/fixtures/maximal/rules.toml");
const ROOT_DOC: &str =
    include_str!("../../../../../docs/src/reference/apimock-toml-root-settings.md");
const RULE_SET_DOC: &str = include_str!("../../../../../docs/src/reference/rule-set-schema.md");

/// `text` with every line ending made `\r\n` or `\n`, whatever a checkout
/// (git's `autocrlf` on Windows) made of it. The tests that compare whole
/// files run in both, so the CRLF case is exercised on every platform and
/// not only where a checkout happens to produce it.
fn with_eol(text: &str, crlf: bool) -> String {
    let lf = text.replace("\r\n", "\n");
    if crlf { lf.replace('\n', "\r\n") } else { lf }
}

/// Write the fixture into `dir`, with the TLS paths made absolute (they
/// resolve against the process's working directory, not the config's).
/// Returns the root config's path and its exact text.
fn instantiate(dir: &Path, crlf: bool) -> (PathBuf, String) {
    let here = dir.to_string_lossy().replace('\\', "/");
    let root = with_eol(&ROOT_TEMPLATE.replace("@DIR@", &here), crlf);
    std::fs::write(dir.join("apimock.toml"), &root).unwrap();
    std::fs::write(dir.join("rules.toml"), with_eol(RULE_SET, crlf)).unwrap();
    std::fs::write(dir.join("cert.pem"), "x\n").unwrap();
    std::fs::write(dir.join("key.pem"), "x\n").unwrap();
    std::fs::write(dir.join("mw.rhai"), "// middleware\n").unwrap();
    std::fs::create_dir_all(dir.join("responses")).unwrap();
    std::fs::write(dir.join("responses/data.csv"), "id,name\n1,a\n").unwrap();
    (dir.join("apimock.toml"), root)
}

fn load(dir: &Path, crlf: bool) -> (Workspace, String) {
    let (root_path, root_text) = instantiate(dir, crlf);
    (
        Workspace::load(root_path).expect("the fixture loads"),
        root_text,
    )
}

fn rule_set_node(ws: &Workspace) -> NodeId {
    ws.snapshot()
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .unwrap()
        .id
}

fn rule_nodes(ws: &Workspace) -> Vec<NodeId> {
    ws.snapshot()
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Rule))
        .map(|n| n.id)
        .collect()
}

/// `(url_path, weight)` of each rule in the file, in order, read from
/// what is on disk.
fn rules_on_disk(dir: &Path) -> Vec<(String, Option<i64>)> {
    let text = std::fs::read_to_string(dir.join("rules.toml")).unwrap();
    let doc: toml::Table = toml::from_str(&text).unwrap();
    doc["rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| {
            let request = &rule["when"]["request"];
            let path = match &request["url_path"] {
                toml::Value::String(s) => s.clone(),
                other => other["value"].as_str().unwrap().to_owned(),
            };
            (path, rule.get("weight").and_then(|w| w.as_integer()))
        })
        .collect()
}

#[test]
fn the_fixture_loads() {
    let dir = tempfile::tempdir().unwrap();
    let (ws, _) = load(dir.path(), false);
    assert_eq!(rule_nodes(&ws).len(), 3);
}

/// § 1, the root fixture: an edit unrelated to the five keys the writer
/// used to delete changes one value and nothing else, in LF and CRLF.
#[test]
fn an_unrelated_root_edit_changes_only_that_value() {
    for crlf in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (mut ws, root_text) = load(dir.path(), crlf);

        ws.apply(EditCommand::UpdateRootSetting {
            key: RootSettingKey::ListenerPort,
            value: EditValue::Integer(4000),
        })
        .unwrap();
        ws.save().unwrap();

        let after = std::fs::read_to_string(dir.path().join("apimock.toml")).unwrap();
        assert_eq!(
            after,
            root_text.replace("port = 3991 # port-comment", "port = 4000 # port-comment"),
            "crlf={crlf}"
        );
        // Every key the writer used to delete is in that text, because the
        // whole file is equal apart from the one value.
        for kept in [
            "handshake_timeout_seconds = 3 # hs-comment",
            "max_connections = 8 # mc-comment",
            "cors_allow_credentials_origins = [\"https://app.example.test\"] # cors-comment",
            "max_request_body_bytes = 1024 # body-comment",
            "middleware_max_operations = 5000 # ops-comment",
        ] {
            assert!(after.contains(kept), "crlf={crlf}: lost `{kept}`:\n{after}");
        }
    }
}

/// Adding a rule leaves the whole of the existing file as it was: the new
/// rule is appended and nothing before it moves. `[default]` and
/// `[guard]` are in that file. In LF and CRLF, and in CRLF every added line
/// is CRLF too.
#[test]
fn adding_a_rule_appends_and_leaves_the_rest_byte_for_byte() {
    for crlf in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (mut ws, root_text) = load(dir.path(), crlf);
        let parent = rule_set_node(&ws);

        ws.apply(EditCommand::AddRule {
            parent,
            rule: RulePayload {
                url_path: Some("/new".to_owned()),
                respond: RespondPayload {
                    text: Some("new".to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            },
        })
        .unwrap();
        ws.save().unwrap();

        let after = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
        assert!(
            after.starts_with(&with_eol(RULE_SET, crlf)),
            "crlf={crlf}: the existing file changed:\n{after:?}"
        );
        assert!(after.contains("url_path = \"/new\""), "{after}");
        assert!(after.contains("[default] # default-comment"));
        assert!(after.contains("[guard] # guard-comment"));
        if crlf {
            assert!(
                !after.replace("\r\n", "").contains('\n'),
                "bare LF in {after:?}"
            );
        } else {
            assert!(!after.contains('\r'), "CR in an LF file: {after:?}");
        }
        // The root was not part of this change and was not rewritten.
        assert_eq!(
            std::fs::read_to_string(dir.path().join("apimock.toml")).unwrap(),
            root_text
        );
    }
}

/// Editing a rule keeps the keys on it the writer does not manage, and
/// every comment above its keys.
#[test]
fn editing_a_rule_keeps_its_weight_and_comments() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ws, _) = load(dir.path(), false);
    let rule = rule_nodes(&ws)[0];

    ws.apply(EditCommand::UpdateRule {
        id: rule,
        rule: RulePayload {
            url_path: Some("/slower".to_owned()),
            priority: Some(10),
            respond: RespondPayload {
                text: Some("slow".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
    })
    .unwrap();
    ws.save().unwrap();

    let after = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    assert!(after.contains("weight = 3 # weight-one"), "{after}");
    assert!(after.contains("# above-url_path comment"), "{after}");
    assert!(after.contains("[default] # default-comment"), "{after}");
    assert_eq!(
        rules_on_disk(dir.path())[0],
        ("/slower".to_owned(), Some(3))
    );
}

/// A managed key the user removed is still removed. The first rule's
/// `priority` is managed; `weight`, beside it, is not.
#[test]
fn a_managed_key_removed_through_the_editor_is_removed_from_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ws, _) = load(dir.path(), false);
    let rule = rule_nodes(&ws)[0];
    assert!(
        std::fs::read_to_string(dir.path().join("rules.toml"))
            .unwrap()
            .contains("priority = 10 # priority-comment")
    );

    ws.apply(EditCommand::UpdateRule {
        id: rule,
        rule: RulePayload {
            url_path: Some("/slow".to_owned()),
            priority: None,
            respond: RespondPayload {
                text: Some("slow".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
    })
    .unwrap();
    ws.save().unwrap();

    let after = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    assert!(
        !after.contains("priority = 10"),
        "priority should be gone:\n{after}"
    );
    assert!(after.contains("weight = 3 # weight-one"), "{after}");
}

/// Deleting a rule must not hand its unmanaged keys to its neighbour.
/// With rows matched by index, the rule after the deleted one would have
/// inherited its `weight`.
#[test]
fn deleting_a_rule_takes_its_weight_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ws, _) = load(dir.path(), false);
    let first = rule_nodes(&ws)[0];

    ws.apply(EditCommand::DeleteRule { id: first }).unwrap();
    ws.save().unwrap();

    assert_eq!(
        rules_on_disk(dir.path()),
        vec![
            ("/orders".to_owned(), Some(5)),
            ("/data".to_owned(), Some(9)),
        ]
    );
    let after = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    assert!(!after.contains("weight-one"), "{after}");
    assert!(after.contains("weight = 5 # weight-two"), "{after}");
    assert!(after.contains("weight = 9 # weight-three"), "{after}");
}

/// Moving a rule moves its weight and its comments with it, and the file
/// then lists the rules in the order the editor shows.
#[test]
fn moving_a_rule_moves_its_weight_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ws, _) = load(dir.path(), false);
    let last = rule_nodes(&ws)[2];

    ws.apply(EditCommand::MoveRule {
        id: last,
        new_index: 0,
    })
    .unwrap();
    ws.save().unwrap();

    assert_eq!(
        rules_on_disk(dir.path()),
        vec![
            ("/data".to_owned(), Some(9)),
            ("/slow".to_owned(), Some(3)),
            ("/orders".to_owned(), Some(5)),
        ]
    );
    let after = std::fs::read_to_string(dir.path().join("rules.toml")).unwrap();
    assert!(after.contains("[default] # default-comment"), "{after}");
    // And it still loads, in the new order.
    let root = dir.path().join("apimock.toml");
    Workspace::load(root).expect("the rewritten config still loads");
}

/// R-01: a rule's `weight` is now emitted by the writer, so the rendered
/// baseline (which decides `has_unsaved_changes` and which files a save
/// rewrites) is produced by the same function that reads it back. A file
/// with weights must therefore show no change when nothing was edited, and
/// a save with no edits must write nothing.
#[test]
fn a_file_with_weights_shows_no_spurious_change() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ws, root_text) = load(dir.path(), false);

    assert!(
        !ws.has_unsaved_changes(),
        "weights make a fresh load look edited"
    );
    let saved = ws.save().unwrap();
    assert!(saved.changed_files.is_empty(), "{:?}", saved.changed_files);
    assert!(saved.diff_summary.is_empty(), "{:?}", saved.diff_summary);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("rules.toml")).unwrap(),
        with_eol(RULE_SET, false)
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("apimock.toml")).unwrap(),
        root_text
    );

    // And after a real save, the workspace is clean again.
    ws.apply(EditCommand::UpdateRootSetting {
        key: RootSettingKey::ListenerPort,
        value: EditValue::Integer(4000),
    })
    .unwrap();
    assert!(ws.has_unsaved_changes());
    let saved = ws.save().unwrap();
    assert_eq!(saved.changed_files.len(), 1, "{:?}", saved.changed_files);
    assert!(!ws.has_unsaved_changes());
}

// ---------------------------------------------------------------------
// The fixture covers every key the docs list.
// ---------------------------------------------------------------------

/// Every `| `field` | … |` row of the markdown tables in `md`, with the
/// heading it sits under. Code fences are skipped (a `#` comment inside
/// one is not a heading).
fn documented_fields(md: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut heading = String::new();
    let mut in_fence = false;
    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if line.starts_with('#') {
            heading = line.trim_start_matches('#').trim().to_owned();
            continue;
        }
        if let Some(row) = line.strip_prefix("| ") {
            let first = row.split('|').next().unwrap_or("").trim();
            let inner = first.trim_matches('`');
            if first.len() > 2
                && first.starts_with('`')
                && first.ends_with('`')
                && !inner.contains('`')
            {
                out.push((heading.clone(), inner.to_owned()));
            }
        }
    }
    out
}

/// Does `path` exist somewhere in `value`, looking through arrays (a rule
/// set's `[[rules]]` is an array of tables).
fn has_path(value: &toml::Value, path: &[&str]) -> bool {
    match (value, path) {
        (_, []) => true,
        (toml::Value::Array(items), _) => items.iter().any(|item| has_path(item, path)),
        (toml::Value::Table(t), [head, rest @ ..]) => {
            t.get(*head).is_some_and(|v| has_path(v, rest))
        }
        _ => false,
    }
}

/// A user-chosen name in a documented field (`headers.<name>`,
/// `body.json."<dotted.path>"`) stands for "a table of these"; require
/// the table, with something in it.
fn without_placeholder(field: &str) -> Vec<&str> {
    let cut = field.find('<').map_or(field, |i| &field[..i]);
    cut.trim_end_matches(['.', '"']).split('.').collect()
}

#[test]
fn the_fixture_sets_every_key_the_docs_list() {
    let root: toml::Value = toml::from_str(&ROOT_TEMPLATE.replace("@DIR@", "/d")).unwrap();
    let rule_set: toml::Value = toml::from_str(RULE_SET).unwrap();

    // (heading in the doc, where that heading's table lives in the file)
    let root_headings: &[(&str, &[&str])] = &[
        ("`[listener]`", &["listener"]),
        ("`[listener.tls]`", &["listener", "tls"]),
        ("`[log]`", &["log"]),
        ("`[service]`", &["service"]),
        ("`[file_tree_view]`", &["file_tree_view"]),
    ];
    let rule_set_headings: &[(&str, &[&str])] = &[
        ("`[prefix]`", &["prefix"]),
        ("`when.request`", &["rules", "when", "request"]),
        ("`respond`", &["rules", "respond"]),
    ];

    let mut checked = 0;
    let mut check = |doc: &str, headings: &[(&str, &[&str])], file: &toml::Value, name: &str| {
        let fields = documented_fields(doc);
        for (heading, base) in headings {
            let under: Vec<_> = fields.iter().filter(|(h, _)| h == heading).collect();
            assert!(
                !under.is_empty(),
                "{name}: no table rows found under {heading}; \
                 did the page's headings change? This test would otherwise pass vacuously"
            );
            for (_, field) in under {
                let mut path: Vec<&str> = base.to_vec();
                path.extend(without_placeholder(field));
                assert!(
                    has_path(file, &path),
                    "{name}: the docs list `{field}` under {heading}, but the fixture does not \
                     set `{}`. Add it to crates/apimock-config/tests/fixtures/maximal/",
                    path.join(".")
                );
                checked += 1;
            }
        }
    };
    check(ROOT_DOC, root_headings, &root, "apimock.toml");
    check(RULE_SET_DOC, rule_set_headings, &rule_set, "rule set");

    // The keys the page documents in prose or in its example, not in a
    // table. Listed here by hand, because there is nothing to parse.
    for path in [
        &["strategy"][..],
        &["default", "delay_response_milliseconds"],
        &["guard"],
        &["rules", "priority"],
        &["rules", "weight"],
    ] {
        assert!(
            has_path(&rule_set, path),
            "fixture lacks {}",
            path.join(".")
        );
        checked += 1;
    }

    assert!(checked >= 40, "only {checked} documented keys were checked");
}
