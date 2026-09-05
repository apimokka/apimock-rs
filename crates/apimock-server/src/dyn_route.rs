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
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let request_path =
        Path::new(fallback_respond_dir).join(url_path.strip_prefix("/").unwrap_or_default());

    let request_file_name = request_path
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    // Locate the parent dir. No parent at all (e.g. path was empty) →
    // 500, since that indicates a bug elsewhere. Missing-parent-as-404
    // is checked below, only if the fast paths below don't resolve the
    // request — no need to stat it twice.
    let Some(parent) = request_path.parent() else {
        return internal_server_error_response(
            &format!("parent dir not found: url_path = {}", url_path),
            request_headers,
            cors_allow_credentials_origins,
        );
    };
    let dir = parent.to_owned();

    // RFC 077 P-06: resolve with bounded `stat`s before ever listing the
    // directory. Before this fix, the whole directory was listed on
    // every request — even the overwhelmingly common ones below, which
    // never needed to see another file's name.
    //
    // # Deliberate precedence note
    //
    // The previous single-pass order tried a case-insensitive listing
    // match before extension inference. This tries an exact-path stat,
    // then extension inference, before falling back to the listing
    // below. The two orders can only disagree when a directory holds
    // both a bare, differently-cased file (e.g. `FOO`) *and* an
    // extension match (`foo.json`) for the same request — a
    // configuration nothing in this codebase's tests exercises. Plain
    // case-insensitive matching is unchanged and still runs, below, for
    // whatever neither stat resolves.
    let mut found = if is_existing_file(&request_path).await {
        Some(request_path.clone())
    } else {
        None
    };

    // Extension inference: `/foo` → `foo.json` / `foo.json5` / `foo.csv`.
    // This is the shape the README's zero-config pitch relies on
    // (`/hello` -> `hello.json`), and it was always a direct stat per
    // candidate — it just never ran until after the listing above it.
    if found.is_none()
        && request_path.extension().is_none()
        && let Some(stem) = request_path.file_stem().and_then(|s| s.to_str())
    {
        for ext in JSON_COMPATIBLE_EXTENSIONS {
            let candidate = dir.join(format!("{}.{}", stem, ext));
            if is_existing_file(&candidate).await {
                found = Some(candidate);
                break;
            }
        }
    }

    // Last resort: list the directory for a case-insensitive name match
    // (e.g. a client-side path that canonicalised differently than the
    // filesystem did). Only reached when neither stat above resolved
    // the request.
    if found.is_none() {
        if !dir.exists() {
            return not_found_response(request_headers, cors_allow_credentials_origins);
        }

        found = match find_by_case_insensitive_listing(&dir, request_file_name).await {
            Ok(found) => found,
            // Both `err` sources carry `dir`'s own filesystem path
            // (RFC 065 D4) — logged in full, server-side; the client
            // only ever sees a generic message naming the problem, not
            // the server's directory layout.
            Err(err) => {
                log::error!("{}", err);
                return internal_server_error_response(
                    "failed to read fallback directory",
                    request_headers,
                    cors_allow_credentials_origins,
                );
            }
        };
    }

    let Some(found) = found else {
        return not_found_response(request_headers, cors_allow_credentials_origins);
    };

    let file_path = found.to_str().unwrap_or_default();
    FileResponse::new(
        file_path,
        None,
        request_headers,
        confine_to,
        cors_allow_credentials_origins,
    )
    .file_content_response()
    .await
}

/// `true` iff `path` exists and is a regular file — a single `stat`,
/// off the async runtime like the directory listing it lets most
/// requests skip (see `dyn_route_content`).
async fn is_existing_file(path: &Path) -> bool {
    let path = path.to_owned();
    task::spawn_blocking(move || path.is_file())
        .await
        .unwrap_or(false)
}

