use hyper::HeaderMap;
use tokio::task;

use std::{
    fs,
    path::{Path, PathBuf},
};

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
/// 1. **Case-insensitive match, at every segment** (RFC 075 F-05) —
///    browsers often canonicalize paths (`/Users` vs `/users`), and
///    operators rarely care. apimock folds case itself rather than
///    delegating to the filesystem, because filesystem case behaviour
///    is not portable: Linux is case-sensitive, Windows and macOS
///    (APFS default) are not, so the same config and request used to
///    get different answers depending on which segment differed and
///    which platform served it. Uniform, apimock-enforced folding is
///    the only version of this that a committed rule set can rely on
///    identically everywhere.
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
    let base_dir = Path::new(fallback_respond_dir);
    let relative = url_path.strip_prefix('/').unwrap_or(url_path);
    let segments: Vec<&str> = relative.split('/').filter(|s| !s.is_empty()).collect();

    // A bare "/" (or an all-slashes path) has no segment of its own to
    // resolve — this is the pre-existing "index" mechanism, unchanged
    // by RFC 075: search for the fallback directory *itself*, by name,
    // in its own parent. That candidate is a directory, not a file, and
    // gets handed to `FileResponse` unfiltered exactly like any other
    // directory match below — `resolve_with_json_compatible_extensions`
    // there is what actually finds `index.json`/`index.json5`/
    // `index.html` inside it.
    //
    // `file_name()` is `None` when `fallback_respond_dir` is exactly
    // `.` (the default) — `.` is a special "current directory" path
    // component, not a named one. Falling back to `""` reproduces the
    // pre-RFC-075 code's own behaviour there exactly (it used
    // `.unwrap_or_default()` at this same spot): the search degenerates
    // to an empty name that can never match a real directory entry, so
    // it falls straight through to 404 rather than 500 — a case-free
    // config still 404s correctly, it just doesn't have an "own name"
    // to look for.
    let (file_name, parent_segments): (&str, &[&str]) = match segments.split_last() {
        Some((&last, rest)) => (last, rest),
        None => (
            base_dir.file_name().and_then(|s| s.to_str()).unwrap_or(""),
            &[],
        ),
    };
    let search_base_dir = if segments.is_empty() {
        // Search in the fallback directory's own parent, not inside
        // itself — see the comment above.
        let Some(parent) = base_dir.parent() else {
            return internal_server_error_response(
                &format!("parent dir not found: url_path = {}", url_path),
                request_headers,
                cors_allow_credentials_origins,
            );
        };
        parent
    } else {
        base_dir
    };

    // RFC 077 P-06's fast path, preserved: try the whole remaining path
    // as it was literally requested, one `stat` (plus bounded extension
    // inference) against the *naive*, not-yet-case-corrected parent
    // directory. On a case-insensitive filesystem this already resolves
    // every segment correctly — the OS folds case for every segment at
    // once, for free — so the common case still costs nothing beyond
    // what P-06 already established. Falling through to the per-segment
    // walk below is what makes Linux (case-sensitive) match what the
    // other two platforms already gave for free here (RFC 075 F-05).
    let mut naive_parent_dir = search_base_dir.to_owned();
    for &segment in parent_segments {
        naive_parent_dir.push(segment);
    }
    match resolve_final_segment(&naive_parent_dir, file_name).await {
        Ok(Some(found)) => {
            return serve_found(
                found,
                request_headers,
                confine_to,
                cors_allow_credentials_origins,
            )
            .await;
        }
        Ok(None) => {}
        Err(err) => {
            return report_listing_failure(err, request_headers, cors_allow_credentials_origins);
        }
    }

    if parent_segments.is_empty() {
        // No intermediate segment exists to have a case mismatch — the
        // fast path above already checked the only possible location.
        return not_found_response(request_headers, cors_allow_credentials_origins);
    }

    // RFC 075 F-05: the fast path missed. Walk every intermediate
    // segment through apimock's own case-insensitive comparison instead
    // of the filesystem's, so Linux resolves a case-mismatched
    // directory the same way macOS/Windows already did above.
    let mut resolved_parent_dir = search_base_dir.to_owned();
    for &segment in parent_segments {
        match resolve_dir_segment(&resolved_parent_dir, segment).await {
            Ok(Some(next)) => resolved_parent_dir = next,
            Ok(None) => return not_found_response(request_headers, cors_allow_credentials_origins),
            Err(err) => {
                return report_listing_failure(
                    err,
                    request_headers,
                    cors_allow_credentials_origins,
                );
            }
        }
    }

    match resolve_final_segment(&resolved_parent_dir, file_name).await {
        Ok(Some(found)) => {
            serve_found(
                found,
                request_headers,
                confine_to,
                cors_allow_credentials_origins,
            )
            .await
        }
        Ok(None) => not_found_response(request_headers, cors_allow_credentials_origins),
        Err(err) => report_listing_failure(err, request_headers, cors_allow_credentials_origins),
    }
}

