//! Tests for `Headers::is_match`, `Headers::validate`, and the
//! TOML deserialise surface that feeds them.
//!
//! # What this module covers
//!
//! 5.6.0 added these tests to fill a long-standing gap in the routing
//! crate's coverage. Until 5.5.0, `Headers::is_match` was exercised
//! only indirectly (through full integration runs in `apimock`'s test
//! binaries, and through `apimock-config`'s round-trip tests in
//! 5.5.0). The matcher's variant-by-variant behaviour, the missing-
//! key path, and the multi-condition AND evaluation have no other
//! direct coverage.
//!
//! # Why we test through TOML deserialise rather than struct literals
//!
//! Building a `Headers` value by hand requires importing
//! `ConditionStatement`, `RuleOp`, and the `ConditionKey` alias —
//! verbose and prone to drifting out of sync with the on-disk shape.
//! TOML fixtures double as deserialise-surface tests: every successful
//! parse confirms the serde wiring still works.

use std::collections::HashMap;

use hyper::HeaderMap;
use hyper::header::{HeaderName, HeaderValue};

use super::Headers;

// ---------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------

/// Parse a TOML fragment of the shape that appears under
/// `[when.request.headers]` into a `Headers` value. Strips one level
/// of TOML indirection so test bodies can read like the on-disk form.
fn parse_headers(toml_text: &str) -> Headers {
    // Wrap the fragment in a top-level `headers` key so the
    // serde transparent shape can deserialise.
    let wrapped = format!("[headers]\n{}", toml_text);
    #[derive(serde::Deserialize)]
    struct Wrapper {
        headers: Headers,
    }
    let w: Wrapper = toml::from_str(&wrapped).expect("parse headers TOML");
    w.headers
}

/// Build a `HeaderMap` from `(key, value)` pairs.
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

// ---------------------------------------------------------------------
// is_match — operator coverage
// ---------------------------------------------------------------------

