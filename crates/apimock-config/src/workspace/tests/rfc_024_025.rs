//! Tests for RFC 024 (external-change detection) and RFC 025
//! (per-rule-set strategy override).

use std::io::Write;

use crate::Workspace;

use super::common::make_workspace;

// ── RFC 024: external-change detection ───────────────────────────────

#[test]
fn has_external_changes_false_immediately_after_load() {
    let (_dir, root) = make_workspace();
    let ws = Workspace::load(root).unwrap();
    assert!(
        !ws.has_external_changes(),
        "no changes expected right after load"
    );
}

#[test]
fn has_external_changes_true_after_file_modified() {
    let (dir, root) = make_workspace();
    let ws = Workspace::load(root).unwrap();

    // Modify the rule-set file on disk.
    let rs_path = dir.path().join("apimock-rule-set.toml");
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&rs_path)
        .unwrap();
    writeln!(f, "# extra comment").unwrap();
    drop(f); // ensure write is flushed

    // Allow mtime resolution (some filesystems have 1-second granularity).
    // We force a detectable size change via the appended line, so even
    // sub-second-resolution FS should see the change.
    assert!(
        ws.has_external_changes(),
        "should detect the appended content"
    );
}

#[test]
fn sync_from_disk_reloads_updated_content() {
    let (dir, root) = make_workspace();
    let mut ws = Workspace::load(root).unwrap();

    // Append a new rule to the rule-set file.
    let rs_path = dir.path().join("apimock-rule-set.toml");
    let extra = concat!(
        "\n[[rules]]\n",
        "when.request.url_path = \"/new-route\"\n",
        "respond = { text = \"new\" }\n",
    );
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&rs_path)
        .unwrap();
    f.write_all(extra.as_bytes()).unwrap();
    drop(f);

    ws.sync_from_disk().unwrap();

    let snap = ws.snapshot();
    let rule_count: usize = snap.routes.rule_sets.iter().map(|rs| rs.rules.len()).sum();
    assert_eq!(
        rule_count, 3,
        "synced workspace should see 3 rules (2 original + 1 new)"
    );
}

#[test]
fn has_external_changes_false_after_sync() {
    let (dir, root) = make_workspace();
    let mut ws = Workspace::load(root).unwrap();

    let rs_path = dir.path().join("apimock-rule-set.toml");
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&rs_path)
        .unwrap();
    writeln!(f, "# change").unwrap();
    drop(f);

    assert!(ws.has_external_changes());
    ws.sync_from_disk().unwrap();
    assert!(
        !ws.has_external_changes(),
        "after sync, no external changes"
    );
}

// ── RFC 025: per-rule-set strategy ───────────────────────────────────

#[test]
fn update_rule_set_strategy_applies() {
    use crate::view::{EditCommand, NodeKind};

    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).unwrap();

    let snap = ws.snapshot();
    let rs_id = snap
        .files
        .iter()
        .flat_map(|f| f.nodes.iter())
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .map(|n| n.id)
        .expect("rule set node");

    // Initially no per-rule-set strategy.
    let rule_sets = &ws.snapshot().routes.rule_sets;
    assert!(rule_sets[0].strategy.is_none());

    ws.apply(EditCommand::UpdateRuleSetStrategy {
        id: rs_id,
        strategy: Some("round_robin".to_owned()),
    })
    .unwrap();

    let rule_sets = &ws.snapshot().routes.rule_sets;
    assert_eq!(rule_sets[0].strategy.as_deref(), Some("round_robin"));
}

#[test]
fn update_rule_set_strategy_clear_with_none() {
    use crate::view::{EditCommand, NodeKind};

    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).unwrap();

    let snap = ws.snapshot();
    let rs_id = snap
        .files
        .iter()
        .flat_map(|f| f.nodes.iter())
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .map(|n| n.id)
        .unwrap();

    // Set, then clear.
    ws.apply(EditCommand::UpdateRuleSetStrategy {
        id: rs_id,
        strategy: Some("uniform_random".to_owned()),
    })
    .unwrap();
    ws.apply(EditCommand::UpdateRuleSetStrategy {
        id: rs_id,
        strategy: None,
    })
    .unwrap();

    let rule_sets = &ws.snapshot().routes.rule_sets;
    assert!(
        rule_sets[0].strategy.is_none(),
        "strategy should be cleared to None"
    );
}

#[test]
fn update_rule_set_strategy_unknown_name_errors() {
    use crate::view::{EditCommand, NodeKind};

    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).unwrap();

    let snap = ws.snapshot();
    let rs_id = snap
        .files
        .iter()
        .flat_map(|f| f.nodes.iter())
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .map(|n| n.id)
        .unwrap();

    let result = ws.apply(EditCommand::UpdateRuleSetStrategy {
        id: rs_id,
        strategy: Some("nonexistent_strategy".to_owned()),
    });
    assert!(result.is_err(), "unknown strategy name should return Err");
}
