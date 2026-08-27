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
    confine_to: Option<&Path>,
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
        // Both `err` variants below carry `dir`'s own filesystem path
        // (RFC 065 D4) — logged in full, server-side; the client only
        // ever sees a generic message naming the problem, not the
        // server's directory layout.
        Ok(Err(err)) => {
            log::error!("{}", err);
            return internal_server_error_response(
                "failed to read fallback directory",
                request_headers,
            );
        }
        Err(err) => {
            log::error!(
                "failed to get dir entries ({}): {}",
                dir.to_string_lossy(),
                err
            );
            return internal_server_error_response(
                "failed to read fallback directory",
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
    FileResponse::new(file_path, None, request_headers, confine_to)
        .file_content_response()
        .await
}

#[cfg(test)]
mod tests {
    use hyper::HeaderMap;

    use crate::response::confine::canonical_dir;

    use super::dyn_route_content;

    /// RFC 063: this calls `dyn_route_content` directly with a `url_path`
    /// that still carries `..` — bypassing `normalize_url_path`, the
    /// other, independent defence (applied earlier, at request-parse
    /// time). Confirms confinement alone, without that first layer,
    /// still refuses the escape.
    #[tokio::test]
    async fn a_raw_dot_dot_is_refused_even_without_url_normalisation_first() {
        let outer = tempfile::tempdir().unwrap();
        let respond_dir = outer.path().join("respond_dir");
        std::fs::create_dir(&respond_dir).unwrap();
        std::fs::write(outer.path().join("outside.txt"), "SECRET-OUTSIDE-CONTENT").unwrap();

        let confine_to = canonical_dir(respond_dir.to_str().unwrap());
        assert!(confine_to.is_some(), "fixture directory must canonicalise");

        let response = dyn_route_content(
            "/../outside.txt",
            respond_dir.to_str().unwrap(),
            &HeaderMap::new(),
            confine_to.as_deref(),
        )
        .await
        .expect("dyn_route_content must not fail to build a response");

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
    }

    /// `confine_to: None` means "the base directory couldn't be
    /// canonicalised at load time" (e.g. it doesn't exist), which must
    /// fail closed — refuse everything — rather than skip the check.
    /// Distinct from the pre-fix behaviour, which had no `confine_to`
    /// parameter to be `None` in the first place; that comparison is
    /// made by re-running this file's tests against the pre-fix source
    /// (reported alongside this evidence), not by this test.
    #[tokio::test]
    async fn a_base_that_failed_to_canonicalise_refuses_every_candidate() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.json"), "{}").unwrap();

        let response = dyn_route_content(
            "/hello.json",
            dir.path().to_str().unwrap(),
            &HeaderMap::new(),
            None,
        )
        .await
        .expect("dyn_route_content must not fail to build a response");

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
    }
}
