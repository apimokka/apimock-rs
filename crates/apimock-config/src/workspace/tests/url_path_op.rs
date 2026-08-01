//! RFC 013 — url_path / url_path_op validation.

use super::common::make_workspace;
use crate::{
    view::{ConfigFileKind, EditCommand, NodeKind},
    workspace::Workspace,
};

// ── RFC 013: url_path / url_path_op validation ────────────────────────

#[test]
fn url_path_op_without_path_is_invalid_payload() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");

    let snap = ws.snapshot();
    let rs_id = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .unwrap()
        .id;

    let result = ws.apply(EditCommand::AddRule {
        parent: rs_id,
        rule: crate::view::RulePayload {
            url_path: None,
            url_path_op: Some(crate::view::UrlPathOp::StartsWith),
            respond: crate::view::RespondPayload {
                text: Some("hi".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
    });

    assert!(
        result.is_err(),
        "url_path_op: Some(_) with url_path: None should be an error"
    );
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("url_path_op requires url_path"),
        "error message should mention url_path_op: {}",
        err_msg
    );
}

#[test]
fn url_path_op_without_path_errors_on_update_rule_too() {
    let (_dir, root) = make_workspace();
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

    let result = ws.apply(EditCommand::UpdateRule {
        id: rule_id,
        rule: crate::view::RulePayload {
            url_path: None,
            url_path_op: Some(crate::view::UrlPathOp::Contains),
            respond: crate::view::RespondPayload {
                text: Some("hi".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
    });

    assert!(
        result.is_err(),
        "UpdateRule with op but no path should error"
    );
}

#[test]
fn url_path_and_op_both_none_is_valid() {
    // Both None → no URL constraint → valid (preserve for UpdateRule,
    // absent for AddRule). Must not be rejected.
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");

    let snap = ws.snapshot();
    let rs_id = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .unwrap()
        .id;

    // A rule with method but no URL constraint is valid.
    let result = ws.apply(EditCommand::AddRule {
        parent: rs_id,
        rule: crate::view::RulePayload {
            url_path: None,
            url_path_op: None,
            method: Some("POST".to_owned()),
            respond: crate::view::RespondPayload {
                text: Some("ok".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
    });

    assert!(
        result.is_ok(),
        "url_path: None, url_path_op: None should be accepted"
    );
}

#[test]
fn url_path_some_with_op_is_valid() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");

    let snap = ws.snapshot();
    let rs_id = snap
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::RuleSet))
        .unwrap()
        .id;

    let result = ws.apply(EditCommand::AddRule {
        parent: rs_id,
        rule: crate::view::RulePayload {
            url_path: Some("/api".to_owned()),
            url_path_op: Some(crate::view::UrlPathOp::StartsWith),
            respond: crate::view::RespondPayload {
                text: Some("ok".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
    });

    assert!(
        result.is_ok(),
        "url_path: Some + url_path_op: Some should succeed"
    );
}
