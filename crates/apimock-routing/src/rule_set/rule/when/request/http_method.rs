use hyper::Method;
use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    /// is match
    ///
    /// # RFC 077 P-07
    ///
    /// `eq_ignore_ascii_case` compares byte-by-byte with no allocation;
    /// the previous `.to_lowercase() == .to_lowercase()` allocated a new
    /// `String` on both sides of every comparison, on every request.
    /// Case-insensitivity itself is unchanged — `hyper::Method` doesn't
    /// normalise a wire method's case (`Method::from_bytes` preserves
    /// whatever the client sent for a non-canonical casing), so this
    /// still matches e.g. `get` against [`HttpMethod::Get`].
    pub fn is_match(&self, http_method: &Method) -> bool {
        self.as_str().eq_ignore_ascii_case(http_method.as_str())
    }

    /// as str
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }
}

impl std::fmt::Display for HttpMethod {
    /// RFC 079 M-09: this used to render `"HTTP Method is GET"` — a
    /// sentence, not a value, which produced nonsense wherever it was
    /// interpolated (`Request`'s own `Display` joins each present
    /// condition's rendering with `" && "`, so a rule condition summary
    /// would read `... && HTTP Method is GET && ...`). Renders like
    /// this module's sibling conditions instead — `url_path`'s own
    /// `Display` is `` url_path`{value}` ``; this is `` method`{value}` ``,
    /// the same shape with the TOML key name that identifies it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "method`{}`", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    //! RFC 077 P-07: pinned before the allocation-free rewrite so
    //! case-insensitivity survives it.
    use super::*;

    #[test]
    fn matches_the_canonical_uppercase_method() {
        assert!(HttpMethod::Get.is_match(&Method::GET));
        assert!(HttpMethod::Post.is_match(&Method::POST));
    }

    #[test]
    fn matches_a_lowercase_wire_method() {
        let lowercase_get = Method::from_bytes(b"get").unwrap();
        assert!(HttpMethod::Get.is_match(&lowercase_get));
    }

    #[test]
    fn matches_a_mixed_case_wire_method() {
        let mixed_case_delete = Method::from_bytes(b"DeLeTe").unwrap();
        assert!(HttpMethod::Delete.is_match(&mixed_case_delete));
    }

    #[test]
    fn does_not_match_a_different_method() {
        assert!(!HttpMethod::Get.is_match(&Method::POST));
        assert!(!HttpMethod::Put.is_match(&Method::DELETE));
    }

    /// RFC 079 M-09: renders the method value, not a sentence — pins
    /// the fix so `HTTP Method is GET` (nonsense once interpolated into
    /// `Request`'s own composed `Display`) can't come back.
    #[test]
    fn display_renders_the_method_value_not_a_sentence() {
        assert_eq!(format!("{}", HttpMethod::Get), "method`GET`");
        assert_eq!(format!("{}", HttpMethod::Post), "method`POST`");
        assert!(!format!("{}", HttpMethod::Delete).contains("HTTP Method is"));
    }
}
