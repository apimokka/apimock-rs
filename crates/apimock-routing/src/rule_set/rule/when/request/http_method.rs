use hyper::Method;
use serde::Deserialize;

/// Every method a rule's `method = "..."` accepts, with the variant it
/// names. This is the one list config is checked against and the one
/// refusal messages are generated from, so the two cannot drift.
/// RFC 082 Amendment 1.
const MATCHABLE: [(&str, HttpMethod); 5] = [
    ("GET", HttpMethod::Get),
    ("POST", HttpMethod::Post),
    ("PUT", HttpMethod::Put),
    ("DELETE", HttpMethod::Delete),
    ("PATCH", HttpMethod::Patch),
];

/// Methods a user will plausibly try that are deliberately not matchable,
/// each with the reason its refusal quotes. The clause follows the method
/// name in the message. RFC 082 § 4 and Amendment 1.
const NOT_MATCHABLE: [(&str, &str); 4] = [
    (
        "OPTIONS",
        "is answered by the built-in CORS preflight handler before rule sets are consulted, so it cannot be matched by a rule",
    ),
    (
        "HEAD",
        "is not matchable yet: a rule could answer it with a response body, which HTTP forbids for HEAD",
    ),
    (
        "TRACE",
        "is not supported: TRACE is commonly disabled as a security measure and has no meaning for a mock server",
    ),
    (
        "CONNECT",
        "is not supported: CONNECT is a proxy mechanism with no meaning for a mock server",
    ),
];

/// An HTTP method a rule can match on.
///
/// `#[non_exhaustive]` (RFC 082): the next method added is not a breaking
/// change for a consumer's `match` — which must carry a `_` arm.
#[derive(Clone, Deserialize, Debug)]
#[serde(try_from = "String")]
#[non_exhaustive]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
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
    ///
    /// Deliberately an exhaustive `match`: a new variant that is not
    /// given a name here fails to compile, which is the first of the
    /// three things keeping config and the variant set in step (RFC 082
    /// Amendment 1).
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
        }
    }

    /// The method named by a config token, matched exactly and
    /// case-sensitively — the same spelling TOML config accepts. `None`
    /// for anything not in the matchable set, including every
    /// [`unmatchable_reason`](Self::unmatchable_reason) method.
    pub fn parse_config_token(token: &str) -> Option<HttpMethod> {
        MATCHABLE
            .iter()
            .find(|(name, _)| *name == token)
            .map(|(_, method)| method.clone())
    }

    /// The names a rule's `method` accepts, in the order refusal messages
    /// list them. RFC 082 Amendment 1.
    pub fn matchable_names() -> Vec<&'static str> {
        MATCHABLE.iter().map(|(name, _)| *name).collect()
    }

    /// Why a method a user might try is deliberately not matchable, as a
    /// clause that follows the method's name (`"is answered by …"`).
    /// `None` for a method that is not in the deliberate-exclusion list —
    /// including a plain typo. RFC 082 Amendment 1.
    pub fn unmatchable_reason(token: &str) -> Option<&'static str> {
        NOT_MATCHABLE
            .iter()
            .find(|(name, _)| *name == token)
            .map(|(_, reason)| *reason)
    }
}

impl TryFrom<String> for HttpMethod {
    type Error = String;