/// The last resort in `dyn_route_content`: list `dir` and find
/// `request_file_name` case-insensitively. Only reached when neither an
/// exact-path stat nor extension inference resolved the request.
///
/// # Why `spawn_blocking` and not `tokio::fs`
///
/// `tokio::fs` is a thin async wrapper that internally calls
/// `spawn_blocking` itself. Using it directly would add a layer of
/// indirection while we iterate a `DirEntry` stream, so one
/// `spawn_blocking` for the whole scan is simpler and uses the same
/// thread pool underneath.
async fn find_by_case_insensitive_listing(
    dir: &Path,
    request_file_name: &str,
) -> Result<Option<std::path::PathBuf>, String> {
    let dir_for_blocking_task = dir.to_owned();
    let request_file_name = request_file_name.to_owned();
    let read_dir_result =
        task::spawn_blocking(move || -> Result<Option<std::path::PathBuf>, String> {
            let entries = fs::read_dir(dir_for_blocking_task.as_path()).map_err(|err| {
                format!(
                    "failed to get dir: {} ({})",
                    dir_for_blocking_task.to_string_lossy(),
                    err
                )
            })?;
            for entry in entries {
                let entry = entry.map_err(|err| {
                    format!(
                        "failed to get dir entry from dir: {} ({})",
                        dir_for_blocking_task.to_string_lossy(),
                        err
                    )
                })?;
                let path = entry.path();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default();
                if name.eq_ignore_ascii_case(&request_file_name) {
                    return Ok(Some(path));
                }
            }
            Ok(None)
        })
        .await;

    match read_dir_result {
        Ok(Ok(found)) => Ok(found),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(format!(
            "failed to get dir entries ({}): {}",
            dir.to_string_lossy(),
            err
        )),
    }
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
            &[],
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
            &[],
        )
        .await
        .expect("dyn_route_content must not fail to build a response");

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
    }

    /// Read a response body to bytes, for tests that need to confirm
    /// *which* file was served, not only that something 200'd.
    async fn body_bytes(response: hyper::Response<crate::types::BoxBody>) -> Vec<u8> {
        http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes()
            .to_vec()
    }

    /// RFC 077 P-06: the common case — request path matches a file's
    /// name and case exactly — resolves via the fast stat path, never
    /// touching the case-insensitive listing fallback.
    #[tokio::test]
    async fn exact_case_match_resolves_without_listing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.json"), "{\"ok\":true}").unwrap();
        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let response = dyn_route_content(
            "/hello.json",
            dir.path().to_str().unwrap(),
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        )
        .await
        .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        assert_eq!(body_bytes(response).await, b"{\"ok\":true}");
    }

    /// RFC 077 P-06: extension inference (`/hello` -> `hello.json`) — the
    /// README's zero-config shape — still resolves, now via a direct
    /// stat rather than a directory listing.
    #[tokio::test]
    async fn extension_inference_still_resolves() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.json"), "{\"ok\":true}").unwrap();
        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let response = dyn_route_content(
            "/hello",
            dir.path().to_str().unwrap(),
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        )
        .await
        .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        assert_eq!(body_bytes(response).await, b"{\"ok\":true}");
    }

    /// RFC 077 P-06: a case mismatch (`/Hello.JSON` against a file
    /// literally named `hello.json`) still resolves — the
    /// case-insensitive listing fallback is unchanged, only deferred
    /// until the cheap stats above it miss.
    #[tokio::test]
    async fn case_mismatch_still_resolves_via_the_listing_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.json"), "{\"ok\":true}").unwrap();
        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let response = dyn_route_content(
            "/Hello.JSON",
            dir.path().to_str().unwrap(),
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        )
        .await
        .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        assert_eq!(body_bytes(response).await, b"{\"ok\":true}");
    }

    /// REVIEW-001 F-01: the disclosed P-06 precedence change (exact-path
    /// stat + extension inference now run before the case-insensitive
    /// listing) is platform-dependent, not universal — a directory
    /// holding both a bare, differently-cased file (`FOO`) and an
    /// extension match (`foo.json`) for the same extension-less request
    /// (`/foo`) resolves differently depending on whether the
    /// filesystem's own `stat` is case-sensitive:
    ///
    /// - Case-sensitive (Linux, the outlier): the exact-path stat for
    ///   the literal `foo` misses `FOO` entirely, so extension inference
    ///   (which now runs first) finds `foo.json` — the disclosed change.
    /// - Case-insensitive (macOS APFS default, Windows NTFS default):
    ///   the exact-path stat for `foo` *is* `FOO` at the OS level, so the
    ///   bare file wins here too, same as before this fix — no change.
    ///
    /// Detected at runtime against this test's own directory (not
    /// assumed from `cfg!(target_os)`, which can be wrong — a
    /// case-sensitive APFS volume or a case-insensitive Linux mount both
    /// exist) so this test states what actually happened rather than
    /// what platform it ran on, and CI's three-OS matrix is what
    /// confirms both branches actually occur.
    #[tokio::test]
    async fn bare_differently_cased_file_vs_extension_match_resolves_per_filesystem_case_sensitivity()
     {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("FOO"), "BARE-FILE-CONTENT").unwrap();
        std::fs::write(dir.path().join("foo.json"), "{\"extension\":\"inferred\"}").unwrap();

        let filesystem_is_case_insensitive = dir.path().join("foo").is_file();

        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let response = dyn_route_content(
            "/foo",
            dir.path().to_str().unwrap(),
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        )
        .await
        .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = body_bytes(response).await;

        if filesystem_is_case_insensitive {
            assert_eq!(
                body, b"BARE-FILE-CONTENT",
                "case-insensitive filesystem: the exact-path stat for \
                 \"foo\" already resolves to \"FOO\" at the OS level, so \
                 the bare file should still win, unchanged from before \
                 this fix"
            );
        } else {
            assert_eq!(
                body, b"{\"extension\":\"inferred\"}",
                "case-sensitive filesystem: the exact-path stat for \
                 \"foo\" misses \"FOO\", so extension inference (which \
                 now runs before the listing) should resolve to \
                 \"foo.json\" — the disclosed precedence change"
            );
        }
    }

    /// A directory with no matching file at all — every path (exact
    /// stat, extension inference, listing) misses — still 404s rather
    /// than erroring.
    #[tokio::test]
    async fn no_match_anywhere_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("other.json"), "{}").unwrap();
        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let response = dyn_route_content(
            "/does-not-exist",
            dir.path().to_str().unwrap(),
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        )
        .await
        .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
    }
}
