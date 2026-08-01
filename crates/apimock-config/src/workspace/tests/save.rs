//! Workspace::save — persistence, round-trip, atomic write, diff tracking.

use super::common::make_workspace;
use crate::{
    view::{ConfigFileKind, EditCommand, EditValue, NodeKind},
    workspace::Workspace,
};

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
