//! Per-condition rule evaluation, shared by `match-test` and `get --why`
//! (RFC 055).
//!
//! # Why this exists as data, not `println!`
//!
//! `match-test` (RFC 015) checks a rule's conditions and prints a
//! tick/cross line per condition directly — there was only ever one
//! consumer, so there was nothing to share. `get --why` needs the same
//! per-condition detail in **two** shapes: human text and RFC 053's
//! JSON envelope (name, expectation, actual value, whether it held).
//! Printing directly can't serve both, so the check itself now returns
//! structured [`ConditionCheck`]s; each caller renders them how it needs.
//!
//! `match-test`'s own printed output is unchanged — verified by its
//! existing tests and the worked example in
//! `crates/apimock/examples/validate-in-ci/`, which asserts on it
//! character-for-character. That's why every [`ConditionCheck`] carries
//! a `legacy_text` field: the exact string `match-test` always printed
//! after the tick, computed once here instead of reconstructed
//! per-caller from the structured fields (which don't preserve the
//! original formatting exactly — e.g. `url_path` and `header` checks
//! never printed an "actual" value; `get`'s structured output does).

use apimock_routing::ParsedRequest;
use apimock_routing::rule_set::rule::Rule;

/// One condition's evaluation against a request: what was expected,
/// what was actually there, and whether that was a match.
pub(super) struct ConditionCheck {
    /// The condition's kind, and its key where it has one —
    /// `"url_path"`, `"method"`, `"header:x-api-key"`, `"body.json:customer.tier"`.
    pub name: String,
    /// Human-readable operator and expected value, e.g. `equal "/orders"`.
    pub expectation: String,
    /// Human-readable actual value, or `(absent)` — always computed,
    /// even for the two condition kinds whose legacy text omits it.
    pub actual: String,
    pub matched: bool,
    /// Exactly what `match-test` has always printed after the tick for
    /// this condition. See the module doc comment for why this is its
    /// own field rather than derived from the three above.
    pub legacy_text: String,
}

/// Evaluate every condition `rule` declares against `parsed`, in the
/// same order `match-test` has always checked and printed them:
/// `url_path`, `method`, headers, body. A rule with no conditions of a
/// given kind contributes nothing for that kind — matching the original
/// `check_*` functions' early returns.
pub(super) fn evaluate_rule(rule: &Rule, parsed: &ParsedRequest) -> Vec<ConditionCheck> {
    let mut out = Vec::new();
    check_url_path(&rule.when.request, parsed, &mut out);
    check_method(&rule.when.request, parsed, &mut out);
    check_headers(&rule.when.request, parsed, &mut out);
    check_body(&rule.when.request, parsed, &mut out);
    out
}

fn check_url_path(
    req: &apimock_routing::rule_set::rule::when::request::Request,
    parsed: &ParsedRequest,
    out: &mut Vec<ConditionCheck>,
) {
    use apimock_routing::rule_set::rule::when::request::url_path::UrlPathConfig;
    match req.url_path_config.as_ref() {
        None => {}
        Some(UrlPathConfig::Simple(p)) => {
            let matched = parsed.url_path == *p;
            out.push(ConditionCheck {
                name: "url_path".to_owned(),
                expectation: format!("equal {:?}", p),
                actual: parsed.url_path.clone(),
                matched,
                legacy_text: format!("url_path equal {:?}", p),
            });
        }
        Some(UrlPathConfig::Detailed(u)) => {
            let op = u.op.clone().unwrap_or_default();
            let matched = op.is_match(&parsed.url_path, &u.value);
            out.push(ConditionCheck {
                name: "url_path".to_owned(),
                expectation: format!("{} {:?}", op, u.value),
                actual: parsed.url_path.clone(),
                matched,
                legacy_text: format!("url_path {} {:?}", op, u.value),
            });
        }
    }
}

fn check_method(
    req: &apimock_routing::rule_set::rule::when::request::Request,
    parsed: &ParsedRequest,
    out: &mut Vec<ConditionCheck>,
) {
    use apimock_routing::rule_set::rule::when::request::http_method::HttpMethod;
    let expected = match req.http_method.as_ref() {
        None => return,
        Some(HttpMethod::Get) => "GET",
        Some(HttpMethod::Post) => "POST",
        Some(HttpMethod::Put) => "PUT",
        Some(HttpMethod::Delete) => "DELETE",
    };
    let actual = parsed.component_parts.method.as_str();
    let matched = actual.eq_ignore_ascii_case(expected);
    out.push(ConditionCheck {
        name: "method".to_owned(),
        expectation: expected.to_owned(),
        actual: actual.to_owned(),
        matched,
        legacy_text: format!("method {} (actual: {})", expected, actual),
    });
}

fn check_headers(
    req: &apimock_routing::rule_set::rule::when::request::Request,
    parsed: &ParsedRequest,
    out: &mut Vec<ConditionCheck>,
) {
    let Some(headers) = req.headers.as_ref() else {
        return;
    };
    for (name, stmt) in &headers.0 {
        use apimock_routing::rule_set::rule::when::request::headers::header_operator::HeaderOperator;
        let op = stmt.op.clone().unwrap_or_default();
        let matched = match &op {
            HeaderOperator::Exists => parsed.component_parts.headers.contains_key(name.as_str()),
            HeaderOperator::Absent => !parsed.component_parts.headers.contains_key(name.as_str()),
            _ => match parsed.component_parts.headers.get(name.as_str()) {
                None => false,
                Some(hv) => {
                    let v = hv.to_str().unwrap_or("");
                    op.to_rule_op().is_match(v, &stmt.value)
                }
            },
        };
        let actual = parsed
            .component_parts
            .headers
            .get(name.as_str())
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_owned())
            .unwrap_or_else(|| "(absent)".to_owned());
        out.push(ConditionCheck {
            name: format!("header:{}", name),
            expectation: format!("{} {:?}", op, stmt.value),
            actual,
            matched,
            legacy_text: format!("header {:?} {} {:?}", name, op, stmt.value),
        });
    }
}

fn check_body(
    req: &apimock_routing::rule_set::rule::when::request::Request,
    parsed: &ParsedRequest,
    out: &mut Vec<ConditionCheck>,
) {
    use apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind;
    use apimock_routing::rule_set::rule::when::request::body::body_operator::BodyOperator;
    use apimock_routing::util::json::json_value_by_jsonpath;

    let Some(body) = req.body.as_ref() else {
        return;
    };
    let Some(body_json) = parsed.body_json.as_ref() else {
        out.push(ConditionCheck {
            name: "body".to_owned(),
            expectation: "a JSON body".to_owned(),
            actual: "(no JSON body)".to_owned(),
            matched: false,
            legacy_text: "body (request has no JSON body)".to_owned(),
        });
        return;
    };
    let Some(conditions) = body.0.get(&BodyKind::Json) else {
        return;
    };
    for (path, stmt) in conditions {
        let op = stmt.op.clone().unwrap_or_default();
        let resolved = json_value_by_jsonpath(body_json, path);
        let matched = match resolved {
            None => matches!(op, BodyOperator::Absent),
            Some(v) => op.is_match(v, &stmt.value),
        };
        let actual = resolved
            .map(|v| v.to_string())
            .unwrap_or_else(|| "(absent)".to_owned());
        out.push(ConditionCheck {
            name: format!("body.json:{}", path),
            expectation: format!("{} {:?}", op, stmt.value),
            actual: actual.clone(),
            matched,
            legacy_text: format!(
                "body.json {:?} {} {:?}  (actual: {})",
                path, op, stmt.value, actual
            ),
        });
    }
}
