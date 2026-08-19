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
///
/// # `#[non_exhaustive]` (RFC 052)
///
/// Three RFCs landed on `main` this month each adding a field to one of
/// five public structs including this one, and every addition was,
/// strictly, an undetected breaking API change. `#[non_exhaustive]`
/// makes the next one additive instead: construct with [`Self::new`]
/// (and [`Self::with_body`] where a body needs attaching), not a struct
/// literal — the literal form now only compiles inside this crate:
///
/// ```compile_fail
/// use apimock_routing::ParsedRequest;
///
/// let _ = ParsedRequest {
///     url_path: todo!(),
///     component_parts: todo!(),
///     body_json: todo!(),
///     body_len: todo!(),
/// };
/// ```
#[derive(Debug)]
#[non_exhaustive]
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

impl ParsedRequest {
    /// Construct with no body — the shape every test and bench fixture
    /// in this workspace actually wants (RFC 052 § 2: cross-crate sites
    /// build this "with mostly defaults"). Chain [`Self::with_body`] for
    /// the one case that has a real body to attach.
    pub fn new(url_path: String, component_parts: Parts) -> Self {
        Self {
            url_path,
            component_parts,
            body_json: None,
            body_len: None,
        }
    }

    /// Attach a parsed body. Not folded into [`Self::new`] as extra
    /// parameters: most cross-crate construction sites (test and bench
    /// fixtures with no body to attach) stop at `new` alone, and a
    /// positional `body_json`/`body_len` pair on every one of those calls
    /// would read as `None, None` noise rather than intent.
    pub fn with_body(mut self, body_json: Option<Value>, body_len: Option<usize>) -> Self {
        self.body_json = body_json;
        self.body_len = body_len;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts() -> Parts {
        hyper::Request::builder()
            .method("GET")
            .uri("/x")
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    #[test]
    fn new_has_no_body() {
        let req = ParsedRequest::new("/x".to_owned(), parts());
        assert_eq!(req.url_path, "/x");
        assert!(req.body_json.is_none());
        assert!(req.body_len.is_none());
    }

    #[test]
    fn with_body_attaches_json_and_len() {
        let req = ParsedRequest::new("/x".to_owned(), parts())
            .with_body(Some(serde_json::json!({"a": 1})), Some(9));
        assert_eq!(req.body_json.unwrap()["a"], 1);
        assert_eq!(req.body_len, Some(9));
    }

    #[test]
    fn with_body_can_clear_back_to_no_body() {
        // Chaining twice must leave the second call's values, not merge
        // with the first — `with_body` replaces, it doesn't accumulate.
        let req = ParsedRequest::new("/x".to_owned(), parts())
            .with_body(Some(serde_json::json!({"a": 1})), Some(9))
            .with_body(None, None);
        assert!(req.body_json.is_none());
        assert!(req.body_len.is_none());
    }
}
