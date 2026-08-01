//! Tests for RFC 027 (rule priority), RFC 029 (per-condition diff).

use crate::Workspace;
use crate::view::{EditCommand, NodeKind, RulePayload, RespondPayload, UrlPathOp};

use super::common::make_workspace;

// ── RFC 027: priority field round-trip ────────────────────────────────

#[test]
fn priority_in_rule_payload_applied_and_visible_in_view() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).unwrap();

    let snap = ws.snapshot();
    let rs_id = snap.files.iter()
        .flat_map(|f| f.nodes.iter())
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .map(|n| n.id).unwrap();

    // Add a rule with priority = 5.
    ws.apply(EditCommand::AddRule {
        parent: rs_id,
        rule: RulePayload {
            url_path: Some("/high-priority".into()),
            url_path_op: Some(UrlPathOp::Equal),
            method: None,
            priority: Some(5),
            headers: None,
            body: None,
            respond: RespondPayload { text: Some("priority".into()), ..Default::default() },
        },
    }).unwrap();

    let snap = ws.snapshot();
    let rules = &snap.routes.rule_sets[0].rules;
    let added = rules.iter().find(|r| r.priority == Some(5));
    assert!(added.is_some(), "priority should be set to 5");
}

#[test]
fn priority_round_trips_through_save_load() {
    let (dir, root) = make_workspace();
    let mut ws = Workspace::load(root.clone()).unwrap();

    let snap = ws.snapshot();
    let rs_id = snap.files.iter()
        .flat_map(|f| f.nodes.iter())
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .map(|n| n.id).unwrap();

    ws.apply(EditCommand::AddRule {
        parent: rs_id,
        rule: RulePayload {
            url_path: Some("/prio-check".into()),
            url_path_op: None,
            method: None,
            priority: Some(10),
            headers: None,
            body: None,
            respond: RespondPayload { text: Some("ok".into()), ..Default::default() },
        },
    }).unwrap();
    ws.save().unwrap();

    let ws2 = Workspace::load(root).unwrap();
    let snap2 = ws2.snapshot();
    let rules = &snap2.routes.rule_sets[0].rules;
    let saved = rules.iter().find(|r| r.priority == Some(10));
    assert!(saved.is_some(), "priority should survive save/load round-trip");

    // Clean up tempdir
    drop(dir);
}

// ── RFC 029: per-condition diff ───────────────────────────────────────

#[test]
fn diff_includes_header_condition_added() {
    use crate::view::HeaderConditionPayload;

    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).unwrap();

    // Get the first rule's NodeId.
    let snap = ws.snapshot();
    let rule_id = snap.files.iter()
        .flat_map(|f| f.nodes.iter())
        .find(|n| matches!(n.kind, NodeKind::Rule))
        .map(|n| n.id).unwrap();

    ws.apply(EditCommand::AddHeaderCondition {
        rule_id,
        condition: HeaderConditionPayload {
            name: "x-tenant".into(),
            op: crate::view::HeaderOp::Equal,
            value: Some("acme".into()),
        },
    }).unwrap();

    let result = ws.save().unwrap();
    let has_header_diff = result.diff_summary.iter().any(|d| matches!(
        d.kind, crate::view::DiffKind::HeaderConditionAdded
    ));
    assert!(has_header_diff, "save diff should include HeaderConditionAdded item");
}

#[test]
fn diff_includes_body_condition_added() {
    use crate::view::BodyConditionPayload;

    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).unwrap();

    let snap = ws.snapshot();
    let rule_id = snap.files.iter()
        .flat_map(|f| f.nodes.iter())
        .find(|n| matches!(n.kind, NodeKind::Rule))
        .map(|n| n.id).unwrap();

    ws.apply(EditCommand::AddBodyCondition {
        rule_id,
        condition: BodyConditionPayload {
            kind: crate::view::BodyConditionKind::Json,
            path: "action".into(),
            op: crate::view::BodyOp::Equal,
            value: serde_json::json!("create"),
        },
    }).unwrap();

    let result = ws.save().unwrap();
    let has_body_diff = result.diff_summary.iter().any(|d| matches!(
        d.kind, crate::view::DiffKind::BodyConditionAdded
    ));
    assert!(has_body_diff, "save diff should include BodyConditionAdded item");
}
