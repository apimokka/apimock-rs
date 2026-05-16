//! Tests for `super::Workspace`.
//!
//! # Why this is a sibling file rather than an inline `#[cfg(test)] mod`
//!
//! `workspace.rs` is large enough that mixing tests with implementation
//! made navigating it tedious. Extracting the test body to
//! `workspace/tests.rs` keeps the implementation file focused and lets
//! editors / IDEs jump between the two halves quickly. The behaviour is
//! identical to a sibling-`mod tests` arrangement; the only difference
//! is file layout.

use super::*;
use crate::error::ApplyError;
use crate::view::{
    ConfigFileKind, ConfigNodeView, EditCommand, EditValue, NodeId, NodeKind,
    Severity,
};
use std::fs;
use std::path::PathBuf;

/// Create a minimal on-disk workspace and return the tempdir guard
/// + absolute path to the root apimock.toml.
fn make_workspace() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");

    // Rule-set file with one file-based rule + one text rule.
    let rule_set_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/api/users\"\n",
        "respond = { text = \"ok\" }\n",
        "\n",
        "[[rules]]\n",
        "when.request.url_path = \"/api/health\"\n",
        "respond = { status = 204 }\n",
    );
    let rs_path = dir.path().join("apimock-rule-set.toml");
    fs::write(&rs_path, rule_set_toml).unwrap();

    // Fallback dir so validation doesn't fail.
    let fallback = dir.path().join("fallback");
    fs::create_dir_all(&fallback).unwrap();

    let root_toml = format!(
        "[listener]\n\
         ip_address = \"127.0.0.1\"\n\
         port = 3001\n\
         \n\
         [service]\n\
         rule_sets = [\"{}\"]\n\
         fallback_respond_dir = \"{}\"\n",
        rs_path.file_name().unwrap().to_string_lossy(),
        fallback.file_name().unwrap().to_string_lossy(),
    );
    let root_path = dir.path().join("apimock.toml");
    fs::write(&root_path, root_toml).unwrap();

    (dir, root_path)
}

#[test]
fn load_returns_workspace_with_seeded_ids() {
    let (_dir, root) = make_workspace();
    let ws = Workspace::load(root).expect("load");
    // Root + fallback + 1 rule set + 2 rules + 2 responds = 7 nodes
    // registered in the id index.
    let n = ws.ids.id_to_address.len();
    assert!(n >= 7, "expected at least 7 seeded nodes, got {}", n);
}

#[test]
fn snapshot_shapes_match_spec() {
    let (_dir, root) = make_workspace();
    let ws = Workspace::load(root).expect("load");
    let snap = ws.snapshot();

    // Should have a root file view + a rule-set file view.
    assert!(!snap.files.is_empty());
    let root_view = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::Root))
        .expect("root file present");
    assert!(root_view.nodes.iter().any(|n| n.display_name == "apimock.toml"));

    let rs_view = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .expect("rule set present");
    // rule-set + 2 rules + 2 responds
    assert!(rs_view.nodes.len() >= 5);
    assert!(rs_view.nodes.iter().any(|n| matches!(n.kind, NodeKind::RuleSet)));
    assert!(rs_view.nodes.iter().any(|n| matches!(n.kind, NodeKind::Rule)));
    assert!(rs_view.nodes.iter().any(|n| matches!(n.kind, NodeKind::Respond)));
}

#[test]
fn snapshot_nodes_have_unique_ids() {
    let (_dir, root) = make_workspace();
    let ws = Workspace::load(root).expect("load");
    let snap = ws.snapshot();

    let mut seen = std::collections::HashSet::new();
    for file in &snap.files {
        for node in &file.nodes {
            assert!(
                seen.insert(node.id),
                "duplicate NodeId in snapshot: {}",
                node.id
            );
        }
    }
}

#[test]
fn snapshot_respond_node_displays_rule_content() {
    let (_dir, root) = make_workspace();
    let ws = Workspace::load(root).expect("load");
    let snap = ws.snapshot();

    let rs_view = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap();

    // First respond should say "text: ok"; second "status: 204".
    let responds: Vec<_> = rs_view
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Respond))
        .collect();
    assert_eq!(responds.len(), 2);
    assert!(responds[0].display_name.contains("ok"));
    assert!(responds[1].display_name.contains("204"));
}

