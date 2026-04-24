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

    /// validate (ok when deserialization is successful)
    pub fn validate(&self) -> bool {
        true
    }
}