#[test]
fn is_match_op_default_equal_when_op_omitted() {
    // No `op` field — defaults to Equal.
    let headers = parse_headers(r#"x-api-key = { value = "secret" }"#);
    let req = make_request_headers([("x-api-key", "secret")]);
    assert!(headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_default_equal_no_match() {
    let headers = parse_headers(r#"x-api-key = { value = "secret" }"#);
    let req = make_request_headers([("x-api-key", "wrong")]);
    assert!(!headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_equal_match() {
    let headers = parse_headers(r#"x-api-key = { op = "equal", value = "secret" }"#);
    let req = make_request_headers([("x-api-key", "secret")]);
    assert!(headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_not_equal_match() {
    let headers = parse_headers(r#"x-api-key = { op = "not_equal", value = "secret" }"#);
    let req = make_request_headers([("x-api-key", "different")]);
    assert!(headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_not_equal_when_equal_returns_false() {
    let headers = parse_headers(r#"x-api-key = { op = "not_equal", value = "secret" }"#);
    let req = make_request_headers([("x-api-key", "secret")]);
    assert!(!headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_starts_with_match() {
    let headers = parse_headers(r#"user-agent = { op = "starts_with", value = "Mozilla" }"#);
    let req = make_request_headers([("user-agent", "Mozilla/5.0 (X11)")]);
    assert!(headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_starts_with_no_match() {
    let headers = parse_headers(r#"user-agent = { op = "starts_with", value = "Mozilla" }"#);
    let req = make_request_headers([("user-agent", "curl/8.4")]);
    assert!(!headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_contains_match() {
    let headers = parse_headers(r#"user-agent = { op = "contains", value = "Firefox" }"#);
    let req = make_request_headers([("user-agent", "Mozilla/5.0 Firefox/120")]);
    assert!(headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_contains_no_match() {
    let headers = parse_headers(r#"user-agent = { op = "contains", value = "Firefox" }"#);
    let req = make_request_headers([("user-agent", "Mozilla/5.0 Chrome/120")]);
    assert!(!headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_op_wild_card_match() {
    let headers = parse_headers(r#"user-agent = { op = "wild_card", value = "Mozilla*" }"#);
    let req = make_request_headers([("user-agent", "Mozilla/5.0 anything-here")]);
    assert!(headers.is_match(&req, 0, 0));
}

// ---------------------------------------------------------------------
// is_match — request-shape coverage
// ---------------------------------------------------------------------

#[test]
fn is_match_key_missing_returns_false() {
    let headers = parse_headers(r#"x-api-key = { value = "secret" }"#);
    // Request has no x-api-key header at all.
    let req = make_request_headers([("user-agent", "x")]);
    assert!(!headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_multiple_conditions_all_match() {
    let headers = parse_headers(
        r#"x-api-key = { value = "secret" }
x-tenant = { op = "equal", value = "acme" }"#,
    );
    let req = make_request_headers([("x-api-key", "secret"), ("x-tenant", "acme")]);
    assert!(headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_multiple_conditions_one_fails() {
    let headers = parse_headers(
        r#"x-api-key = { value = "secret" }
x-tenant = { op = "equal", value = "acme" }"#,
    );
    let req = make_request_headers([("x-api-key", "secret"), ("x-tenant", "wrong")]);
    assert!(!headers.is_match(&req, 0, 0));
}

#[test]
fn is_match_utf8_decode_failure_returns_true() {
    // HeaderValue accepts arbitrary bytes; non-ASCII bytes fail
    // `to_str()`. Headers::is_match's documented behaviour in that
    // case is to log and return `true` (treat as match) so the rule
    // still has a chance to run.
    //
    // This pins that behaviour. If the project later decides UTF-8
    // failures should be `false`, this test is the place to flip
    // expectations.
    let headers = parse_headers(r#"x-thing = { value = "anything" }"#);
    let mut req = HeaderMap::new();
    req.insert(
        HeaderName::from_static("x-thing"),
        HeaderValue::from_bytes(&[0xff, 0xfe, 0x80]).unwrap(),
    );
    assert!(headers.is_match(&req, 0, 0));
}

// ---------------------------------------------------------------------
// validate
// ---------------------------------------------------------------------

#[test]
fn validate_empty_returns_false() {
    let h = Headers(HashMap::new());
    assert!(!h.validate());
}

#[test]
fn validate_non_empty_returns_true() {
    let h = parse_headers(r#"x-api-key = { value = "v" }"#);
    assert!(h.validate());
}

// ---------------------------------------------------------------------
// TOML deserialise — surface confirmation
// ---------------------------------------------------------------------

#[test]
fn deserialize_value_only() {
    let h = parse_headers(r#"x-api-key = { value = "secret" }"#);
    assert!(h.0.contains_key("x-api-key"));
    assert!(h.0["x-api-key"].op.is_none());
    assert_eq!(h.0["x-api-key"].value, "secret");
}

#[test]
fn deserialize_with_each_op_variant() {
    // One TOML doc carrying every op the routing crate accepts —
    // confirms the serde rename_all = "snake_case" wiring across all
    // five variants in one shot.
    let h = parse_headers(
        r#"a = { op = "equal", value = "x" }
b = { op = "not_equal", value = "x" }
c = { op = "starts_with", value = "x" }
d = { op = "contains", value = "x" }
e = { op = "wild_card", value = "x" }"#,
    );
    assert_eq!(h.0.len(), 5);
    for key in ["a", "b", "c", "d", "e"] {
        assert!(h.0[key].op.is_some(), "op missing for `{}`", key);
    }
}

#[test]
fn deserialize_multiple_keys_preserve_each_value() {
    let h = parse_headers(
        r#"x-one = { value = "alpha" }
x-two = { value = "beta" }
x-three = { value = "gamma" }"#,
    );
    assert_eq!(h.0.len(), 3);
    assert_eq!(h.0["x-one"].value, "alpha");
    assert_eq!(h.0["x-two"].value, "beta");
    assert_eq!(h.0["x-three"].value, "gamma");
}
