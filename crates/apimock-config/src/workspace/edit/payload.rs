//! Payload → model converters used by the apply layer.
//!
//! # Why GUI payloads aren't the same as routing-crate types
//!
//! The view layer's `EditValue` / `RulePayload` / `RespondPayload`
//! shapes are *GUI-friendly* — flat structs, no derived caches, no
//! validation pre-baked. The routing crate's `Rule` / `Respond` carry
//! cached fields (`status_code`, normalised `url_path`, etc.) and live
//! behind validation. This module is the thin layer that builds a
//! validated `Rule` / `Respond` from a payload and feeds it back to
//! the apply code.

use crate::error::{ApplyError, ConfigError};
use crate::view::{BodyOp, EditValue, HeaderOp, UrlPathOp};
use std::collections::HashMap;

/// Build a `Rule` from a GUI-shaped payload.
///
/// # `existing` — preserving unspecified fields
///
/// `RulePayload.headers` / `body` use `Option<Vec<_>>` semantics:
/// - `None` → preserve from existing rule (or keep empty for AddRule).
/// - `Some([])` → clear.
/// - `Some([…])` → replace.
///
/// `url_path_op` is `None` → defaults to `Equal`.
pub(super) fn build_rule_from_payload(
    payload: crate::view::RulePayload,
    rule_set: &apimock_routing::RuleSet,
    rs_idx: usize,
    existing: Option<&apimock_routing::Rule>,
) -> Result<apimock_routing::Rule, ApplyError> {
    use apimock_routing::rule_set::rule::Rule;
    use apimock_routing::rule_set::rule::when::When;
    use apimock_routing::rule_set::rule::when::request::{
        Request, http_method::HttpMethod, url_path::UrlPathConfig,
    };

    // ── URL path (RFC 001) ────────────────────────────────────────────
    let url_path_config = payload.url_path.as_ref().map(|s| {
        match payload.url_path_op {
            None | Some(UrlPathOp::Equal) => UrlPathConfig::Simple(s.clone()),
            Some(op) => {
                use apimock_routing::rule_set::rule::when::request::url_path::UrlPath as RoutingUrlPath;
                UrlPathConfig::Detailed(RoutingUrlPath {
                    value: s.clone(),
                    value_with_prefix: String::new(), // filled by compute_derived_fields
                    op: Some(url_path_op_to_routing(op)),
                })
            }
        }
    });

    let http_method = match payload.method.as_deref() {
        Some("GET") | Some("get") => Some(HttpMethod::Get),
        Some("POST") | Some("post") => Some(HttpMethod::Post),
        Some("PUT") | Some("put") => Some(HttpMethod::Put),
        Some("DELETE") | Some("delete") => Some(HttpMethod::Delete),
        Some(other) => {
            return Err(ApplyError::InvalidPayload {
                reason: format!(
                    "unsupported HTTP method `{}` — supported: GET, POST, PUT, DELETE",
                    other
                ),
            });
        }
        None => None,
    };

    // ── Headers (RFC 002) ─────────────────────────────────────────────
    // None → preserve; Some([]) → clear; Some([…]) → replace.
    let headers = match payload.headers {
        None => existing.and_then(|prev| prev.when.request.headers.clone()),
        Some(ref list) if list.is_empty() => None,
        Some(list) => Some(build_headers(&list)?),
    };

    // ── Body (RFC 002) ────────────────────────────────────────────────
    let body = match payload.body {
        None => existing.and_then(|prev| prev.when.request.body.clone()),
        Some(ref list) if list.is_empty() => None,
        Some(list) => Some(build_body(&list)?),
    };

    let request = Request {
        url_path_config,
        url_path: None, // derived by compute_derived_fields
        http_method,
        headers,
        body,
    };

    let rule = Rule {
        when: When { request },
        respond: build_respond_from_payload(payload.respond),
        weight: None,
        priority: None,
    };

    Ok(rule.compute_derived_fields(rule_set, rule_set.rules.len(), rs_idx))
}

// ── RFC 001 helper ────────────────────────────────────────────────────

fn url_path_op_to_routing(op: UrlPathOp) -> apimock_routing::rule_set::rule::when::request::rule_op::RuleOp {
    use apimock_routing::rule_set::rule::when::request::rule_op::RuleOp;
    match op {
        UrlPathOp::Equal => RuleOp::Equal,
        UrlPathOp::StartsWith => RuleOp::StartsWith,
        UrlPathOp::Contains => RuleOp::Contains,
        UrlPathOp::EndsWith => RuleOp::Contains, // EndsWith not in RuleOp; use Contains as best-effort
        UrlPathOp::WildCard => RuleOp::WildCard,
        UrlPathOp::NotEqual => RuleOp::NotEqual,
    }
}

// ── RFC 002 helpers ───────────────────────────────────────────────────

