//! The parsed form of an incoming HTTP request that the matcher consumes.
//!
//! # Why the struct lives in `apimock-routing` but the constructor doesn't
//!
//! `ParsedRequest` is what [`RuleSet::find_matched`](crate::RuleSet::find_matched)
//! takes as input — so the matcher crate owns the type. But populating
//! one from a `hyper::Request<Incoming>` requires reading the request
//! body (async), looking at headers, and normalising the URL path.
//! That logic is an HTTP-layer activity, so the constructor lives in
//! `apimock-server::parsed_request`. Tests and benches can hand-build a
//! `ParsedRequest` without going through the server code.

use hyper::http::request::Parts;
use serde_json::Value;

/// Request metadata and body, decoded once per request.
///
/// # Why we eagerly collect the body
///
/// Routing decisions depend on body contents (rule-set `body.json`
/// conditions, middleware evaluation). Rather than re-collecting the
/// body each time a matcher asks for it, the upstream constructor
/// consumes the `Incoming` stream once and keeps the parsed JSON
/// around for the lifetime of the request. This is appropriate for a
/// mock server where payloads are small; a production proxy would want
/// streaming instead.
#[derive(Debug)]
pub struct ParsedRequest {
    pub url_path: String,
    pub component_parts: Parts,
    /// Parsed JSON body, if the request had one that parsed successfully.
    /// `None` here means either "no body" or "body present but not JSON";
    /// the two are indistinguishable at the matcher layer and we don't
    /// currently need to distinguish them.
    pub body_json: Option<Value>,
    /// Byte length of the request body, if one arrived. `None` means no
    /// body at all. Populated whether or not the body parsed as JSON, so
    /// together with `body_json` it resolves the ambiguity that field's
    /// own doc comment describes: `body_json: None, body_len: None` is
    /// "no body"; `body_json: None, body_len: Some(n)` is "an `n`-byte
    /// body that wasn't JSON" (RFC 050 — trace-event consumption of this
    /// is presence-and-length only, never content).
    pub body_len: Option<usize>,
}
