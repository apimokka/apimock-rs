use std::path::Path;

use serde_json::Value;

use crate::core::server::constant::ROOT_DIRECTORY_FILE_NAME;

/// File extensions treated as "JSON-compatible" for the fallback route.
///
/// # Why CSV is in this list
///
/// The CSV responder transparently converts rows into a JSON array, so
/// from the client's perspective a `.csv` file behind a URL produces a
/// JSON response just like a `.json` file would. Treating the three
/// extensions as siblings keeps `/foo` → `foo.csv` shortening working.
pub const JSON_COMPATIBLE_EXTENSIONS: [&str; 3] = ["json", "json5", "csv"];

/// Try to resolve a user-visible URL path onto a real file on disk.
///
/// Resolution order (first hit wins):
/// 1. Exact match (`unknown_path` is already a file).
/// 2. `unknown_path.<ext>` for each extension in `JSON_COMPATIBLE_EXTENSIONS`.
/// 3. `unknown_path/index.<ext>` for the same extensions.
/// 4. `unknown_path/index.html` — present so the fallback responder can
///    also serve a directory root HTML, matching common web-server
///    conventions.
///
/// # Why canonicalize failures are swallowed quietly
///
/// `canonicalize` can fail on race conditions (file unlinked between
/// `is_file` and the call). We prefer to return `None` in those rare
/// cases rather than surface the error, since the caller will already
/// produce a 404 — the user-visible result is the same.
pub fn resolve_with_json_compatible_extensions(unknown_path: &str) -> Option<String> {
    // url path is directly pointed to server file
    let p = Path::new(unknown_path);
    if p.is_file() {
        return Some(unknown_path.to_owned());
    }

    // extensions support
    for ext in JSON_COMPATIBLE_EXTENSIONS {
        let with_ext = format!("{}.{}", unknown_path, ext);
        if Path::new(&with_ext).is_file() {
            return Some(with_ext);
        }
    }

    // directory root file name and extensions support
    for ext in JSON_COMPATIBLE_EXTENSIONS {
        let p = Path::new(unknown_path).join(format!("{}.{}", ROOT_DIRECTORY_FILE_NAME, ext));
        if p.is_file() {
            return p
                .canonicalize()
                .ok()
                .and_then(|c| c.to_str().map(|s| s.to_owned()));
        }
    }

    // directory root file name as html
    let p = Path::new(unknown_path).join(format!("{}.html", ROOT_DIRECTORY_FILE_NAME));
    if p.is_file() {
        return p
            .canonicalize()
            .ok()
            .and_then(|c| c.to_str().map(|s| s.to_owned()));
    }

    // missing
    None
}

/// Check whether two paths refer to the same logical JSON-compatible file
/// ignoring which specific extension each one has.
///
/// # Why this helper exists
///
/// Some rule-set entries reference `foo.json` but the on-disk file might
/// be `foo.csv` (if the operator migrated to CSV). This lets route-analysis
/// code treat them as equivalent without caring which extension landed.
/// Returns `false` for any path missing a stem or with a non-UTF-8 one.
pub fn is_equivalent_json_file(request_path: &Path, entry_path: &Path) -> bool {
    let Some(request_file_stem) = request_path.file_stem() else {
        return false;
    };
    let Some(entry_file_stem) = entry_path.file_stem() else {
        return false;
    };

    let request_ext = request_path.extension().unwrap_or_default().to_ascii_lowercase();
    let entry_ext = entry_path.extension().unwrap_or_default().to_ascii_lowercase();

    let Some(request_ext_str) = request_ext.to_str() else {
        return false;
    };
    let Some(entry_ext_str) = entry_ext.to_str() else {
        return false;
    };

    request_file_stem == entry_file_stem
        && JSON_COMPATIBLE_EXTENSIONS.contains(&request_ext_str)
        && JSON_COMPATIBLE_EXTENSIONS.contains(&entry_ext_str)
}

/// Look up a value inside a JSON document using a dotted-path key.
///
/// # Why a home-rolled mini-JSONPath instead of a crate
///
/// We only support the "object key" and "array index" forms — no wildcards,
/// no filters. Those two cover every real use inside this codebase
/// (`body.json` matchers, CSV record wrapping key). Pulling in a full
/// JSONPath crate would add weight and expose features we then have to
/// teach users to avoid. The tiny implementation here fits the need.
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
        .fold(Some(value), |current, key| match current {
            Some(Value::Object(map)) => map.get(key),
            Some(Value::Array(arr)) => key.parse::<usize>().ok().and_then(|i| arr.get(i)),
            _ => None,
        })
}
