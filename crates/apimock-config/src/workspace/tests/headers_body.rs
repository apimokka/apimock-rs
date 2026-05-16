//! Header and body condition preservation (5.5.0 round-trip guarantee).

use super::common::make_workspace_with_headers_and_body;
use crate::{
    view::{ConfigFileKind, EditCommand, NodeKind},
    workspace::Workspace,
};

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
            ..Default::default()
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
            ..Default::default()
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
    assert!(inner.contains_key("action"));
    assert_eq!(inner["action"].value, "go");
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
            ..Default::default()
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