#[test]
fn apply_add_rule_to_existing_rule_set() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");

    // Find the rule-set's NodeId.
    let snap = ws.snapshot();
    let rs_node = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .unwrap();
    let rs_id = rs_node.id;

    let before = ws.config.service.rule_sets[0].rules.len();
    let result = ws
        .apply(EditCommand::AddRule {
            parent: rs_id,
            rule: crate::view::RulePayload {
                url_path: Some("/new/rule".to_owned()),
                method: None,
                respond: crate::view::RespondPayload {
                    text: Some("hi".to_owned()),
                    ..Default::default()
                },
            },
        })
        .expect("apply AddRule");
    assert!(result.requires_reload);
    assert_eq!(ws.config.service.rule_sets[0].rules.len(), before + 1);
    // changed_nodes should include the parent + new rule + new respond.
    assert!(result.changed_nodes.len() >= 3);
}

#[test]
fn apply_update_respond_changes_content() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");

    // Grab the first respond node's id.
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
            text: Some("updated-text".to_owned()),
            ..Default::default()
        },
    })
    .expect("apply UpdateRespond");

    // The respond display should now reflect the new text.
    let snap2 = ws.snapshot();
    let new_display = snap2
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.id == resp_id)
        .unwrap()
        .display_name
        .clone();
    assert!(new_display.contains("updated-text"), "got: {}", new_display);
}

#[test]
fn apply_delete_rule_shifts_successors_and_preserves_ids() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");

    // Grab the two rule NodeIds before edit.
    let snap = ws.snapshot();
    let rs_view = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap();
    let rules: Vec<NodeId> = rs_view
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Rule))
        .map(|n| n.id)
        .collect();
    assert_eq!(rules.len(), 2);

    let first_id = rules[0];
    let second_id = rules[1];

    ws.apply(EditCommand::DeleteRule { id: first_id })
        .expect("delete");

    // After delete: only one rule remains, and its id should still
    // be `second_id` (ID stability across shift).
    let snap2 = ws.snapshot();
    let rs_view2 = snap2
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap();
    let rules_after: Vec<NodeId> = rs_view2
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Rule))
        .map(|n| n.id)
        .collect();
    assert_eq!(rules_after.len(), 1);
    assert_eq!(
        rules_after[0], second_id,
        "second rule's NodeId should survive deletion of the first"
    );
}

#[test]
fn apply_move_rule_preserves_ids() {
    // Build a workspace with 3 rules so there's room to move.
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();
    let rs_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/one\"\n",
        "respond = { text = \"a\" }\n",
        "\n",
        "[[rules]]\n",
        "when.request.url_path = \"/two\"\n",
        "respond = { text = \"b\" }\n",
        "\n",
        "[[rules]]\n",
        "when.request.url_path = \"/three\"\n",
        "respond = { text = \"c\" }\n",
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

    let mut ws = Workspace::load(dir.path().join("apimock.toml")).unwrap();

    let snap = ws.snapshot();
    let rs_view = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap();
    let rule_ids: Vec<NodeId> = rs_view
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Rule))
        .map(|n| n.id)
        .collect();
    assert_eq!(rule_ids.len(), 3);
    let [id_a, id_b, id_c] = [rule_ids[0], rule_ids[1], rule_ids[2]];

    // Move first to last.
    ws.apply(EditCommand::MoveRule {
        id: id_a,
        new_index: 2,
    })
    .expect("move");

    // Verify order by display_name (url_path): should be b, c, a.
    let snap2 = ws.snapshot();
    let rs_view2 = snap2
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap();
    let new_rules: Vec<&ConfigNodeView> = rs_view2
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Rule))
        .collect();
    assert_eq!(new_rules.len(), 3);
    assert_eq!(new_rules[0].id, id_b);
    assert_eq!(new_rules[1].id, id_c);
    assert_eq!(new_rules[2].id, id_a);
}

