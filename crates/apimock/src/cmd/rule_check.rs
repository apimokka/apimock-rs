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
        // RFC 072: must agree with `Headers::is_match` — `Exists`/`Absent`
        // check presence only (a present-but-undecodable header still
        // exists), and a value operator against an undecodable header
        // does not match, regardless of which operator it is. See the
        // agreement test in this crate's tests for why this can't drift
        // from the server path silently again.
        let matched = match &op {
            HeaderOperator::Exists => parsed.component_parts.headers.contains_key(name.as_str()),
            HeaderOperator::Absent => !parsed.component_parts.headers.contains_key(name.as_str()),
            _ => match parsed.component_parts.headers.get(name.as_str()) {
                None => false,
                Some(hv) => match hv.to_str() {
                    Ok(v) => op.to_rule_op().is_match(v, &stmt.value),
                    Err(_) => false,
                },
            },
        };
        let actual = match parsed.component_parts.headers.get(name.as_str()) {
            None => "(absent)".to_owned(),
            Some(hv) => match hv.to_str() {
                Ok(v) => v.to_owned(),
                Err(_) => "(not valid UTF-8)".to_owned(),
            },
        };
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

// ── RFC 072: server/match-test agreement ─────────────────────────────
//
// `apimock_routing::Headers::is_match` (the server path) and
// `check_headers` above (the `match-test`/`get --why` path) are
// **independent implementations of the same rule** — the same shape as
// `respond_validator_agreement.rs` in `apimock-config`, which exists
// because two validators diverged exactly like this once. This is the
// deliverable RFC 072 asks for, not the one-line `return false` flip
// alone: nothing before this test checked that the two paths answer a
// non-UTF-8 header value the same way, and they didn't
// (`headers.rs` returned `true`, `rule_check.rs` silently treated the
// unreadable value as `""` and delegated to the operator anyway).
#[cfg(test)]
mod tests {
    use apimock_routing::RuleSet;

    /// One header condition, run through both paths against one request
    /// header state, asserting they reach the same verdict.
    struct Case {
        name: &'static str,
        /// TOML `op` value, e.g. `"equal"`, `"absent"`. Omitted (`None`)
        /// exercises the default (`equal`).
        op: Option<&'static str>,
        /// TOML `value` for the condition (ignored by presence operators
        /// but always written, since `Headers` requires the field).
        condition_value: &'static str,
        header: HeaderState,
        expect_match: bool,
    }

