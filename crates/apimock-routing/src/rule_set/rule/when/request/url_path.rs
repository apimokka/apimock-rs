use serde::Deserialize;

use super::rule_op::RuleOp;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum UrlPathConfig {
    Simple(String),
    Detailed(UrlPath),
}

impl std::fmt::Display for UrlPathConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = match self {
            UrlPathConfig::Simple(s) => {
                write!(f, "url_path{}`{}`", RuleOp::default(), s)
            }
            UrlPathConfig::Detailed(url_path) => {
                write!(
                    f,
                    "url_path{}`{}`",
                    url_path.op.clone().unwrap_or_default(),
                    url_path.value,
                )
            }
        };
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UrlPath {
    pub value: String,
    #[serde(skip)]
    pub value_with_prefix: String,
    pub op: Option<RuleOp>,
}

impl UrlPath {
    /// check if `url_path` in `when` matches
    pub fn is_match(&self, parsed_request_url_path: &str) -> bool {
        let op = self.op.clone().unwrap_or_default();
        match op {
            // contains op works with raw value (aka without url_path prefix)
            RuleOp::Contains => op.is_match(parsed_request_url_path, self.value.as_str()),
            _ => op.is_match(parsed_request_url_path, self.value_with_prefix.as_str()),
        }
    }

    /// Intentionally trivial — always `true` (RFC 079 F-10/M-04e, found
    /// by the tranche 5 handoff's own refresh — the original RFC 079
    /// text named three of these four; this one was missed).
    ///
    /// `value` is a plain `String` (any value deserialises) and `op` is
    /// a `RuleOp` enum `serde` already restricts to a valid variant, so
    /// there is nothing left to check once deserialisation succeeds —
    /// which is what this method's original one-line comment already
    /// said, just without saying *why* that makes `true` the only
    /// answer. Kept (not removed), same reasoning as `RuleSet::validate`
    /// and `DefaultRespond::validate`'s own doc comments — called from
    /// `Request::validate` today, and the natural place a future
    /// constraint on `UrlPath` would grow into. See RFC 079 § 2 for the
    /// keep-and-document decision this reflects.
    pub fn validate(&self) -> bool {
        true
    }
}
