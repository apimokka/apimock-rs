//! Workspace::save — persistence, round-trip, atomic write, diff tracking.

use super::common::{make_workspace, make_workspace_with_headers_and_body};
use crate::{
    view::{ConfigFileKind, EditCommand, EditValue, NodeKind},
    workspace::Workspace,
};

// -----------------------------------------------------------------
// RFC 056 — in-place save. This is the point of the whole RFC and,
// per the handoff, was written before any of the implementation below
// it existed.
// -----------------------------------------------------------------

#[test]
fn save_preserves_comments_blank_lines_and_key_order() {
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();

    let rs_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/api/users\"\n",
        "respond = { text = \"ok\" }\n",
    );
    let rs_path = dir.path().join("apimock-rule-set.toml");
    std::fs::write(&rs_path, rs_toml).unwrap();

    // Hand-written: a leading comment, a blank line, and the
    // listener's keys in a person's own order (port before
    // ip_address) — none of which canonical rendering would ever
    // produce (it sorts keys and never emits comments).
    let root_toml = format!(
        "# apimock config -- do not remove this comment\n\
         [listener]\n\
         port = 3001\n\
         ip_address = \"127.0.0.1\"\n\
         \n\
         # rule sets live in a sibling file\n\
         [service]\n\
         rule_sets = [\"{}\"]\n\
         fallback_respond_dir = \"{}\"\n",
        rs_path.file_name().unwrap().to_string_lossy(),
        fallback.file_name().unwrap().to_string_lossy(),
    );
    let root_path = dir.path().join("apimock.toml");
    std::fs::write(&root_path, &root_toml).unwrap();

    let mut ws = Workspace::load(root_path.clone()).expect("load");
    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(9999),
    })
    .expect("apply");
    ws.save().expect("save");

    let saved = std::fs::read_to_string(&root_path).expect("read back");

    assert!(
        saved.contains("# apimock config -- do not remove this comment"),
        "leading comment must survive:\n{saved}"
    );
    assert!(
        saved.contains("# rule sets live in a sibling file"),
        "mid-file comment must survive:\n{saved}"
    );
    assert!(
        saved.contains("\n\n"),
        "the blank line separating sections must survive:\n{saved}"
    );
    // Hand-chosen key order (port before ip_address) must survive —
    // canonical rendering would sort alphabetically instead.
    let port_pos = saved.find("port = 9999").expect("edited value present");
    let ip_pos = saved.find("ip_address").expect("untouched key present");
    assert!(
        port_pos < ip_pos,
        "key order (port before ip_address) must survive:\n{saved}"
    );
    assert!(
        !saved.contains("port = 3001"),
        "the old value must be gone:\n{saved}"
    );
}

#[test]
fn save_of_one_file_leaves_the_other_byte_identical() {
    let (_dir, root) = make_workspace();
    let rs_path = root.parent().unwrap().join("apimock-rule-set.toml");
    let rs_before = std::fs::read_to_string(&rs_path).expect("read rule set before save");

    let mut ws = Workspace::load(root.clone()).expect("load");
    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(4242),
    })
    .expect("apply");
    let save = ws.save().expect("save");

    assert!(
        save.changed_files.iter().any(|p| p == &root),
        "expected the root file to be the one that changed"
    );
    assert!(
        !save.changed_files.iter().any(|p| p == &rs_path),
        "the untouched rule-set file must not be reported as changed"
    );

    let rs_after = std::fs::read_to_string(&rs_path).expect("read rule set after save");
    assert_eq!(
        rs_before, rs_after,
        "a save touching only the root file must leave the rule-set file byte-identical"
    );
}

#[test]
fn save_refuses_rather_than_overwrites_a_file_changed_on_disk() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root.clone()).expect("load");

    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(5555),
    })
    .expect("apply");

    // Someone else edits the same file on disk after our load().
    let external_edit = "[listener]\nip_address = \"127.0.0.1\"\nport = 7777\n\n[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \"fallback\"\n";
    std::fs::write(&root, external_edit).expect("simulate external edit");

    let err = ws
        .save()
        .expect_err("save must refuse an externally-changed file");
    assert!(
        matches!(&err, crate::error::SaveError::Conflict { path } if path == &root),
        "expected SaveError::Conflict{{ path: root }}, got {err:?}"
    );

    // The external edit must survive untouched — the worst outcome is
    // silently discarding it, which is exactly what this refusal
    // prevents.
    let on_disk = std::fs::read_to_string(&root).expect("read after refused save");
    assert_eq!(
        on_disk, external_edit,
        "a refused save must not touch the file it refused to overwrite"
    );
}

