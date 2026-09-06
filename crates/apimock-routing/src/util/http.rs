//! HTTP-path utilities used by the matcher.
//!
//! # Why only `normalize_url_path` (and now `percent_decode_url_path`)
//! # live here
//!
//! The full HTTP utility set (content-type inspection, response-delay
//! sleep) was originally grouped into a single `util::http` module when
//! the whole codebase was one crate. In the 5.0 split, the only helper
//! the *matcher* needs is URL-path normalization — content-type and
//! delay are server-side concerns, kept in `apimock-server::http_util`.

/// Percent-decode a raw request path (RFC 075 F-03).
///
/// # Why this is a separate step from `normalize_url_path`, not folded
/// # into it
///
/// `normalize_url_path` is also used to normalise an *operator-authored*
/// prefix (`rule_set.rs`, at config-load time) — plain text a person
/// typed, not URL-encoded client input. Decoding that too would be a
/// silent, surprising transformation of a config author's own string
/// (a literal `%2F` they meant literally would vanish). Scoping decoding
/// to this one function, called only on the incoming request path
/// (`parsed_request.rs`), keeps that risk out of the shared normaliser.
///
/// # Why this must run *before* `normalize_url_path`, never after
///
/// **This is the security-critical ordering in RFC 075.** Percent-
/// decoding after dot-segment normalisation would let `%2e%2e` survive
/// normalisation unrecognised, decode into `..` afterwards, and reach
/// path resolution unnormalised — reintroducing GHSA-72g6-wgrg-vhm7 by a
/// different route. The audit's own finding was that `%2e%2e` does not
/// traverse *today* for exactly one reason: decoding never happens at
/// all. Adding it without this ordering turns a missing feature into a
/// regression. Decode first, then normalise — the existing dot-segment
/// token-removal in `normalize_url_path` then strips a decoded `..`
/// exactly as it already strips a literal one. RFC 063's confinement
/// check downstream is the independent backstop and stays regardless;
/// this ordering is not a substitute for it.
///
/// # Why invalid UTF-8 after decoding is replaced, not rejected
///
/// A percent-escape can decode to a byte sequence that isn't valid
/// UTF-8 (e.g. a lone continuation byte). This project's request path is
/// `&str` throughout, so there is no lossless way to carry raw bytes
/// through the matcher; replacing the invalid sequence with U+FFFD
/// (`Cow`'s lossy decode) means such a path simply won't match any real
/// file or rule — a 404, the same fail-closed outcome as any other
/// unresolvable path — rather than needing a new error path for an
/// input this project has nowhere to route a byte-exact response to
/// anyway.
pub fn percent_decode_url_path(url_path: &str) -> std::borrow::Cow<'_, str> {
    percent_encoding::percent_decode_str(url_path).decode_utf8_lossy()
}

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
    use super::{normalize_url_path, percent_decode_url_path};

    #[test]
    fn ordinary_percent_escapes_decode() {
        assert_eq!(percent_decode_url_path("/my%20file.json"), "/my file.json");
        assert_eq!(percent_decode_url_path("/caf%C3%A9.json"), "/café.json");
    }

    #[test]
    fn a_plus_is_not_treated_as_a_space() {
        // Unlike query-string/form decoding, `+` in a *path* is not a
        // space substitute (RFC 3986 reserves that behaviour for
        // `application/x-www-form-urlencoded`, not paths) — confirming
        // `percent_encoding::percent_decode_str` (not
        // `form_urlencoded`) is genuinely what's in use here.
        assert_eq!(percent_decode_url_path("/a+b.json"), "/a+b.json");
    }

    #[test]
    fn an_encoded_dot_dot_decodes_to_a_literal_dot_dot() {
        // The decode step alone, in isolation — the security-critical
        // claim (that normalisation then strips it) is pinned
        // separately below, and end-to-end against a real server in
        // `crates/apimock/tests/server/response/confinement/dyn_route.rs`.
        assert_eq!(
            percent_decode_url_path("/%2e%2e/outside.txt"),
            "/../outside.txt"
        );
        assert_eq!(
            percent_decode_url_path("/%2E%2E/outside.txt"),
            "/../outside.txt"
        );
        assert_eq!(
            percent_decode_url_path("/%2e%2e%2foutside.txt"),
            "/../outside.txt"
        );
    }

    #[test]
    fn invalid_utf8_after_decoding_is_replaced_not_rejected() {
        // `%FF` alone is never valid UTF-8 (0xFF is not a valid UTF-8
        // lead byte) — decodes to U+FFFD rather than panicking or
        // returning an error, so the caller always gets a `str` back.
        let decoded = percent_decode_url_path("/%FF.json");
        assert!(decoded.contains('\u{FFFD}'), "decoded was: {decoded:?}");
    }

    #[test]
    fn an_incomplete_or_invalid_escape_passes_through_literally() {
        // Not exactly two hex digits after `%` — left alone, per
        // `percent_encoding`'s own documented, standards-compliant
        // behaviour, rather than this project inventing its own
        // leniency rule for malformed input.
        assert_eq!(percent_decode_url_path("/100%off"), "/100%off");
    }

    /// RFC 075 § 1's actual security requirement, exercised as the two
    /// steps are meant to run: decode, then normalise — proving the
    /// *ordering* this whole RFC exists to get right, not just that
    /// each step works in isolation.
    #[test]
    fn decoding_then_normalising_strips_an_encoded_dot_dot() {
        let decoded = percent_decode_url_path("/%2e%2e%2foutside.txt");
        assert_eq!(normalize_url_path(&decoded, None), "/outside.txt");
    }

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
