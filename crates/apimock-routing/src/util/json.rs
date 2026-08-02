//! JSON-body inspection used by rule matching.
//!
//! # Why only one helper lives here
//!
//! Pre-5.0 this module also contained `resolve_with_json_compatible_extensions`
//! and `is_equivalent_json_file` — both of which are filesystem-touching
//! utilities used by the dyn-route fallback. In the 5.0 split those
//! moved to `apimock-server` where they belong. What stays here is the
//! one helper the *matcher* calls on every request with a JSON body.

use serde_json::Value;

/// File extensions treated as "JSON-compatible" for the fallback route.
///
/// Referenced from `apimock-server` via re-export for the dyn-route
/// fallback. Stays in routing because the *definition* of what counts
/// as a JSON-like file is a routing-semantics concern.
pub const JSON_COMPATIBLE_EXTENSIONS: [&str; 3] = ["json", "json5", "csv"];

/// Look up a value inside a JSON document using a dotted-path key.
///
/// # Why a home-rolled mini-syntax instead of canonical JSONPath
///
/// We only support the "object key" and "array index" forms — no
/// wildcards, no filters, no expression language. Those two cover
/// every real use inside this codebase (`body.json` matchers, CSV
/// record-wrapping key). Pulling in a full JSONPath crate would add
/// weight and expose features we would then have to teach users to
/// avoid.
///
/// **This is not canonical JSONPath / RFC 9535.** The leading `$.`
/// of canonical JSONPath is *not* recognised — `$` is treated as a
/// literal object key, which is almost never what the writer
/// intended. A path like `"$.foo.bar"` therefore matches an object
/// that has a top-level `$` key, *not* the value at `foo.bar`. Use
/// the dotted form below.
///
/// Supported shapes:
/// - `a.b.c` for nested object keys
/// - `0.foo` / `items.2.name` for array indexing with a numeric segment
///
/// Returns `None` if any segment is missing or has the wrong shape
/// (e.g. trying to index an object with a numeric string, or vice versa).
pub fn json_value_by_jsonpath<'a>(value: &'a Value, jsonpath: &str) -> Option<&'a Value> {
    jsonpath
        .split('.')
        .try_fold(value, |current, key| match current {
            Value::Object(map) => map.get(key),
            Value::Array(arr) => key.parse::<usize>().ok().and_then(|i| arr.get(i)),
            _ => None,
        })
}