#[test]
fn save_reports_a_read_failure_distinctly_from_a_conflict() {
    // REVIEW-001 §4: a read failure ahead of the conflict check (here,
    // the file having been deleted out from under us) must not be
    // folded into `Conflict` — `Conflict`'s message tells the caller to
    // reload, which is not a fix for "the file is gone" any more than
    // it would be for a permission error.
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root.clone()).expect("load");

    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(6161),
    })
    .expect("apply");

    std::fs::remove_file(&root).expect("simulate the file vanishing externally");

    let err = ws
        .save()
        .expect_err("save must fail when it can't confirm the file is unchanged");
    assert!(
        matches!(&err, crate::error::SaveError::Read { path, .. } if path == &root),
        "expected SaveError::Read{{ path: root, .. }}, got {err:?}"
    );
}

#[test]
fn save_add_then_remove_header_condition_round_trips_in_place() {
    // The riskiest part of in-place reconciliation is a dynamically
    // keyed sub-table (headers, here) gaining a key and later losing
    // it, across two *separate, real* saves — stale-key removal
    // exercised against a document that in-place mutation itself
    // wrote, not just against the freshly-loaded original file.
    //
    // Both edits target the header condition this test adds — RFC 016
    // per-condition NodeIds aren't seeded at load() (only assigned
    // when `AddHeaderCondition` creates one), so there's no supported
    // way to address the fixture's own pre-existing "x-api-key"
    // condition here. That's an existing RFC 016 addressing gap, not
    // something this RFC touches; "x-api-key" instead serves this
    // test as the untouched sibling key that must survive both saves.
    let (_dir, root) = make_workspace_with_headers_and_body();
    let mut ws = Workspace::load(root.clone()).expect("load");
    let rule_id = ws
        .snapshot()
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Rule))
        .unwrap()
        .id;

    let add_result = ws
        .apply(EditCommand::AddHeaderCondition {
            rule_id,
            condition: crate::view::HeaderConditionPayload {
                name: "x-new-header".to_owned(),
                op: crate::view::HeaderOp::Equal,
                value: Some("new-value".to_owned()),
            },
        })
        .expect("add header condition");
    let cond_id = add_result
        .changed_nodes
        .iter()
        .copied()
        .find(|id| *id != rule_id)
        .expect("AddHeaderCondition returns the new condition's id");
    ws.save().expect("save after add");

    // Round 1, confirmed via a fresh load: the insert-new-key path
    // persisted, and the fixture's own header condition is untouched.
    let ws_after_add = Workspace::load(root.clone()).expect("reload after add");
    let rule_after_add = &ws_after_add.config().service.rule_sets[0].rules[0];
    let headers_after_add = rule_after_add
        .when
        .request
        .headers
        .as_ref()
        .expect("headers");
    assert!(
        headers_after_add.0.contains_key("x-new-header"),
        "newly added header condition must survive a save + reload"
    );
    assert!(
        headers_after_add.0.contains_key("x-api-key"),
        "the fixture's original header condition must still be present"
    );

    // Round 2, same still-open `ws` (its `original_text` now reflects
    // what round 1 actually wrote): remove the condition just added.
    ws.apply(EditCommand::RemoveHeaderCondition { id: cond_id })
        .expect("remove the condition added above");
    ws.save().expect("save after remove");

    let ws_after_remove = Workspace::load(root).expect("reload after remove");
    let rule_after_remove = &ws_after_remove.config().service.rule_sets[0].rules[0];
    let headers_after_remove = rule_after_remove
        .when
        .request
        .headers
        .as_ref()
        .expect("x-api-key keeps the headers table alive");
    assert!(
        !headers_after_remove.0.contains_key("x-new-header"),
        "the removed header condition must not come back after save + reload"
    );
    assert!(
        headers_after_remove.0.contains_key("x-api-key"),
        "the untouched sibling header condition must survive the other one's removal"
    );
}

// -----------------------------------------------------------------
// 5.2.0 — save() + diff tests
// -----------------------------------------------------------------