#[test]
fn apply_update_root_setting_port() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");
    ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(9999),
    })
    .expect("update port");
    assert_eq!(ws.config.listener.as_ref().unwrap().port, 9999);
}

#[test]
fn apply_update_root_setting_bad_port_is_invalid_payload() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");
    let result = ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::ListenerPort,
        value: EditValue::Integer(70_000),
    });
    assert!(matches!(result, Err(ApplyError::InvalidPayload { .. })));
}

#[test]
fn apply_unknown_node_id() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");
    let result = ws.apply(EditCommand::DeleteRule { id: NodeId::new() });
    assert!(matches!(result, Err(ApplyError::UnknownNode { .. })));
}

#[test]
fn apply_wrong_kind_error() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");
    // Grab a respond id and try to delete it as a rule.
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
    let result = ws.apply(EditCommand::DeleteRule { id: resp_id });
    assert!(matches!(result, Err(ApplyError::WrongNodeKind { .. })));
}

#[test]
fn validate_surfaces_per_node_diagnostics() {
    // Build a workspace with an invalid rule (file_path + text).
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();
    let rs_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/bad\"\n",
        "respond = { file_path = \"nonexistent.json\", text = \"x\" }\n",
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

    // Construct Workspace manually, bypassing Config::new's validation
    // (which would reject the invalid respond at load time).
    // We do this by first writing a valid rule-set, loading, then
    // mutating through apply.
    std::fs::write(
        dir.path().join("apimock-rule-set.toml"),
        concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/bad\"\n",
            "respond = { text = \"ok\" }\n",
        ),
    )
    .unwrap();
    let mut ws = Workspace::load(dir.path().join("apimock.toml")).unwrap();
    let resp_id = ws
        .snapshot()
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Respond))
        .unwrap()
        .id;

    // Now update respond to an invalid combination (file_path + text).
    let result = ws
        .apply(EditCommand::UpdateRespond {
            id: resp_id,
            respond: crate::view::RespondPayload {
                file_path: Some("nope.json".to_owned()),
                text: Some("also-text".to_owned()),
                ..Default::default()
            },
        })
        .expect("apply should succeed even with bad payload");

    // diagnostics should include at least two errors: exclusivity
    // violation + file-not-found.
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(
        errors.len() >= 2,
        "expected at least 2 errors; got {}: {:?}",
        errors.len(),
        result.diagnostics
    );

    // validate() should agree.
    let report = ws.validate();
    assert!(!report.is_valid);
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
    assert!(save.changed_files.iter().any(|p| p.file_name()
        .map(|n| n.to_string_lossy().contains("apimock-rule-set"))
        .unwrap_or(false)));
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
    assert!(save.diff_summary.iter().any(|d| matches!(
        d.kind,
        crate::view::DiffKind::Updated
    )));

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
    let file_entry = tree.entries.iter().find(|e| e.name == "users.json").unwrap();
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
        d.summary.contains("rule #1")
            && matches!(d.kind, crate::view::DiffKind::Updated)
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
// 5.5.0 — Headers / Body round-trip through Workspace
// -----------------------------------------------------------------

/// Build a workspace whose rule set carries one header and one body
/// match condition, plus a fallback dir so validation passes.
fn make_workspace_with_headers_and_body() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();

    let rs_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/api/protected\"\n",
        "when.request.headers.x-api-key = { value = \"shh\" }\n",
        "when.request.body.json.\"$.action\" = { op = \"equal\", value = \"go\" }\n",
        "respond = { text = \"ok\" }\n",
    );
    let rs_path = dir.path().join("apimock-rule-set.toml");
    std::fs::write(&rs_path, rs_toml).unwrap();

    let root_toml = format!(
        "[listener]\n\
         ip_address = \"127.0.0.1\"\n\
         port = 3001\n\
         \n\
         [service]\n\
         rule_sets = [\"{}\"]\n\
         fallback_respond_dir = \"{}\"\n",
        rs_path.file_name().unwrap().to_string_lossy(),
        fallback.file_name().unwrap().to_string_lossy(),
    );
    let root_path = dir.path().join("apimock.toml");
    std::fs::write(&root_path, root_toml).unwrap();
    (dir, root_path)
}