    enum HeaderState {
        Absent,
        Utf8(&'static str),
        /// Present, but not valid UTF-8 — the case RFC 072 is about.
        NonUtf8(&'static [u8]),
    }

    fn corpus() -> Vec<Case> {
        use HeaderState::*;
        vec![
            // ── The baseline matrix for one operator (equal) ─────────
            Case {
                name: "equal: valid UTF-8, matching value",
                op: Some("equal"),
                condition_value: "expected",
                header: Utf8("expected"),
                expect_match: true,
            },
            Case {
                name: "equal: valid UTF-8, non-matching value",
                op: Some("equal"),
                condition_value: "expected",
                header: Utf8("wrong"),
                expect_match: false,
            },
            Case {
                name: "equal: empty condition value against an empty header value",
                op: Some("equal"),
                condition_value: "",
                header: Utf8(""),
                expect_match: true,
            },
            Case {
                name: "equal: header genuinely absent",
                op: Some("equal"),
                condition_value: "expected",
                header: Absent,
                expect_match: false,
            },
            // ── The defect this RFC fixes: non-UTF-8, every value
            //    operator, all must fail closed ──────────────────────
            Case {
                name: "equal: non-UTF-8 header value does not satisfy the condition",
                op: Some("equal"),
                condition_value: "expected",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "not_equal: non-UTF-8 must still not satisfy — the case a naive \
                       empty-string stand-in gets backwards",
                op: Some("not_equal"),
                condition_value: "expected",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "starts_with: non-UTF-8 does not satisfy",
                op: Some("starts_with"),
                condition_value: "exp",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "not_starts_with: non-UTF-8 does not satisfy",
                op: Some("not_starts_with"),
                condition_value: "exp",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "ends_with: non-UTF-8 does not satisfy",
                op: Some("ends_with"),
                condition_value: "ted",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "not_ends_with: non-UTF-8 does not satisfy",
                op: Some("not_ends_with"),
                condition_value: "ted",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "contains: non-UTF-8 does not satisfy",
                op: Some("contains"),
                condition_value: "pec",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "not_contains: non-UTF-8 does not satisfy",
                op: Some("not_contains"),
                condition_value: "pec",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "wild_card: non-UTF-8 does not satisfy",
                op: Some("wild_card"),
                condition_value: "exp*",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "regex: non-UTF-8 does not satisfy",
                op: Some("regex"),
                condition_value: "^exp",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "not_regex: non-UTF-8 does not satisfy",
                op: Some("not_regex"),
                condition_value: "^exp",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            // ── Presence operators: "cannot read" != "not present" ──
            Case {
                name: "exists: non-UTF-8 header still satisfies exists — it IS present",
                op: Some("exists"),
                condition_value: "",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: true,
            },
            Case {
                name: "exists: genuinely absent header does not satisfy exists",
                op: Some("exists"),
                condition_value: "",
                header: Absent,
                expect_match: false,
            },
            Case {
                name: "absent: non-UTF-8 header does NOT satisfy absent — the deliberate \
                       decision (present but unreadable is not the same as not present)",
                op: Some("absent"),
                condition_value: "",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
            Case {
                name: "absent: genuinely absent header satisfies absent",
                op: Some("absent"),
                condition_value: "",
                header: Absent,
                expect_match: true,
            },
            // ── Default operator (omitted → equal) also fails closed ─
            Case {
                name: "default (equal) operator omitted: non-UTF-8 does not satisfy",
                op: None,
                condition_value: "expected",
                header: NonUtf8(&[0xFF, 0xFE]),
                expect_match: false,
            },
        ]
    }

    /// A `RuleSet` with one rule, one header condition on `x-token`, built
    /// from a real TOML file on disk — `RuleSet` has no cross-crate
    /// literal constructor by design (see its own doc comment), so this
    /// goes through `RuleSet::new` rather than trying to fake one.
    fn rule_set_with_header_condition(dir: &std::path::Path, case: &Case) -> RuleSet {
        let op_line = case
            .op
            .map(|op| format!("op = \"{op}\", "))
            .unwrap_or_default();
        let rs_path = dir.join("apimock-rule-set.toml");
        std::fs::write(
            &rs_path,
            format!(
                "[[rules]]\n\
                 when.request.url_path = \"/probe\"\n\
                 respond.text = \"ok\"\n\
                 [rules.when.request.headers]\n\
                 x-token = {{ {op_line}value = \"{value}\" }}\n",
                value = case.condition_value,
            ),
        )
        .expect("write rule-set file");
        RuleSet::new(rs_path.to_str().unwrap(), dir.to_str().unwrap(), 0).expect("RuleSet::new")
    }

    fn parsed_request_with_header(case: &Case) -> apimock_routing::ParsedRequest {
        let mut builder = hyper::Request::builder().method("GET").uri("/probe");
        if let HeaderState::Utf8(v) = case.header {
            builder = builder.header("x-token", v);
        } else if let HeaderState::NonUtf8(bytes) = case.header {
            builder = builder.header(
                "x-token",
                hyper::header::HeaderValue::from_bytes(bytes)
                    .expect("raw bytes are a structurally valid header value"),
            );
        }
        let (component_parts, _) = builder.body(()).unwrap().into_parts();
        apimock_routing::ParsedRequest::new("/probe".to_owned(), component_parts)
    }

    #[test]
    fn server_and_match_test_agree_on_every_case_in_the_shared_corpus() {
        for case in corpus() {
            let dir = tempfile::tempdir().expect("tempdir");
            let rule_set = rule_set_with_header_condition(dir.path(), &case);
            let rule = &rule_set.rules[0];
            let parsed = parsed_request_with_header(&case);

            let server_matched = rule
                .when
                .request
                .headers
                .as_ref()
                .expect("rule has a headers condition")
                .is_match(&parsed.component_parts.headers, 0, 0);

            let checks = super::evaluate_rule(rule, &parsed);
            let header_check = checks
                .iter()
                .find(|c| c.name == "header:x-token")
                .expect("evaluate_rule must produce a check for the header condition");
            let match_test_matched = header_check.matched;

            assert_eq!(
                server_matched, match_test_matched,
                "{}: server says matched={server_matched}, match-test says matched={match_test_matched} — the two paths disagree",
                case.name
            );
            assert_eq!(
                server_matched, case.expect_match,
                "{}: server and match-test agree with each other (matched={server_matched}) but not with the expected verdict (matched={})",
                case.name, case.expect_match
            );
        }
    }
}