/// Build the response for a resolved file — the one place `dyn_route_content`
/// actually reads and serves content, shared by both the fast path and
/// the per-segment-resolved fallback.
async fn serve_found(
    found: PathBuf,
    request_headers: &HeaderMap,
    confine_to: Option<&Path>,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
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

/// A directory listing failed to read (as opposed to simply not
/// containing a match) — both `resolve_final_segment` and
/// `resolve_dir_segment`'s error carries `dir`'s own filesystem path
/// (RFC 065 D4): logged in full, server-side; the client only ever sees
/// a generic message naming the problem, not the server's directory
/// layout.
fn report_listing_failure(
    err: String,
    request_headers: &HeaderMap,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    log::error!("{}", err);
    internal_server_error_response(
        "failed to read fallback directory",
        request_headers,
        cors_allow_credentials_origins,
    )
}

/// Resolve the final path segment (`file_name`) inside `dir`, using the
/// same three-tier strategy regardless of whether `dir` is the naive
/// (requested-case) parent or one already resolved case-insensitively:
/// exact stat, extension inference (only when `file_name` has no
/// extension), then a case-insensitive directory listing as the last
/// resort.
///
/// # The listing tier's result is *not* filtered to files
///
/// A resolved candidate here can be a directory — deliberately: this is
/// the pre-existing mechanism (unchanged by RFC 075) behind both `/`
/// resolving to `index.json` and `/subdir` resolving to
/// `subdir/index.json5`. `dyn_route_content` hands whatever this
/// returns straight to `FileResponse`, whose own
/// `resolve_with_json_compatible_extensions` recognises a directory
/// candidate and looks for `index.*` inside it. Filtering the listing
/// tier to files-only here would silently break both of those.
async fn resolve_final_segment(dir: &Path, file_name: &str) -> Result<Option<PathBuf>, String> {
    let candidate = dir.join(file_name);
    if is_existing_file(&candidate).await {
        return Ok(Some(candidate));
    }

    // Extension inference: `/foo` → `foo.json` / `foo.json5` / `foo.csv`.
    // This is the shape the README's zero-config pitch relies on
    // (`/hello` -> `hello.json`), and it's always been a direct stat per
    // candidate, independent of the listing below.
    let inferred_names: Vec<String> = if Path::new(file_name).extension().is_none()
        && let Some(stem) = Path::new(file_name).file_stem().and_then(|s| s.to_str())
    {
        for ext in JSON_COMPATIBLE_EXTENSIONS {
            let candidate = dir.join(format!("{}.{}", stem, ext));
            if is_existing_file(&candidate).await {
                return Ok(Some(candidate));
            }
        }
        JSON_COMPATIBLE_EXTENSIONS
            .iter()
            .map(|ext| format!("{}.{}", stem, ext))
            .collect()
    } else {
        Vec::new()
    };

    if !dir.exists() {
        return Ok(None);
    }

    // RFC 075 F-05: case-folding and extension inference must combine,
    // not just each work alone — a request for `/CaseDir/file` must
    // still resolve `File.json` even though neither the exact-stat tier
    // above (case matched, no extension) nor the extension-inference
    // tier above (extension matched, case didn't) can find it alone.
    // Try the bare name first, then each extension-inferred name, all
    // case-insensitively, in the same priority order the exact-stat
    // tiers above already used.
    let mut candidate_names = Vec::with_capacity(1 + inferred_names.len());
    candidate_names.push(file_name.to_owned());
    candidate_names.extend(inferred_names);
    find_by_case_insensitive_listing(dir, &candidate_names).await
}

/// Resolve one intermediate directory segment inside `dir`: exact stat
/// (as a directory), then a case-insensitive listing match that is also
/// a directory. No extension inference — that's a final-segment concept
/// only.
async fn resolve_dir_segment(dir: &Path, segment: &str) -> Result<Option<PathBuf>, String> {
    let candidate = dir.join(segment);
    if is_existing_dir(&candidate).await {
        return Ok(Some(candidate));
    }

    if !dir.exists() {
        return Ok(None);
    }
    match find_by_case_insensitive_listing(dir, &[segment.to_owned()]).await? {
        Some(found) if is_existing_dir(&found).await => Ok(Some(found)),
        _ => Ok(None),
    }
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

/// `true` iff `path` exists and is a directory — `is_existing_file`'s
/// counterpart, used to resolve intermediate path segments (RFC 075
/// F-05) rather than the final one.
async fn is_existing_dir(path: &Path) -> bool {
    let path = path.to_owned();
    task::spawn_blocking(move || path.is_dir())
        .await
        .unwrap_or(false)
}

/// The last resort in `resolve_final_segment` and `resolve_dir_segment`:
/// list `dir` and find `name` case-insensitively. Only reached when an
/// exact-path stat (and, for a final segment, extension inference)
/// didn't resolve the request.
///
/// # Unicode-aware, not ASCII-only (RFC 075 § 2a)
///
/// Tranche 3's fast path (`is_existing_file`/`is_existing_dir` against
/// the literal requested path) delegates to the OS's own `stat`, which
/// on a case-insensitive filesystem folds *Unicode* case (a request for
/// `CAFÉ.json` resolves `café.json` there for free). If this listing
/// compared ASCII-only (`eq_ignore_ascii_case`), Linux would refuse the
/// exact same request the fast path already accepts on macOS/Windows —
/// reintroducing F-05's own defect for non-ASCII names specifically,
/// through the fix meant to remove it. Comparing with `to_lowercase()`
/// instead means this listing accepts everything the fast path's OS
/// delegation already does, so the two can't disagree with each other:
/// whichever one runs for a given request answers the same way the
/// other would have. Full Unicode case-folding equivalence with any
/// specific OS's own tables is not claimed or needed here — only that
/// this project's two paths never contradict each other; the same
/// class of limitation this project's own docs already carry for
/// Unicode *normalisation* (RFC 075 § 4's NFC/NFD scope-out), just
/// applied to folding instead of encoding.
///
/// # Why `spawn_blocking` and not `tokio::fs`
///
/// `tokio::fs` is a thin async wrapper that internally calls
/// `spawn_blocking` itself. Using it directly would add a layer of
/// indirection while we iterate a `DirEntry` stream, so one
/// `spawn_blocking` for the whole scan is simpler and uses the same
/// thread pool underneath.
/// `candidate_names` is checked in order — the first name (not the
/// first directory entry) with a case-insensitive match wins, so a
/// caller combining a bare name with extension-inferred variants (see
/// `resolve_final_segment`) gets the same priority the exact-stat tiers
/// ahead of this one already use, regardless of the OS's own (arbitrary)
/// listing order.
async fn find_by_case_insensitive_listing(
    dir: &Path,
    candidate_names: &[String],
) -> Result<Option<std::path::PathBuf>, String> {
    let dir_for_blocking_task = dir.to_owned();
    let candidate_names_lower: Vec<String> =
        candidate_names.iter().map(|n| n.to_lowercase()).collect();
    let read_dir_result =
        task::spawn_blocking(move || -> Result<Option<std::path::PathBuf>, String> {
            let entries = fs::read_dir(dir_for_blocking_task.as_path())
                .map_err(|err| {
                    format!(
                        "failed to get dir: {} ({})",
                        dir_for_blocking_task.to_string_lossy(),
                        err
                    )
                })?
                .map(|entry| {
                    let entry = entry.map_err(|err| {
                        format!(
                            "failed to get dir entry from dir: {} ({})",
                            dir_for_blocking_task.to_string_lossy(),
                            err
                        )
                    })?;
                    let path = entry.path();
                    let name_lower = path
                        .file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_lowercase();
                    Ok((name_lower, path))
                })
                .collect::<Result<Vec<(String, PathBuf)>, String>>()?;

            for candidate_lower in &candidate_names_lower {
                if let Some((_, path)) = entries.iter().find(|(name, _)| name == candidate_lower) {
                    return Ok(Some(path.clone()));
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
    /// what platform it ran on. CI's three-OS matrix passing on Linux,
    /// macOS, and Windows is consistent with both branches occurring
    /// (the latter two default to case-insensitive filesystems) — the
    /// test itself, passing either way, doesn't print or otherwise
    /// prove which branch a given runner actually took.
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