#[test]
fn save_preserves_headers_through_disk_round_trip() {
    // Load → no edits → save → reload. Headers must survive.
    let (_dir, root) = make_workspace_with_headers_and_body();
    let mut ws = Workspace::load(root.clone()).expect("load");

    // Force a save by editing the URL path of the rule. (No-edit
    // saves are no-ops because baseline == rendered, so the file
    // wouldn't actually be written and we couldn't observe the
    // round-trip.)
    let snap = ws.snapshot();
    let rule_id = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Rule))
        .unwrap()
        .id;
    ws.apply(EditCommand::UpdateRule {
        id: rule_id,
        rule: crate::view::RulePayload {
            url_path: Some("/api/protected/v2".to_owned()),
            method: None,
            respond: crate::view::RespondPayload {
                text: Some("ok".to_owned()),
                ..Default::default()
            },
        },
    })
    .expect("apply UpdateRule");
    let result = ws.save().expect("save");
    assert!(!result.changed_files.is_empty());

    // Re-load and verify the header condition survived the round trip.
    let ws2 = Workspace::load(root).expect("reload");
    let rule = &ws2.config().service.rule_sets[0].rules[0];
    let h = rule
        .when
        .request
        .headers
        .as_ref()
        .expect("headers preserved through Workspace save → reload");
    assert!(h.0.contains_key("x-api-key"));
    assert_eq!(h.0["x-api-key"].value, "shh");
}

#[test]
fn save_preserves_body_through_disk_round_trip() {
    let (_dir, root) = make_workspace_with_headers_and_body();
    let mut ws = Workspace::load(root.clone()).expect("load");

    // Same trick: force a save via UpdateRule, verify body survives.
    let snap = ws.snapshot();
    let rule_id = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Rule))
        .unwrap()
        .id;
    ws.apply(EditCommand::UpdateRule {
        id: rule_id,
        rule: crate::view::RulePayload {
            url_path: Some("/api/protected".to_owned()),
            method: Some("POST".to_owned()),
            respond: crate::view::RespondPayload {
                text: Some("ok".to_owned()),
                ..Default::default()
            },
        },
    })
    .expect("apply UpdateRule");
    let _ = ws.save().expect("save");

    let ws2 = Workspace::load(root).expect("reload");
    let rule = &ws2.config().service.rule_sets[0].rules[0];
    let b = rule
        .when
        .request
        .body
        .as_ref()
        .expect("body preserved through Workspace save → reload");
    let json_kind = apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind::Json;
    let inner = b.0.get(&json_kind).expect("json body kind present");
    assert!(inner.contains_key("$.action"));
    assert_eq!(inner["$.action"].value, "go");
}

#[test]
fn update_rule_does_not_strip_unspecified_headers_or_body() {
    // The semantics-change test: even though RulePayload doesn't
    // carry headers / body fields, UpdateRule must preserve them on
    // the in-memory rule (not just at save time).
    let (_dir, root) = make_workspace_with_headers_and_body();
    let mut ws = Workspace::load(root).expect("load");

    let snap = ws.snapshot();
    let rule_id = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Rule))
        .unwrap()
        .id;

    ws.apply(EditCommand::UpdateRule {
        id: rule_id,
        rule: crate::view::RulePayload {
            url_path: Some("/api/different".to_owned()),
            method: None,
            respond: crate::view::RespondPayload {
                text: Some("changed".to_owned()),
                ..Default::default()
            },
        },
    })
    .expect("apply");

    // In-memory check: headers and body must STILL be present.
    let rule = &ws.config().service.rule_sets[0].rules[0];
    assert!(
        rule.when.request.headers.is_some(),
        "UpdateRule should preserve headers even though RulePayload doesn't carry them"
    );
    assert!(
        rule.when.request.body.is_some(),
        "UpdateRule should preserve body even though RulePayload doesn't carry them"
    );
}