fn build_headers(
    input: &[crate::view::HeaderConditionPayload],
) -> Result<apimock_routing::rule_set::rule::when::request::headers::Headers, ApplyError> {
    use apimock_routing::rule_set::rule::when::condition_statement::ConditionStatement;

    let mut map: HashMap<String, ConditionStatement> = HashMap::new();
    for cond in input {
        let op = header_op_to_routing(cond.op);
        let value = cond.value.clone().unwrap_or_default();
        map.insert(cond.name.to_lowercase(), ConditionStatement { op: Some(op), value });
    }
    Ok(apimock_routing::rule_set::rule::when::request::headers::Headers(map))
}

fn header_op_to_routing(op: HeaderOp) -> apimock_routing::rule_set::rule::when::request::rule_op::RuleOp {
    use apimock_routing::rule_set::rule::when::request::rule_op::RuleOp;
    match op {
        HeaderOp::Equal => RuleOp::Equal,
        HeaderOp::Contains => RuleOp::Contains,
        HeaderOp::StartsWith => RuleOp::StartsWith,
        HeaderOp::EndsWith => RuleOp::Contains, // fallback
        HeaderOp::Regex | HeaderOp::Exists | HeaderOp::Absent => RuleOp::Equal,
        HeaderOp::NotEqual => RuleOp::NotEqual,
        HeaderOp::WildCard => RuleOp::WildCard,
    }
}

fn build_body(
    input: &[crate::view::BodyConditionPayload],
) -> Result<apimock_routing::rule_set::rule::when::request::body::Body, ApplyError> {
    use apimock_routing::rule_set::rule::when::request::body::{
        Body, BodyConditionStatement,
        body_kind::BodyKind,
    };

    let mut json_map: HashMap<String, BodyConditionStatement> = HashMap::new();
    for cond in input {
        let op = body_op_to_routing(cond.op);
        let value = value_to_string(&cond.value);
        json_map.insert(cond.path.clone(), BodyConditionStatement { op: Some(op), value });
    }

    let mut outer = HashMap::new();
    outer.insert(BodyKind::Json, json_map);
    Ok(Body(outer))
}

fn body_op_to_routing(op: BodyOp) -> apimock_routing::rule_set::rule::when::request::body::body_operator::BodyOperator {
    use apimock_routing::rule_set::rule::when::request::body::body_operator::BodyOperator;
    match op {
        BodyOp::Equal => BodyOperator::Equal,
        BodyOp::EqualString => BodyOperator::EqualString,
        BodyOp::Contains => BodyOperator::Contains,
        BodyOp::StartsWith => BodyOperator::StartsWith,
        BodyOp::EndsWith => BodyOperator::EndsWith,
        BodyOp::Regex => BodyOperator::Regex,
        BodyOp::EqualTyped => BodyOperator::EqualTyped,
        BodyOp::EqualNumber => BodyOperator::EqualNumber,
        BodyOp::GreaterThan => BodyOperator::GreaterThan,
        BodyOp::LessThan => BodyOperator::LessThan,
        BodyOp::GreaterOrEqual => BodyOperator::GreaterOrEqual,
        BodyOp::LessOrEqual => BodyOperator::LessOrEqual,
        BodyOp::Exists => BodyOperator::Exists,
        BodyOp::Absent => BodyOperator::Absent,
        BodyOp::ArrayLengthEqual => BodyOperator::ArrayLengthEqual,
        BodyOp::ArrayLengthAtLeast => BodyOperator::ArrayLengthAtLeast,
        BodyOp::ArrayContains => BodyOperator::ArrayContains,
    }
}

/// Convert a `serde_json::Value` to the string form stored in
/// `BodyConditionStatement.value`.
fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub(super) fn build_respond_from_payload(payload: crate::view::RespondPayload) -> apimock_routing::Respond {
    apimock_routing::Respond {
        file_path: payload.file_path,
        csv_records_key: None,
        text: payload.text,
        status: payload.status,
        status_code: None, // derived later
        headers: None,
        delay_response_milliseconds: payload.delay_milliseconds,
    }
}

pub(super) fn value_as_string(value: &EditValue) -> Result<String, ApplyError> {
    match value {
        EditValue::String(s) => Ok(s.clone()),
        EditValue::Enum(s) => Ok(s.clone()),
        other => Err(ApplyError::InvalidPayload {
            reason: format!("expected a string, got {:?}", other),
        }),
    }
}

pub(super) fn value_as_integer(value: &EditValue) -> Result<i64, ApplyError> {
    match value {
        EditValue::Integer(n) => Ok(*n),
        other => Err(ApplyError::InvalidPayload {
            reason: format!("expected an integer, got {:?}", other),
        }),
    }
}

pub(super) fn value_as_bool(value: &EditValue) -> Result<bool, ApplyError> {
    match value {
        EditValue::Boolean(b) => Ok(*b),
        other => Err(ApplyError::InvalidPayload {
            reason: format!("expected a boolean, got {:?}", other),
        }),
    }
}

/// Wrap a ConfigError produced inside an apply command as an
/// `ApplyError::InvalidPayload`.
pub(super) fn internal_path_err(err: ConfigError) -> ApplyError {
    ApplyError::InvalidPayload {
        reason: format!("internal path resolution failed: {}", err),
    }
}