#[test]
fn save_is_noop_when_nothing_changed() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");

    // Newly-loaded workspace has no edits → save should write 0 files.
    // (Actually, if the rendered output differs from the on-disk
    // formatting the baseline check still skips writing because
    // baseline holds the on-disk text byte-for-byte. So even though
    // round-trip isn't formatting-stable, the no-edit path is safe.)
    let result = ws.save().expect("save");
    assert_eq!(result.changed_files.len(), 0);
    assert_eq!(result.diff_summary.len(), 0);
    assert!(!result.requires_reload);
}

#[test]
fn save_persists_rule_set_edit_and_round_trips() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root.clone()).expect("load");

    // Find the first rule and update its text.
    let snap = ws.snapshot();
    let resp_id = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Respond))
        .unwrap()
        .id;

    ws.apply(EditCommand::UpdateRespond {
        id: resp_id,
        respond: crate::view::RespondPayload {
            text: Some("HELLO_FROM_SAVE".to_owned()),
            ..Default::default()
        },
    })
    .expect("apply");

    assert!(ws.has_unsaved_changes());

    let save = ws.save().expect("save");
    assert!(save.changed_files.iter().any(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().contains("apimock-rule-set"))
            .unwrap_or(false)
    }));
    assert!(!save.diff_summary.is_empty());
    // After save, has_unsaved_changes is now false (baseline updated).
    assert!(!ws.has_unsaved_changes());

    // Re-load from disk: the edit must be visible.
    let ws2 = Workspace::load(root).expect("reload");
    let snap2 = ws2.snapshot();
    let any_respond_has_text = snap2
        .files
        .iter()
        .flat_map(|f| f.nodes.iter())
        .any(|n| n.display_name.contains("HELLO_FROM_SAVE"));
    assert!(
        any_respond_has_text,
        "expected the saved text to round-trip through disk"
    );
}

#[test]
fn save_persists_root_edit_and_flags_reload() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root.clone()).expect("load");

    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(8888),
    })
    .expect("apply");

    let save = ws.save().expect("save");
    assert!(
        save.changed_files.iter().any(|p| p == &root),
        "root file should be in changed_files"
    );
    assert!(
        save.requires_reload,
        "listener port change should request reload"
    );

    // Diff summary should mention the root node.
    assert!(
        save.diff_summary
            .iter()
            .any(|d| matches!(d.kind, crate::view::DiffKind::Updated))
    );

    // Round-trip: re-load and verify the port stuck.
    let ws2 = Workspace::load(root).expect("reload");
    assert_eq!(
        ws2.config.listener.as_ref().unwrap().port,
        8888,
        "port edit must round-trip through disk"
    );
}

#[test]
fn save_atomic_write_does_not_corrupt_on_concurrent_read() {
    // We can't really exercise a race here, but we *can* verify the
    // file is always parseable after save — i.e. there's no observable
    // moment where the file is empty / half-written.
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root.clone()).expect("load");

    // Make a bunch of edits then save.
    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(9001),
    })
    .expect("apply");

    let _ = ws.save().expect("save");

    // File should be fully readable + parseable.
    let text = std::fs::read_to_string(&root).expect("read after save");
    assert!(text.contains("port"));
    assert!(text.contains("9001"));
    let _: toml::Value = toml::from_str(&text).expect("post-save TOML must parse");
}

#[test]
fn has_unsaved_changes_tracks_edit_state() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");
    assert!(!ws.has_unsaved_changes());

    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(8080),
    })
    .expect("apply");
    assert!(ws.has_unsaved_changes());

    let _ = ws.save().expect("save");
    assert!(!ws.has_unsaved_changes());
}

// -----------------------------------------------------------------
// 5.3.0 — Step 5 routing snapshot enrichment
// -----------------------------------------------------------------

#[test]
fn snapshot_routes_populated_from_rule_sets() {
    let (_dir, root) = make_workspace();
    let ws = Workspace::load(root).expect("load");
    let snap = ws.snapshot();

    // Rule sets in routes view should mirror the in-memory model.
    assert_eq!(snap.routes.rule_sets.len(), 1);
    let rs_view = &snap.routes.rule_sets[0];
    assert_eq!(rs_view.index, 0);
    // make_workspace() creates 2 rules.
    assert_eq!(rs_view.rules.len(), 2);

    // First rule: url_path "/api/users", text response.
    let r0 = &rs_view.rules[0];
    let url_path = r0.when.url_path.as_ref().expect("url_path present");
    assert_eq!(url_path.value, "/api/users");
    // op should be a TOML-form name, not a Display-formatted spaces-string.
    assert_eq!(url_path.op, "equal");
    // No method constraint.
    assert!(r0.when.method.is_none());
}

