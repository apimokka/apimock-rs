//! Core Workspace behaviour: load, snapshot, apply, validate.

use super::common::make_workspace;
use crate::{
    error::ApplyError,
    view::{ConfigFileKind, ConfigNodeView, EditCommand, EditValue, NodeId, NodeKind, Severity},
    workspace::Workspace,
};

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
    assert!(
        root_view
            .nodes
            .iter()
            .any(|n| n.display_name == "apimock.toml")
    );

    let rs_view = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .expect("rule set present");
    // rule-set + 2 rules + 2 responds
    assert!(rs_view.nodes.len() >= 5);
    assert!(
        rs_view
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::RuleSet))
    );
    assert!(
        rs_view
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Rule))
    );
    assert!(
        rs_view
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Respond))
    );
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
                ..Default::default()
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
