//! Tests for `Body::is_match`, `Body::validate`, and the TOML
//! deserialise surface that feeds them.
//!
//! # Coverage scope
//!
//! 5.6.0 added these tests for the same reason as
//! `headers::tests` — to fill a gap left when the routing crate's
//! original test surface stopped at `RuleOp` and `glob`. `Body`
//! evaluates request bodies against jsonpath-keyed conditions, with
//! several behavioural quirks (value coercion to string, multi-path
//! AND, missing-path → false) that are easy to regress without
//! direct tests.
//!
//! # On the path syntax
//!
//! `body.json` keys use the routing crate's dotted-path mini-syntax,
//! not standard JSONPath. The supported shapes are object keys
//! (`a.b.c`) and array indexing (`items.2.name`); the leading `$.`
//! and bracket-notation forms of canonical JSONPath are **not**
//! supported. See `apimock_routing::util::json::json_value_by_jsonpath`
//! for the full contract. These tests use the supported form.

use std::collections::HashMap;

use hyper::Request;
use serde_json::{Value, json};

use super::Body;
use super::body_kind::BodyKind;
use crate::parsed_request::ParsedRequest;

// ---------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------

/// Parse a TOML fragment of the shape that appears under
/// `[when.request.body]` into a `Body` value.
fn parse_body(toml_text: &str) -> Body {
    // Wrap the fragment so the transparent shape can deserialise.
    let wrapped = format!("[body]\n{}", toml_text);
    #[derive(serde::Deserialize)]
    struct Wrapper {
        body: Body,
    }
    let w: Wrapper = toml::from_str(&wrapped).expect("parse body TOML");
    w.body
}

fn make_parsed_request(body_json: Option<Value>) -> ParsedRequest {
    let req = Request::builder()
        .method("POST")
        .uri("/test")
        .body(())
        .expect("build request");
    let (component_parts, _) = req.into_parts();
    ParsedRequest {
        url_path: "/test".to_owned(),
        component_parts,
        body_json,
    }
}

// ---------------------------------------------------------------------
// is_match — request shape
// ---------------------------------------------------------------------

#[test]
fn is_match_no_body_returns_false() {
    let body = parse_body(r#"json."x" = { value = "y" }"#);
    let req = make_parsed_request(None);
    assert!(!body.is_match(&req));
}

#[test]
fn is_match_no_json_kind_returns_false() {
    // A Body whose outer map has no Json key (constructed by hand —
    // TOML can't currently express this since BodyKind only has Json).
    let body = Body(HashMap::new());
    let req = make_parsed_request(Some(json!({"x": "y"})));
    assert!(!body.is_match(&req));
}

#[test]
fn is_match_empty_json_kind_returns_false() {
    // Json key exists but its inner map is empty.
    let mut outer = HashMap::new();
    outer.insert(BodyKind::Json, HashMap::new());
    let body = Body(outer);
    let req = make_parsed_request(Some(json!({"x": "y"})));
    assert!(!body.is_match(&req));
}

// ---------------------------------------------------------------------
// is_match — operator coverage on jsonpath hits
// ---------------------------------------------------------------------

#[test]
fn is_match_jsonpath_hit_equal() {
    let body = parse_body(r#"json."action" = { op = "equal", value = "go" }"#);
    let req = make_parsed_request(Some(json!({"action": "go"})));
    assert!(body.is_match(&req));
}

#[test]
fn is_match_jsonpath_hit_starts_with() {
    let body = parse_body(r#"json."tag" = { op = "starts_with", value = "v1" }"#);
    let req = make_parsed_request(Some(json!({"tag": "v1.2.3"})));
    assert!(body.is_match(&req));
}

#[test]
fn is_match_jsonpath_hit_contains() {
    let body = parse_body(r#"json."message" = { op = "contains", value = "error" }"#);
    let req = make_parsed_request(Some(json!({"message": "an error occurred"})));
    assert!(body.is_match(&req));
}

#[test]
fn is_match_jsonpath_miss_returns_false() {
    let body = parse_body(r#"json."missing" = { value = "x" }"#);
    let req = make_parsed_request(Some(json!({"present": "x"})));
    assert!(!body.is_match(&req));
}

// ---------------------------------------------------------------------
// is_match — non-string JSON value coercion
// ---------------------------------------------------------------------

#[test]
fn is_match_jsonpath_value_number_coerced_to_string() {
    // The matcher stringifies non-String JSON values via `to_string()`
    // and compares with `RuleOp::is_match`. For Number, `to_string()`
    // produces e.g. "42".
    let body = parse_body(r#"json."count" = { value = "42" }"#);
    let req = make_parsed_request(Some(json!({"count": 42})));
    assert!(body.is_match(&req));
}

#[test]
fn is_match_jsonpath_value_object_coerced_to_string() {
    // `to_string()` on a JSON Object yields a compact JSON encoding
    // like `{"k":"v"}`. We pin that as the matcher's expectation so a
    // GUI rule writer knows what to compare against.
    let body = parse_body(r#"json."obj" = { value = "{\"k\":\"v\"}" }"#);
    let req = make_parsed_request(Some(json!({"obj": {"k": "v"}})));
    assert!(body.is_match(&req));
}

// ---------------------------------------------------------------------
// is_match — multi-condition AND
// ---------------------------------------------------------------------

#[test]
fn is_match_multiple_jsonpaths_all_match() {
    let body = parse_body(
        r#"json."action" = { value = "go" }
json."user" = { op = "starts_with", value = "alice" }"#,
    );
    let req = make_parsed_request(Some(json!({"action": "go", "user": "alice42"})));
    assert!(body.is_match(&req));
}

#[test]
fn is_match_multiple_jsonpaths_one_fails() {
    let body = parse_body(
        r#"json."action" = { value = "go" }
json."user" = { op = "starts_with", value = "alice" }"#,
    );
    let req = make_parsed_request(Some(json!({"action": "go", "user": "bob"})));
    assert!(!body.is_match(&req));
}

// ---------------------------------------------------------------------
// validate
// ---------------------------------------------------------------------

#[test]
fn validate_empty_outer_returns_false() {
    let body = Body(HashMap::new());
    assert!(!body.validate());
}

#[test]
fn validate_empty_inner_returns_false() {
    let mut outer = HashMap::new();
    outer.insert(BodyKind::Json, HashMap::new());
    let body = Body(outer);
    assert!(!body.validate());
}

#[test]
fn validate_non_empty_returns_true() {
    let body = parse_body(r#"json."x" = { value = "y" }"#);
    assert!(body.validate());
}

// ---------------------------------------------------------------------
// TOML deserialise — surface confirmation
// ---------------------------------------------------------------------

#[test]
fn deserialize_simple_jsonpath() {
    let body = parse_body(r#"json."foo" = { value = "bar" }"#);
    let inner = body.0.get(&BodyKind::Json).expect("json kind present");
    assert!(inner.contains_key("foo"));
    assert_eq!(inner["foo"].value, "bar");
}

#[test]
fn deserialize_nested_jsonpath() {
    let body = parse_body(r#"json."user.address.city" = { value = "Tokyo" }"#);
    let inner = body.0.get(&BodyKind::Json).unwrap();
    assert!(inner.contains_key("user.address.city"));
}

#[test]
fn deserialize_multiple_jsonpaths() {
    let body = parse_body(
        r#"json."a" = { value = "1" }
json."b" = { op = "equal", value = "2" }
json."c" = { op = "starts_with", value = "3" }"#,
    );
    let inner = body.0.get(&BodyKind::Json).unwrap();
    assert_eq!(inner.len(), 3);
}