#[test]
fn snapshot_when_view_summary_has_method_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();
    let rs_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/api/v1\"\n",
        "when.request.method = \"GET\"\n",
        "respond = { text = \"ok\" }\n",
    );
    std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
    std::fs::write(
        dir.path().join("apimock.toml"),
        format!(
            "[listener]\nip_address = \"127.0.0.1\"\nport = 3001\n[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \"{}\"\n",
            fallback.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();

    let ws = Workspace::load(dir.path().join("apimock.toml")).unwrap();
    let snap = ws.snapshot();
    let when = &snap.routes.rule_sets[0].rules[0].when;
    assert_eq!(when.method.as_deref(), Some("GET"));
    let summary = when.summary();
    assert!(summary.contains("GET"));
    assert!(summary.contains("/api/v1"));
}

#[test]
fn snapshot_file_tree_depth1_eager() {
    // Set up a fallback dir with both a file and a subdirectory.
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();
    std::fs::write(fallback.join("users.json"), "{}").unwrap();
    let subdir = fallback.join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::write(subdir.join("hidden.json"), "{}").unwrap();

    let rs_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/api/users\"\n",
        "respond = { text = \"ok\" }\n",
    );
    std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
    std::fs::write(
        dir.path().join("apimock.toml"),
        format!(
            "[listener]\nip_address = \"127.0.0.1\"\nport = 3001\n[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \"{}\"\n",
            fallback.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();

    let ws = Workspace::load(dir.path().join("apimock.toml")).unwrap();
    let snap = ws.snapshot();
    let tree = snap.routes.file_tree.as_ref().expect("file tree present");

    // Depth-1: see both users.json (file) and subdir (directory).
    assert_eq!(tree.entries.len(), 2);
    let names: Vec<&str> = tree.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"users.json"));
    assert!(names.contains(&"subdir"));

    // Subdirectory must NOT be expanded — children empty Vec, not the
    // contents of subdir.
    let subdir_entry = tree.entries.iter().find(|e| e.name == "subdir").unwrap();
    assert!(matches!(
        subdir_entry.kind,
        apimock_routing::view::FileNodeKind::Directory
    ));
    let children = subdir_entry
        .children
        .as_ref()
        .expect("directory has Some(children) flag");
    assert!(
        children.is_empty(),
        "subdirectory should not be eagerly expanded"
    );

    // File node carries a route_hint.
    let file_entry = tree
        .entries
        .iter()
        .find(|e| e.name == "users.json")
        .unwrap();
    assert_eq!(file_entry.route_hint.as_deref(), Some("/users"));
    assert!(file_entry.children.is_none());

    // list_directory expands the subdirectory on demand.
    let expanded = ws.list_directory(std::path::Path::new(&subdir_entry.path));
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].name, "hidden.json");
}

#[test]
fn save_per_rule_diff_after_rule_edit() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root.clone()).expect("load");

    // Edit just the first rule's respond.
    let snap = ws.snapshot();
    let resp_id = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Respond))
        .unwrap()
        .id;

    ws.apply(EditCommand::UpdateRespond {
        id: resp_id,
        respond: crate::view::RespondPayload {
            text: Some("CHANGED".to_owned()),
            ..Default::default()
        },
    })
    .expect("apply");

    let save = ws.save().expect("save");
    // Per-rule diff: the diff_summary should include exactly one rule
    // entry, not just a rule-set-level one.
    let rule_diffs: Vec<_> = save
        .diff_summary
        .iter()
        .filter(|d| d.summary.contains("rule #"))
        .collect();
    assert!(
        !rule_diffs.is_empty(),
        "expected per-rule diff entries; got {:?}",
        save.diff_summary
    );

    // Specifically rule #1 should be Updated (we edited rules[0]).
    let updated_rule_1 = rule_diffs.iter().find(|d| {
        d.summary.contains("rule #1") && matches!(d.kind, crate::view::DiffKind::Updated)
    });
    assert!(
        updated_rule_1.is_some(),
        "expected an Updated entry for rule #1; got {:?}",
        save.diff_summary
    );
}

