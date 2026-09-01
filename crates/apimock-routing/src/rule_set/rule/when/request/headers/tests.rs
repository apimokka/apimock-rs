//! Tests for `Headers::is_match`, `Headers::validate`, and the
//! TOML deserialise surface that feeds them (RFC 017 extended).

use hyper::HeaderMap;
use hyper::header::{HeaderName, HeaderValue};

use super::Headers;

// ── Fixture helpers ───────────────────────────────────────────────────

fn parse_headers(toml_text: &str) -> Headers {
    let wrapped = format!("[headers]\n{}", toml_text);
    #[derive(serde::Deserialize)]
    struct Wrapper {
        headers: Headers,
    }
    let w: Wrapper = toml::from_str(&wrapped).expect("parse headers TOML");
    w.headers
}

fn make_request_headers<I: IntoIterator<Item = (&'static str, &'static str)>>(
    pairs: I,
) -> HeaderMap<HeaderValue> {
    let mut map = HeaderMap::new();
    for (k, v) in pairs {
        map.insert(
            HeaderName::from_static(k),
            HeaderValue::from_str(v).unwrap(),
        );
    }
    map
}

// ── Value operators ───────────────────────────────────────────────────

#[test]
fn is_match_op_default_equal_when_op_omitted() {
    let h = parse_headers(r#"x-foo = { value = "bar" }"#);
    assert!(h.is_match(&make_request_headers([("x-foo", "bar")]), 0, 0));
}

#[test]
fn is_match_op_default_equal_no_match() {
    let h = parse_headers(r#"x-foo = { value = "bar" }"#);
    assert!(!h.is_match(&make_request_headers([("x-foo", "baz")]), 0, 0));
}

#[test]
fn is_match_op_equal_match() {
    let h = parse_headers(r#"x-foo = { op = "equal", value = "bar" }"#);
    assert!(h.is_match(&make_request_headers([("x-foo", "bar")]), 0, 0));
}

#[test]
fn is_match_op_not_equal_match() {
    let h = parse_headers(r#"x-foo = { op = "not_equal", value = "bar" }"#);
    assert!(h.is_match(&make_request_headers([("x-foo", "baz")]), 0, 0));
}

#[test]
fn is_match_op_not_equal_when_equal_returns_false() {
    let h = parse_headers(r#"x-foo = { op = "not_equal", value = "bar" }"#);
    assert!(!h.is_match(&make_request_headers([("x-foo", "bar")]), 0, 0));
}

#[test]
fn is_match_op_starts_with_match() {
    let h = parse_headers(r#"authorization = { op = "starts_with", value = "Bearer " }"#);
    assert!(h.is_match(
        &make_request_headers([("authorization", "Bearer token123")]),
        0,
        0
    ));
}

#[test]
fn is_match_op_starts_with_no_match() {
    let h = parse_headers(r#"authorization = { op = "starts_with", value = "Bearer " }"#);
    assert!(!h.is_match(
        &make_request_headers([("authorization", "Basic abc")]),
        0,
        0
    ));
}

#[test]
fn is_match_op_ends_with_match() {
    // RFC 017: EndsWith now works correctly (was previously falling back to Contains).
    let h = parse_headers(r#"x-trace = { op = "ends_with", value = "-done" }"#);
    assert!(h.is_match(&make_request_headers([("x-trace", "req-1234-done")]), 0, 0));
}

#[test]
fn is_match_op_ends_with_no_match() {
    let h = parse_headers(r#"x-trace = { op = "ends_with", value = "-done" }"#);
    assert!(!h.is_match(
        &make_request_headers([("x-trace", "req-1234-pending")]),
        0,
        0
    ));
}

#[test]
fn is_match_op_ends_with_distinguishes_from_contains() {
    // "-done" appears in the middle; ends_with must not match.
    let h = parse_headers(r#"x-trace = { op = "ends_with", value = "-done" }"#);
    assert!(!h.is_match(&make_request_headers([("x-trace", "req-done-final")]), 0, 0));
}

#[test]
fn is_match_op_contains_match() {
    let h = parse_headers(r#"x-foo = { op = "contains", value = "ar" }"#);
    assert!(h.is_match(&make_request_headers([("x-foo", "bar")]), 0, 0));
}

#[test]
fn is_match_op_contains_no_match() {
    let h = parse_headers(r#"x-foo = { op = "contains", value = "zz" }"#);
    assert!(!h.is_match(&make_request_headers([("x-foo", "bar")]), 0, 0));
}

#[test]
fn is_match_op_wild_card_match() {
    let h = parse_headers(r#"x-foo = { op = "wild_card", value = "b*r" }"#);
    assert!(h.is_match(&make_request_headers([("x-foo", "bar")]), 0, 0));
    assert!(h.is_match(&make_request_headers([("x-foo", "beer")]), 0, 0));
}

#[test]
fn is_match_op_regex_match() {
    // RFC 017: Regex now works correctly (was previously falling back to Equal).
    let h = parse_headers(r#"content-type = { op = "regex", value = "^application/(json|xml)$" }"#);
    assert!(h.is_match(
        &make_request_headers([("content-type", "application/json")]),
        0,
        0
    ));
    assert!(h.is_match(
        &make_request_headers([("content-type", "application/xml")]),
        0,
        0
    ));
    assert!(!h.is_match(
        &make_request_headers([("content-type", "text/plain")]),
        0,
        0
    ));
}

// ── Presence operators (RFC 017) ──────────────────────────────────────

#[test]
fn is_match_op_exists_header_present() {
    // RFC 017: Exists now works (was silently Equal "").
    let h = parse_headers(r#"x-api-key = { op = "exists" }"#);
    assert!(h.is_match(&make_request_headers([("x-api-key", "any-value")]), 0, 0));
}

#[test]
fn is_match_op_exists_header_present_empty_value() {
    let h = parse_headers(r#"x-api-key = { op = "exists" }"#);
    // Key present with empty string → Exists should still match.
    assert!(h.is_match(&make_request_headers([("x-api-key", "")]), 0, 0));
}

#[test]
fn is_match_op_exists_header_absent() {
    let h = parse_headers(r#"x-api-key = { op = "exists" }"#);
    assert!(!h.is_match(&make_request_headers([("other-header", "value")]), 0, 0));
}

#[test]
fn is_match_op_absent_header_absent() {
    // RFC 017: Absent now works (was silently Equal "").
    let h = parse_headers(r#"x-internal = { op = "absent" }"#);
    assert!(h.is_match(&make_request_headers([("other-header", "value")]), 0, 0));
}

#[test]
fn is_match_op_absent_header_present() {
    let h = parse_headers(r#"x-internal = { op = "absent" }"#);
    assert!(!h.is_match(&make_request_headers([("x-internal", "secret")]), 0, 0));
}

// ── Key-missing / AND-logic ───────────────────────────────────────────

#[test]
fn is_match_key_missing_returns_false() {
    let h = parse_headers(r#"x-foo = { value = "bar" }"#);
    assert!(!h.is_match(&make_request_headers([("other", "bar")]), 0, 0));
}

#[test]
fn is_match_multiple_conditions_all_match() {
    let h = parse_headers(
        r#"x-tenant = { value = "acme" }
x-role = { op = "starts_with", value = "admin" }"#,
    );
    let req = make_request_headers([("x-tenant", "acme"), ("x-role", "admin-user")]);
    assert!(h.is_match(&req, 0, 0));
}

#[test]
fn is_match_multiple_conditions_one_fails() {
    let h = parse_headers(
        r#"x-tenant = { value = "acme" }
x-role = { op = "starts_with", value = "admin" }"#,
    );
    let req = make_request_headers([("x-tenant", "acme"), ("x-role", "viewer")]);
    assert!(!h.is_match(&req, 0, 0));
}

#[test]
fn is_match_utf8_decode_failure_returns_false() {
    // RFC 072: a header condition is a gate; a value that cannot be read
    // as UTF-8 does not satisfy it, regardless of the operator — fail
    // closed, not fail open. See `rule_check.rs`'s agreement test in the
    // `apimock` crate for the corpus proving this holds across every
    // operator, and that `match-test` agrees.
    use hyper::header::HeaderName;
    let h = parse_headers(r#"x-bin = { value = "anything" }"#);
    let mut map = HeaderMap::new();
    map.insert(
        HeaderName::from_static("x-bin"),
        HeaderValue::from_bytes(b"\xff\xfe").unwrap(),
    );
    assert!(!h.is_match(&map, 0, 0));
}

#[test]
fn is_match_utf8_decode_failure_still_satisfies_exists() {
    // The header genuinely is present; `exists` only asks that, and
    // never attempts to decode the value.
    use hyper::header::HeaderName;
    let h = parse_headers(r#"x-bin = { op = "exists" }"#);
    let mut map = HeaderMap::new();
    map.insert(
        HeaderName::from_static("x-bin"),
        HeaderValue::from_bytes(b"\xff\xfe").unwrap(),
    );
    assert!(h.is_match(&map, 0, 0));
}

#[test]
fn is_match_utf8_decode_failure_does_not_satisfy_absent() {
    // "Cannot be read" is not "not present" — a present-but-undecodable
    // header must not satisfy `absent`. The deliberate RFC 072 decision.
    use hyper::header::HeaderName;
    let h = parse_headers(r#"x-bin = { op = "absent" }"#);
    let mut map = HeaderMap::new();
    map.insert(
        HeaderName::from_static("x-bin"),
        HeaderValue::from_bytes(b"\xff\xfe").unwrap(),
    );
    assert!(!h.is_match(&map, 0, 0));
}

// ── Validate ─────────────────────────────────────────────────────────

#[test]
fn validate_empty_returns_false() {
    use indexmap::IndexMap;
    let h = Headers(IndexMap::new());
    assert!(!h.validate());
}

#[test]
fn validate_non_empty_returns_true() {
    let h = parse_headers(r#"x-foo = { value = "bar" }"#);
    assert!(h.validate());
}

// ── Deserialise surface ───────────────────────────────────────────────

#[test]
fn deserialize_value_only() {
    let h = parse_headers(r#"x-foo = { value = "bar" }"#);
    assert_eq!(h.0["x-foo"].value, "bar");
    assert!(h.0["x-foo"].op.is_none());
}

#[test]
fn deserialize_all_value_op_variants() {
    let h = parse_headers(
        r#"a = { op = "equal",      value = "x" }
b = { op = "not_equal",  value = "x" }
c = { op = "starts_with",value = "x" }
d = { op = "ends_with",  value = "x" }
e = { op = "contains",   value = "x" }
f = { op = "wild_card",  value = "x" }
g = { op = "regex",      value = "x" }"#,
    );
    assert_eq!(h.0.len(), 7);
    for key in ["a", "b", "c", "d", "e", "f", "g"] {
        assert!(h.0[key].op.is_some(), "op missing for `{}`", key);
    }
}

#[test]
fn deserialize_presence_op_variants() {
    // Presence operators have no `value` key in TOML.
    let h = parse_headers(
        r#"x-present = { op = "exists" }
x-absent  = { op = "absent" }"#,
    );
    assert_eq!(h.0.len(), 2);
    assert!(h.0["x-present"].op.is_some());
    assert!(h.0["x-absent"].op.is_some());
}

#[test]
fn deserialize_multiple_keys_preserve_each_value() {
    let h = parse_headers(
        r#"x-one   = { value = "alpha" }
x-two   = { value = "beta"  }
x-three = { value = "gamma" }"#,
    );
    assert_eq!(h.0.len(), 3);
    assert_eq!(h.0["x-one"].value, "alpha");
    assert_eq!(h.0["x-two"].value, "beta");
    assert_eq!(h.0["x-three"].value, "gamma");
}

// ── RFC 014: IndexMap insertion-order ────────────────────────────────

#[test]
fn headers_programmatic_insertion_preserves_order() {
    use super::HeaderConditionStatement;
    use indexmap::IndexMap;

    let mut map: IndexMap<String, HeaderConditionStatement> = IndexMap::new();
    map.insert(
        "z-header".to_owned(),
        HeaderConditionStatement {
            op: None,
            value: "z".to_owned(),
        },
    );
    map.insert(
        "a-header".to_owned(),
        HeaderConditionStatement {
            op: None,
            value: "a".to_owned(),
        },
    );
    map.insert(
        "m-header".to_owned(),
        HeaderConditionStatement {
            op: None,
            value: "m".to_owned(),
        },
    );

    let h = Headers(map);
    let keys: Vec<&str> = h.0.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["z-header", "a-header", "m-header"]);
}

#[test]
fn when_view_headers_programmatic_insertion_order() {
    use super::HeaderConditionStatement;
    use crate::rule_set::rule::when::When;
    use crate::rule_set::rule::when::request::Request;
    use crate::view::build::build_when_view;
    use indexmap::IndexMap;

    let mut map: IndexMap<String, HeaderConditionStatement> = IndexMap::new();
    map.insert(
        "z".to_owned(),
        HeaderConditionStatement {
            op: None,
            value: "".to_owned(),
        },
    );
    map.insert(
        "a".to_owned(),
        HeaderConditionStatement {
            op: None,
            value: "".to_owned(),
        },
    );
    map.insert(
        "m".to_owned(),
        HeaderConditionStatement {
            op: None,
            value: "".to_owned(),
        },
    );

    let when = When {
        request: Request {
            url_path_config: None,
            url_path: None,
            http_method: None,
            headers: Some(Headers(map)),
            body: None,
        },
    };
    let view = build_when_view(&when);
    let names: Vec<&str> = view.headers.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["z", "a", "m"]);
}

// ── RFC 021: negated header operator tests ────────────────────────────

#[test]
fn is_match_op_not_contains_match() {
    let h = parse_headers(r#"x-foo = { op = "not_contains", value = "bar" }"#);
    assert!(h.is_match(&make_request_headers([("x-foo", "baz")]), 0, 0));
    assert!(!h.is_match(&make_request_headers([("x-foo", "foobar")]), 0, 0));
}

#[test]
fn is_match_op_not_starts_with_match() {
    let h = parse_headers(r#"authorization = { op = "not_starts_with", value = "Bearer " }"#);
    assert!(h.is_match(
        &make_request_headers([("authorization", "Basic abc")]),
        0,
        0
    ));
    assert!(!h.is_match(
        &make_request_headers([("authorization", "Bearer token")]),
        0,
        0
    ));
}

#[test]
fn is_match_op_not_ends_with_match() {
    let h = parse_headers(r#"x-trace = { op = "not_ends_with", value = "-done" }"#);
    assert!(h.is_match(
        &make_request_headers([("x-trace", "req-1234-pending")]),
        0,
        0
    ));
    assert!(!h.is_match(&make_request_headers([("x-trace", "req-1234-done")]), 0, 0));
}

#[test]
fn is_match_op_not_regex_match() {
    let h =
        parse_headers(r#"content-type = { op = "not_regex", value = "^application/(json|xml)$" }"#);
    assert!(h.is_match(
        &make_request_headers([("content-type", "text/plain")]),
        0,
        0
    ));
    assert!(!h.is_match(
        &make_request_headers([("content-type", "application/json")]),
        0,
        0
    ));
}

#[test]
fn is_match_negated_op_missing_key_returns_false() {
    // Negated value operators still require the key to be present.
    let h = parse_headers(r#"x-required = { op = "not_contains", value = "admin" }"#);
    assert!(!h.is_match(&make_request_headers([("other-header", "value")]), 0, 0));
}
