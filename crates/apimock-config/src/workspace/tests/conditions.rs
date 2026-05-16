//! RFC 016 — per-condition NodeId addressability.

use super::common::make_workspace_with_headers_and_body;
use crate::{
    view::{ConfigFileKind, EditCommand, NodeKind},
    workspace::Workspace,
};

#[test]
fn add_header_condition_creates_condition() {
    let (dir, root) = make_workspace_with_headers_and_body();
    let mut ws = Workspace::load(root).expect("load");
    let snap = ws.snapshot();
    let rule_id = snap
        .files.iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet)).unwrap()
        .nodes.iter()
        .find(|n| matches!(n.kind, NodeKind::Rule)).unwrap()
        .id;

    let result = ws.apply(EditCommand::AddHeaderCondition {
        rule_id,
        condition: crate::view::HeaderConditionPayload {
            name: "x-new-header".to_owned(),
            op: crate::view::HeaderOp::Equal,
            value: Some("new-value".to_owned()),
        },
    }).expect("add ok");

    assert!(!result.changed_nodes.is_empty(), "should return changed node ids");

    // Verify the condition was actually added.
    let rule = &ws.config().service.rule_sets[0].rules[0];
    assert!(rule.when.request.headers.is_some());
    let headers = rule.when.request.headers.as_ref().unwrap();
    assert!(headers.0.contains_key("x-new-header"), "header should be present");
    let _ = dir;
}

#[test]
fn remove_header_condition_deletes_it() {
    let (_dir, root) = make_workspace_with_headers_and_body();
    let mut ws = Workspace::load(root).expect("load");
    let snap = ws.snapshot();
    let rule_id = snap
        .files.iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet)).unwrap()
        .nodes.iter()
        .find(|n| matches!(n.kind, NodeKind::Rule)).unwrap()
        .id;

    // Add a header condition first so we have one to remove.
    let add_result = ws.apply(EditCommand::AddHeaderCondition {
        rule_id,
        condition: crate::view::HeaderConditionPayload {
            name: "x-remove-me".to_owned(),
            op: crate::view::HeaderOp::Equal,
            value: Some("gone".to_owned()),
        },
    }).expect("add");

    let cond_id = add_result.changed_nodes.iter().copied()
        .find(|id| *id != rule_id)
        .expect("cond id");

    // Now remove it.
    ws.apply(EditCommand::RemoveHeaderCondition { id: cond_id }).expect("remove");

    let rule = &ws.config().service.rule_sets[0].rules[0];
    let absent = rule.when.request.headers
        .as_ref()
        .map(|h| !h.0.contains_key("x-remove-me"))
        .unwrap_or(true);
    assert!(absent, "header should be removed");
}

#[test]
fn add_body_condition_creates_condition() {
    let (_dir, root) = make_workspace_with_headers_and_body();
    let mut ws = Workspace::load(root).expect("load");
    let snap = ws.snapshot();
    let rule_id = snap
        .files.iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet)).unwrap()
        .nodes.iter()
        .find(|n| matches!(n.kind, NodeKind::Rule)).unwrap()
        .id;

    ws.apply(EditCommand::AddBodyCondition {
        rule_id,
        condition: crate::view::BodyConditionPayload {
            kind: crate::view::BodyConditionKind::Json,
            path: "user.role".to_owned(),
            op: crate::view::BodyOp::Equal,
            value: serde_json::Value::String("admin".to_owned()),
        },
    }).expect("add body condition");

    let rule = &ws.config().service.rule_sets[0].rules[0];
    assert!(rule.when.request.body.is_some(), "body conditions should exist");
}

#[test]
fn remove_body_condition_deletes_it() {
    let (_dir, root) = make_workspace_with_headers_and_body();
    let mut ws = Workspace::load(root).expect("load");
    let snap = ws.snapshot();
    let rule_id = snap
        .files.iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet)).unwrap()
        .nodes.iter()
        .find(|n| matches!(n.kind, NodeKind::Rule)).unwrap()
        .id;

    let add_result = ws.apply(EditCommand::AddBodyCondition {
        rule_id,
        condition: crate::view::BodyConditionPayload {
            kind: crate::view::BodyConditionKind::Json,
            path: "temp.field".to_owned(),
            op: crate::view::BodyOp::Equal,
            value: serde_json::Value::String("x".to_owned()),
        },
    }).expect("add");

    let cond_id = add_result.changed_nodes.iter().copied()
        .find(|id| *id != rule_id)
        .expect("cond id");

    ws.apply(EditCommand::RemoveBodyCondition { id: cond_id }).expect("remove");
}