#[test]
fn snapshot_script_routes_present_when_middlewares_configured() {
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();

    // Write a no-op Rhai middleware. Just needs to parse — the
    // workspace reads service.middlewares for the script_routes view
    // but doesn't compile Rhai itself (server does that).
    let mw = dir.path().join("noop.rhai");
    std::fs::write(&mw, "// noop\n").unwrap();

    let rs_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/x\"\n",
        "respond = { text = \"ok\" }\n",
    );
    std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
    std::fs::write(
        dir.path().join("apimock.toml"),
        format!(
            "[listener]\nip_address = \"127.0.0.1\"\nport = 3001\n[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nmiddlewares = [\"noop.rhai\"]\nfallback_respond_dir = \"{}\"\n",
            fallback.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();

    let ws = Workspace::load(dir.path().join("apimock.toml")).unwrap();
    let snap = ws.snapshot();
    assert_eq!(snap.routes.script_routes.len(), 1);
    assert_eq!(snap.routes.script_routes[0].index, 0);
    assert_eq!(snap.routes.script_routes[0].source_file, "noop.rhai");
    assert_eq!(snap.routes.script_routes[0].display_name, "noop.rhai");
}

// -----------------------------------------------------------------
// RFC 058 — `respond_dir` no longer grows on every save. Each test
// forces a *real* save (adding a rule, so the rendered output
// genuinely differs from baseline) rather than a no-edit save, since
// a no-op save never wrote the file at all, before or after this RFC
// — the bug only ever showed up on an edit that was already going to
// touch the file.
// -----------------------------------------------------------------

fn workspace_with_prefix(prefix_toml: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let rs_toml = format!(
        "{}[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = {{ text = \"ok\" }}\n",
        prefix_toml
    );
    let rs_path = dir.path().join("apimock-rule-set.toml");
    std::fs::write(&rs_path, rs_toml).unwrap();
    let root_toml =
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n";
    let root_path = dir.path().join("apimock.toml");
    std::fs::write(&root_path, root_toml).unwrap();
    (dir, root_path)
}

fn add_a_rule(ws: &mut Workspace, path: &str) {
    let parent = ws.rule_set_id_at(0).expect("rule set 0 exists");
    ws.apply(EditCommand::AddRule {
        parent,
        rule: crate::view::RulePayload {
            url_path: Some(path.to_owned()),
            url_path_op: None,
            method: None,
            priority: None,
            headers: None,
            body: None,
            respond: crate::view::RespondPayload {
                text: Some("ok".to_owned()),
                ..Default::default()
            },
        },
    })
    .expect("apply");
}

fn respond_dir_line(rule_set_toml: &str) -> Option<&str> {
    rule_set_toml
        .lines()
        .find(|l| l.trim_start().starts_with("respond_dir"))
}

#[test]
fn respond_dir_is_a_fixed_point_across_repeated_real_saves() {
    let (_dir, root) = workspace_with_prefix("[prefix]\nrespond_dir = \".\"\n\n");
    let rs_path = root.parent().unwrap().join("apimock-rule-set.toml");
    let mut ws = Workspace::load(root).expect("load");

    for i in 0..3 {
        add_a_rule(&mut ws, &format!("/added-{i}"));
        let save = ws.save().expect("save");
        assert!(
            !save.changed_files.is_empty(),
            "each round must be a real save, not a no-op — round {i}"
        );
        let text = std::fs::read_to_string(&rs_path).unwrap();
        assert_eq!(
            respond_dir_line(&text),
            Some("respond_dir = \".\""),
            "respond_dir must not grow after round {i}:\n{text}"
        );
    }
}

#[test]
fn no_prefix_section_stays_absent_after_a_real_save() {
    let (_dir, root) = workspace_with_prefix("");
    let rs_path = root.parent().unwrap().join("apimock-rule-set.toml");
    let mut ws = Workspace::load(root).expect("load");

    add_a_rule(&mut ws, "/new");
    let save = ws.save().expect("save");
    assert!(!save.changed_files.is_empty());

    let text = std::fs::read_to_string(&rs_path).unwrap();
    assert!(
        !text.contains("[prefix]"),
        "a rule set that never had [prefix] must not gain one from a save:\n{text}"
    );
}

#[test]
fn respond_dir_responses_round_trips_unchanged_across_a_real_save() {
    let (dir, root) = workspace_with_prefix("[prefix]\nrespond_dir = \"responses\"\n\n");
    std::fs::create_dir(dir.path().join("responses")).unwrap();
    let rs_path = root.parent().unwrap().join("apimock-rule-set.toml");
    let mut ws = Workspace::load(root).expect("load");

    add_a_rule(&mut ws, "/new");
    ws.save().expect("save");

    let text = std::fs::read_to_string(&rs_path).unwrap();
    assert_eq!(respond_dir_line(&text), Some("respond_dir = \"responses\""));
}

#[test]
fn respond_dir_dotted_relative_path_never_collapses() {
    let (dir, root) = workspace_with_prefix("[prefix]\nrespond_dir = \"./responses\"\n\n");
    std::fs::create_dir(dir.path().join("responses")).unwrap();
    let rs_path = root.parent().unwrap().join("apimock-rule-set.toml");
    let mut ws = Workspace::load(root).expect("load");

    add_a_rule(&mut ws, "/new");
    ws.save().expect("save");

    let text = std::fs::read_to_string(&rs_path).unwrap();
    assert_eq!(
        respond_dir_line(&text),
        Some("respond_dir = \"./responses\""),
        "a real path must never be touched by the pure-dot collapse:\n{text}"
    );
}

#[test]
fn a_previously_grown_respond_dir_collapses_on_the_next_real_save() {
    let (_dir, root) = workspace_with_prefix("[prefix]\nrespond_dir = \"././.\"\n\n");
    let rs_path = root.parent().unwrap().join("apimock-rule-set.toml");
    let mut ws = Workspace::load(root).expect("load");

    add_a_rule(&mut ws, "/new");
    ws.save().expect("save");

    let text = std::fs::read_to_string(&rs_path).unwrap();
    assert_eq!(
        respond_dir_line(&text),
        Some("respond_dir = \".\""),
        "a purely-./-segments value must collapse to '.' the next time it's saved:\n{text}"
    );
}

#[test]
fn saving_an_unrelated_file_does_not_touch_a_still_grown_respond_dir() {
    // The narrow repair must never be a standalone rewrite — it rides
    // along with a save the user already asked for, and only for the
    // file that save actually touches.
    let (dir, root) = workspace_with_prefix("[prefix]\nrespond_dir = \"././.\"\n\n");
    let rs_path = root.parent().unwrap().join("apimock-rule-set.toml");
    let before = std::fs::read_to_string(&rs_path).unwrap();

    let mut ws = Workspace::load(root).expect("load");
    // Touch the *root* config, not the rule set — forces a real save
    // that must leave apimock-rule-set.toml completely alone.
    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(9191),
    })
    .expect("apply");
    let save = ws.save().expect("save");
    assert!(
        save.changed_files
            .iter()
            .all(|p| p.file_name().unwrap() != "apimock-rule-set.toml"),
        "only the root config should have changed: {:?}",
        save.changed_files
    );

    let after = std::fs::read_to_string(&rs_path).unwrap();
    assert_eq!(
        before, after,
        "a save that never touches this file must leave it byte-identical, \
         grown respond_dir and all — the repair is not a standalone rewrite"
    );
    let _ = dir;
}

#[test]
fn a_hand_written_rule_set_with_a_prefix_section_survives_a_real_save() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join("responses")).unwrap();
    let rs_toml = "# a person's own comment above their prefix\n[prefix]\nrespond_dir = \"responses\"\n\n[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n";
    let rs_path = dir.path().join("apimock-rule-set.toml");
    std::fs::write(&rs_path, rs_toml).unwrap();
    let root_toml =
        "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n";
    let root_path = dir.path().join("apimock.toml");
    std::fs::write(&root_path, root_toml).unwrap();

    let mut ws = Workspace::load(root_path).expect("load");
    add_a_rule(&mut ws, "/new");
    ws.save().expect("save");

    let after = std::fs::read_to_string(&rs_path).unwrap();
    assert!(
        after.contains("# a person's own comment above their prefix"),
        "RFC 056's comment-preservation guarantee, re-proved with a [prefix] section present:\n{after}"
    );
    assert_eq!(
        respond_dir_line(&after),
        Some("respond_dir = \"responses\"")
    );
}
