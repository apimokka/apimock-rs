use hyper::HeaderMap;
use tokio::task;

use std::{fs, path::Path};

use crate::{
    response::{
        error_response::{internal_server_error_response, not_found_response},
        file_response::FileResponse,
    },
    types::BoxBody,
};
use apimock_routing::util::json::JSON_COMPATIBLE_EXTENSIONS;

/// Serve a request from the fallback `respond_dir` (the file-based "just
/// drop JSON in a folder" mode).
///
/// # Why the matching is case-insensitive and extension-tolerant
///
/// This handler powers the zero-config experience where URL paths map
/// onto files on disk. The two accommodations we make are:
///
/// 1. **Case-insensitive filename match** — browsers often canonicalize
///    paths (`/Users` vs `/users`), and operators rarely care. If a file
///    matches in a case-insensitive compare we use it.
/// 2. **Extension inference** — a request to `/foo` with no extension
///    looks for `foo.json`, `foo.json5`, `foo.csv` in that order, then
///    `foo/index.*`. This means operators can drop a single JSON file
///    and use the shortened URL, which matches how most REST APIs are
///    described in docs.
pub async fn dyn_route_content(
    url_path: &str,
    fallback_respond_dir: &str,
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let request_path =
        Path::new(fallback_respond_dir).join(url_path.strip_prefix("/").unwrap_or_default());

    let request_file_name = request_path
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    // Locate the parent dir. Missing parent → 404. No parent at all
    // (e.g. path was empty) → 500, since that indicates a bug elsewhere.
    let Some(parent) = request_path.parent() else {
        return internal_server_error_response(
            &format!("parent dir not found: url_path = {}", url_path),
            request_headers,
        );
    };
    if !parent.exists() {
        return not_found_response(request_headers);
    }
    let dir = parent.to_owned();

    // Read the directory off the async runtime.
    //
    // # Why `spawn_blocking` and not `tokio::fs`
    //
    // `tokio::fs` is a thin async wrapper that internally calls
    // `spawn_blocking` itself. Using it directly would add a layer of
    // indirection while we iterate a `DirEntry` stream. Since we want
    // the whole directory listing at once (and it's bounded in size),
    // doing one `spawn_blocking` for the full listing is simpler and
    // uses the same thread pool underneath.
    let dir_for_blocking_task = dir.clone();
    let read_dir_result = task::spawn_blocking(move || -> Result<Vec<_>, String> {
        let entries = fs::read_dir(dir_for_blocking_task.as_path()).map_err(|err| {
            format!(
                "failed to get dir: {} ({})",
                dir_for_blocking_task.to_string_lossy(),
                err
            )
        })?;
        entries
            .map(|entry| {
                entry.map_err(|err| {
                    format!(
                        "failed to get dir entry from dir: {} ({})",
                        dir_for_blocking_task.to_string_lossy(),
                        err
                    )
                })
            })
            .collect()
    })
    .await;

    let entries = match read_dir_result {
        Ok(Ok(v)) => v,
        Ok(Err(err)) => {
            return internal_server_error_response(err.as_str(), request_headers);
        }
        Err(err) => {
            return internal_server_error_response(
                &format!("failed to get dir entries (async handling: {})", err),
                request_headers,
            );
        }
    };

    // Case-insensitive exact match within the directory listing.
    let mut found = entries.into_iter().find_map(|entry| {
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_owned();
        if name.eq_ignore_ascii_case(request_file_name) {
            Some(path)
        } else {
            None
        }
    });

    // Extension inference: `/foo` → `foo.json` / `foo.json5` / `foo.csv`.
    if found.is_none()
        && request_path.extension().is_none()
        && let Some(stem) = request_path.file_stem().and_then(|s| s.to_str())
    {
        for ext in JSON_COMPATIBLE_EXTENSIONS {
            let file_path = dir.join(format!("{}.{}", stem, ext));
            if file_path.exists() {
                found = Some(file_path);
                break;
            }
        }
    }

    let Some(found) = found else {
        return not_found_response(request_headers);
    };

    let file_path = found.to_str().unwrap_or_default();
    FileResponse::new(file_path, None, request_headers)
        .file_content_response()
        .await
}
