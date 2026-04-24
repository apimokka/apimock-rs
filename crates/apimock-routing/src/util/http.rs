//! HTTP-path utilities used by the matcher.
//!
//! # Why only `normalize_url_path` lives here
//!
//! The full HTTP utility set (content-type inspection, response-delay
//! sleep) was originally grouped into a single `util::http` module when
//! the whole codebase was one crate. In the 5.0 split, the only helper
//! the *matcher* needs is URL-path normalization — content-type and
//! delay are server-side concerns, kept in `apimock-server::http_util`.

/// Normalize a URL path to one-leading-slash, no-trailing-slash form.
///
/// # Why we canonicalise here instead of at each call site
///
/// Rule-set authors write paths inconsistently — `/api/v1`, `api/v1`,
/// `/api/v1/` — and client requests arrive with similar variation.
/// Choosing one canonical form at the boundary means every matcher
/// downstream compares already-normalized strings, eliminating a class
/// of "why isn't my rule matching?" bugs.
pub fn normalize_url_path(url_path: &str, url_path_prefix: Option<&str>) -> String {
    let url_path_prefix = match url_path_prefix {
        Some(prefix) if !prefix.is_empty() => prefix.strip_suffix('/').unwrap_or(prefix),
        _ => "",
    };

    let url_path = url_path.strip_prefix('/').unwrap_or(url_path);

    let merged = format!("{}/{}", url_path_prefix, url_path);

    // Apply the two strips in the same order as the pre-5.0 implementation.
    // Using intermediate `&str` bindings (rather than chaining) keeps the
    // behaviour identical: each `strip_*` is independent of whether the
    // previous one matched.
    let trimmed = merged.strip_suffix('/').unwrap_or(merged.as_str());
    let trimmed = trimmed.strip_prefix('/').unwrap_or(trimmed);
    format!("/{}", trimmed)
}
