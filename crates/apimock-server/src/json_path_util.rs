//! Filesystem helpers for resolving URL paths onto JSON-compatible files.
//!
//! # Why these live in server, not routing
//!
//! Pre-5.0 these sat alongside `json_value_by_jsonpath` in
//! `core::util::json`. But `resolve_with_json_compatible_extensions`
//! and `is_equivalent_json_file` both touch the filesystem to answer
//! "does this path exist as a file?". That's a server-layer activity —
//! the routing layer stays abstract and doesn't care where bytes come
//! from — so they moved here in 5.0.

use std::path::Path;

use apimock_routing::util::json::JSON_COMPATIBLE_EXTENSIONS;

/// Directory index filename (same spirit as `index.html`).
pub const ROOT_DIRECTORY_FILE_NAME: &str = "index";

/// Try to resolve a user-visible URL path onto a real file on disk.
///
/// Resolution order (first hit wins):
/// 1. Exact match (`unknown_path` is already a file).
/// 2. `unknown_path.<ext>` for each extension in `JSON_COMPATIBLE_EXTENSIONS`.
/// 3. `unknown_path/index.<ext>` for the same extensions.
/// 4. `unknown_path/index.html` — present so the fallback responder
///    can also serve a directory root HTML, matching common
///    web-server conventions.
pub fn resolve_with_json_compatible_extensions(unknown_path: &str) -> Option<String> {
    let p = Path::new(unknown_path);
    if p.is_file() {
        return Some(unknown_path.to_owned());
    }

    for ext in JSON_COMPATIBLE_EXTENSIONS {
        let with_ext = format!("{}.{}", unknown_path, ext);
        if Path::new(&with_ext).is_file() {
            return Some(with_ext);
        }
    }

    for ext in JSON_COMPATIBLE_EXTENSIONS {
        let p = Path::new(unknown_path).join(format!("{}.{}", ROOT_DIRECTORY_FILE_NAME, ext));
        if p.is_file() {
            return p
                .canonicalize()
                .ok()
                .and_then(|c| c.to_str().map(|s| s.to_owned()));
        }
    }

    let p = Path::new(unknown_path).join(format!("{}.html", ROOT_DIRECTORY_FILE_NAME));
    if p.is_file() {
        return p
            .canonicalize()
            .ok()
            .and_then(|c| c.to_str().map(|s| s.to_owned()));
    }

    None
}
