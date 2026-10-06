//! RFC 082 — the `set` path's method payloads. The CLI uppercases
//! `--method` before it reaches here, so these exercise the library
//! surface a GUI or any direct caller of `RulePayload` gets.

use super::common::make_workspace;
use crate::{
    view::{ConfigFileKind, EditCommand, NodeKind, RespondPayload, RulePayload},
    workspace::Workspace,
};

fn first_rule_id(ws: &Workspace) -> crate::view::NodeId {
    ws.snapshot()
        .files
        .iter()
        .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
        .unwrap()
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Rule))
        .unwrap()
        .id
}

fn update_with_method(method: &str) -> Result<(), String> {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");
    let id = first_rule_id(&ws);
    ws.apply(EditCommand::UpdateRule {
        id,
        rule: RulePayload {
            url_path: Some("/api/users".to_owned()),
            method: Some(method.to_owned()),
            respond: RespondPayload {
                text: Some("ok".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
    })
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[test]
fn patch_in_canonical_spelling_is_accepted() {
    assert!(update_with_method("PATCH").is_ok());
}

#[test]
fn patch_in_lowercase_is_accepted_by_the_library_surface() {
    assert!(update_with_method("patch").is_ok());
}

/// The library surface keeps the spelling `payload.rs` has always accepted:
/// exact or all-lowercase, never a mixed case.
#[test]
fn mixed_case_patch_is_refused_by_the_library_surface() {
    let err = update_with_method("Patch").expect_err("Patch should be refused");
    assert!(err.contains("unsupported HTTP method `Patch`"), "{err}");
}

#[test]
fn the_refusal_lists_patch_from_the_shared_source() {
    let err = update_with_method("GTE").expect_err("GTE should be refused");
    assert!(
        err.contains("supported: GET, POST, PUT, DELETE, PATCH"),
        "{err}"
    );
    assert!(
        !err.contains(';'),
        "a typo must carry no reason clause: {err}"
    );
}

#[test]
fn an_excluded_method_is_refused_with_its_reason() {
    let err = update_with_method("OPTIONS").expect_err("OPTIONS should be refused");
    assert!(
        err.contains("supported: GET, POST, PUT, DELETE, PATCH; OPTIONS is answered by the built-in CORS preflight handler"),
        "{err}"
    );
}

/// RFC 082 acceptance: the value round-trips through `toml_writer` on disk —
/// written as `method = "PATCH"`, and loading back as the `Patch` variant.
#[test]
fn patch_round_trips_through_the_writer_on_disk() {
    use apimock_routing::rule_set::rule::when::request::http_method::HttpMethod;

    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root.clone()).expect("load");
    let id = first_rule_id(&ws);
    ws.apply(EditCommand::UpdateRule {
        id,
        rule: RulePayload {
            url_path: Some("/api/users".to_owned()),
            method: Some("PATCH".to_owned()),
            respond: RespondPayload {
                text: Some("ok".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
    })
    .expect("apply PATCH");
    ws.save().expect("save");

    let written = std::fs::read_to_string(root.parent().unwrap().join("apimock-rule-set.toml"))
        .expect("read rule set back");
    assert!(written.contains("method = \"PATCH\""), "{written}");

    let reloaded = Workspace::load(root).expect("reload");
    let rule = &reloaded.config().service.rule_sets[0].rules[0];
    assert!(
        matches!(rule.when.request.http_method, Some(HttpMethod::Patch)),
        "reloaded method was {:?}",
        rule.when.request.http_method
    );
}
