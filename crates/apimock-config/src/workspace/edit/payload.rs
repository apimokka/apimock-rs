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
use crate::view::EditValue;

pub(super) fn build_rule_from_payload(
    payload: crate::view::RulePayload,
    rule_set: &apimock_routing::RuleSet,
    rs_idx: usize,
) -> Result<apimock_routing::Rule, ApplyError> {
    use apimock_routing::rule_set::rule::Rule;
    use apimock_routing::rule_set::rule::when::When;
    use apimock_routing::rule_set::rule::when::request::{
        Request, http_method::HttpMethod, url_path::UrlPathConfig,
    };

    // Build the Request shape from the simple payload. We use the
    // simple UrlPath variant (Simple(String)) because the payload's
    // url_path is a plain string; the richer variants (op, etc.) are
    // out of scope for 5.1 — a GUI can round-trip them once Step-5
    // exposes richer form controls.
    let url_path_config = payload.url_path.as_ref().map(|s| UrlPathConfig::Simple(s.clone()));

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

    let request = Request {
        url_path_config,
        url_path: None, // derived below
        http_method,
        headers: None,
        body: None,
    };

    let rule = Rule {
        when: When { request },
        respond: build_respond_from_payload(payload.respond),
    };

    // compute_derived_fields normalises the URL path with the rule
    // set's prefix and validates the status code. Running it here means
    // the freshly-created rule is ready for matching without a second
    // pass.
    //
    // `rule_idx` at this point is whatever position the rule will
    // occupy after being pushed — use `rule_set.rules.len()` because
    // the push happens immediately after.
    Ok(rule.compute_derived_fields(rule_set, rule_set.rules.len(), rs_idx))
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

/// Wrap a ConfigError produced inside an apply command as an
/// `ApplyError::InvalidPayload`. Apply uses anyhow-ish flattening
/// because the caller doesn't care whether the root cause was a
/// read-fail or a parse-fail — they all surface as "edit couldn't
/// be applied" from the GUI's point of view.
pub(super) fn internal_path_err(err: ConfigError) -> ApplyError {
    ApplyError::InvalidPayload {
        reason: format!("internal path resolution failed: {}", err),
    }
}