    /// The refusal a config load produces for a token that is not
    /// matchable. It always names the valid set. For a method we
    /// deliberately exclude it also says why, so a user cannot read the
    /// refusal as apimock being incomplete (RFC 082 Amendment 1).
    ///
    /// When the token is a matchable name or an excluded one in the wrong
    /// case, the line after the set names the correct spelling (task 018).
    /// An excluded method gets its reason instead of a casing hint:
    /// fixing the case would still leave it unmatchable, so a hint would
    /// send the user to a dead end. Both are looked up in the upper-cased
    /// token, so the name shown is always the canonical one.
    fn try_from(token: String) -> Result<Self, Self::Error> {
        if let Some(method) = HttpMethod::parse_config_token(&token) {
            return Ok(method);
        }
        let names = HttpMethod::matchable_names()
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let mut message = format!("unknown variant `{token}`, expected one of {names}");
        let upper = token.to_ascii_uppercase();
        if let Some(reason) = HttpMethod::unmatchable_reason(&upper) {
            message.push_str(&format!("\n  — {upper} {reason}"));
        } else if let Some(canonical) = HttpMethod::matchable_names()
            .into_iter()
            .find(|name| name.eq_ignore_ascii_case(&token))
        {
            message.push_str(&format!(
                "\n  — did you mean `{canonical}`? method values are upper-case"
            ));
        }
        Err(message)
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

    /// RFC 082: PATCH matches a PATCH wire method, and only that.
    #[test]
    fn patch_matches_patch_and_nothing_else() {
        assert!(HttpMethod::Patch.is_match(&Method::PATCH));
        assert!(!HttpMethod::Patch.is_match(&Method::GET));
        assert!(!HttpMethod::Get.is_match(&Method::PATCH));
    }

    /// RFC 082 § 4 decision, pinned: a lowercase method on the wire still
    /// matches — the wire is case-insensitive even though config is not.
    #[test]
    fn a_lowercase_patch_on_the_wire_matches() {
        let lowercase_patch = Method::from_bytes(b"patch").unwrap();
        assert!(HttpMethod::Patch.is_match(&lowercase_patch));
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

    /// Every matchable name parses to the variant it names, and that
    /// variant names itself back — the round-trip RFC 082 Amendment 1
    /// requires.
    #[test]
    fn every_matchable_name_round_trips() {
        for name in HttpMethod::matchable_names() {
            let method = HttpMethod::parse_config_token(name)
                .unwrap_or_else(|| panic!("{name} is listed as matchable but does not parse"));
            assert_eq!(method.as_str(), name);
        }
    }

    /// The residual gap Amendment 1 names: adding a variant and forgetting
    /// `MATCHABLE` would leave it silently unconfigurable. This exhaustive
    /// match forces the list to be checked whenever a variant is added.
    #[test]
    fn every_variant_is_matchable() {
        fn variant_name(m: &HttpMethod) -> &'static str {
            match m {
                HttpMethod::Get => "GET",
                HttpMethod::Post => "POST",
                HttpMethod::Put => "PUT",
                HttpMethod::Delete => "DELETE",
                HttpMethod::Patch => "PATCH",
            }
        }
        for method in [
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Delete,
            HttpMethod::Patch,
        ] {
            let name = variant_name(&method);
            assert!(
                HttpMethod::matchable_names().contains(&name),
                "{name} is a variant but not in MATCHABLE"
            );
        }
    }

    /// Case is part of config's spelling: `"patch"` is refused, exactly as
    /// `"get"` always was (RFC 082 § 4, pinned so it cannot drift silently).
    #[test]
    fn config_spelling_is_case_sensitive() {
        assert!(HttpMethod::parse_config_token("PATCH").is_some());
        assert!(HttpMethod::parse_config_token("patch").is_none());
        assert!(HttpMethod::try_from("patch".to_owned()).is_err());
    }

    /// Pull the backtick-quoted names out of a refusal's "expected one of"
    /// list, in order.
    fn enumerated_names(refusal: &str) -> Vec<String> {
        let after = refusal
            .split("expected one of ")
            .nth(1)
            .expect("refusal names the valid set");
        let list = after.split('\n').next().unwrap();
        list.split('`')
            .enumerate()
            .filter(|(i, _)| i % 2 == 1)
            .map(|(_, name)| name.to_owned())
            .collect()
    }

    /// RFC 082 Amendment 1, the drift test: the list a refusal enumerates
    /// is exactly `MATCHABLE`'s keys, in order.
    #[test]
    fn refusal_enumerates_exactly_the_matchable_names() {
        let refusal = HttpMethod::try_from("GTE".to_owned()).unwrap_err();
        let listed = enumerated_names(&refusal);
        let expected: Vec<String> = HttpMethod::matchable_names()
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(listed, expected);
    }

    /// A typo gets the enumerated set and no reason clause — unchanged in
    /// shape from before RFC 082, only the list grew.
    #[test]
    fn a_typo_is_refused_with_the_set_and_no_reason() {
        let refusal = HttpMethod::try_from("GTE".to_owned()).unwrap_err();
        assert_eq!(
            refusal,
            "unknown variant `GTE`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`"
        );
    }

    /// RFC 082 Amendment 1: OPTIONS is refused with both the enumerated
    /// set and the reason it is not matchable.
    #[test]
    fn options_is_refused_with_the_set_and_its_reason() {
        let refusal = HttpMethod::try_from("OPTIONS".to_owned()).unwrap_err();
        assert_eq!(
            refusal,
            "unknown variant `OPTIONS`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`\n  — OPTIONS is answered by the built-in CORS preflight handler before rule sets are consulted, so it cannot be matched by a rule"
        );
    }

    /// RFC 082 Amendment 1: HEAD is refused with both the set and its reason.
    #[test]
    fn head_is_refused_with_the_set_and_its_reason() {
        let refusal = HttpMethod::try_from("HEAD".to_owned()).unwrap_err();
        assert_eq!(
            refusal,
            "unknown variant `HEAD`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`\n  — HEAD is not matchable yet: a rule could answer it with a response body, which HTTP forbids for HEAD"
        );
    }

    /// Task 018: a matchable name in the wrong case is refused as before,
    /// with a hint naming the correct spelling after the set.
    #[test]
    fn lowercase_patch_gets_a_casing_hint() {
        let err = HttpMethod::try_from("patch".to_owned()).unwrap_err();
        assert_eq!(
            err,
            "unknown variant `patch`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`\n  — did you mean `PATCH`? method values are upper-case"
        );
    }

    /// Task 018: the hint names the canonical spelling, whatever the
    /// casing the user typed.
    #[test]
    fn mixed_case_patch_gets_the_same_casing_hint() {
        let err = HttpMethod::try_from("Patch".to_owned()).unwrap_err();
        assert!(err.ends_with("\n  — did you mean `PATCH`? method values are upper-case"));
    }

    /// Task 018: the hint is generated from `MATCHABLE`, so every matchable
    /// name in the wrong case gets a hint for itself, with no second list.
    #[test]
    fn every_matchable_name_in_the_wrong_case_is_hinted_with_itself() {
        for name in HttpMethod::matchable_names() {
            let wrong = name.to_ascii_lowercase();
            let err = HttpMethod::try_from(wrong.clone()).unwrap_err();
            assert!(
                err.ends_with(&format!(
                    "\n  — did you mean `{name}`? method values are upper-case"
                )),
                "{wrong} should be hinted with {name}, got: {err}"
            );
        }
    }

    /// Task 018, precedence: a deliberately excluded method in the wrong
    /// case gets its reason, not a casing hint. Fixing the case would
    /// still leave it unmatchable, so the hint would be a dead end.
    #[test]
    fn wrong_cased_exclusion_gets_the_reason_not_a_casing_hint() {
        let err = HttpMethod::try_from("options".to_owned()).unwrap_err();
        assert_eq!(
            err,
            "unknown variant `options`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`\n  — OPTIONS is answered by the built-in CORS preflight handler before rule sets are consulted, so it cannot be matched by a rule"
        );
        assert!(!err.contains("did you mean"));
    }

    /// Task 018: a plain typo gets neither the hint nor a reason, in the
    /// exact spelling or the wrong one.
    #[test]
    fn a_wrong_cased_typo_gets_no_hint() {
        for token in ["gte", "GTE"] {
            let err = HttpMethod::try_from(token.to_owned()).unwrap_err();
            assert_eq!(
                err,
                "unknown variant `".to_owned()
                    + token
                    + "`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`",
                "{token}"
            );
        }
    }

    /// Every deliberate exclusion has a reason, and no matchable method
    /// is also listed as excluded.
    #[test]
    fn exclusions_are_disjoint_from_the_matchable_set() {
        for (name, _) in NOT_MATCHABLE {
            assert!(
                HttpMethod::parse_config_token(name).is_none(),
                "{name} is both matchable and excluded"
            );
            assert!(HttpMethod::unmatchable_reason(name).is_some());
        }
    }
}
