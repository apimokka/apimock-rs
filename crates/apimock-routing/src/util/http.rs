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

    // Defence in depth against a raw `..` segment (RFC 063): this is
    // not the confinement fix itself — the serve-time canonicalise
    // check is — but it closes the ordinary case before a path even
    // reaches file resolution. A bare token removal rather than a full
    // dot-segment resolution (RFC 3986 §5.2): `..` never counts as
    // legitimate in a rule's own `url_path` either, so dropping it
    // outright, without collapsing whatever segment preceded it, is
    // sufficient here and simpler than a real resolver.
    //
    // RFC 077 P-09: build the result directly instead of collecting an
    // intermediate `Vec<&str>`, `.join`-ing it, and `format!`-wrapping
    // that join — same segments, same order, one allocation instead of
    // three.
    let mut result = String::with_capacity(trimmed.len() + 1);
    result.push('/');
    let mut segments = trimmed.split('/').filter(|seg| *seg != "..");
    if let Some(first) = segments.next() {
        result.push_str(first);
        for seg in segments {
            result.push('/');
            result.push_str(seg);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::normalize_url_path;

    #[test]
    fn ordinary_paths_are_unaffected() {
        assert_eq!(normalize_url_path("/api/v1", None), "/api/v1");
        assert_eq!(normalize_url_path("api/v1", None), "/api/v1");
        assert_eq!(normalize_url_path("/api/v1/", None), "/api/v1");
    }

    #[test]
    fn a_leading_dot_dot_segment_is_stripped() {
        assert_eq!(normalize_url_path("/../outside.txt", None), "/outside.txt");
    }

    #[test]
    fn repeated_leading_dot_dot_segments_are_all_stripped() {
        assert_eq!(
            normalize_url_path("/../../outside.txt", None),
            "/outside.txt"
        );
    }

    #[test]
    fn a_mid_path_dot_dot_segment_is_stripped() {
        assert_eq!(normalize_url_path("/foo/../bar", None), "/foo/bar");
    }

    #[test]
    fn a_bare_dot_dot_normalises_to_root() {
        assert_eq!(normalize_url_path("/..", None), "/");
    }

    #[test]
    fn a_prefix_is_still_applied_alongside_stripping() {
        assert_eq!(
            normalize_url_path("/../outside.txt", Some("/api")),
            "/api/outside.txt"
        );
    }
}
