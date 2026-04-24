use serde::Deserialize;

#[cfg(test)]
mod tests;

use crate::util::glob::glob_match;

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RuleOp {
    Equal,
    NotEqual,
    StartsWith,
    Contains,
    WildCard,
}

impl Default for RuleOp {
    fn default() -> Self {
        Self::Equal
    }
}

impl std::fmt::Display for RuleOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal => write!(f, " == "),
            Self::NotEqual => write!(f, " != "),
            Self::StartsWith => write!(f, " starts with "),
            Self::Contains => write!(f, " contains "),
            Self::WildCard => write!(f, " wild card matches "),
        }
    }
}

impl RuleOp {
    /// match with condition
    pub fn is_match(&self, text: &str, checker: &str) -> bool {
        match self {
            Self::Equal => text == checker,
            Self::NotEqual => text != checker,
            Self::StartsWith => text.starts_with(checker),
            Self::Contains => text.contains(checker),
            Self::WildCard => glob_match(checker, text),
        }
    }

    /// format condition params: key, op, value, and optional log_title
    pub fn format_condition(&self, key: &str, value: &str, log_title: Option<&str>) -> String {
        if log_title.is_some() {
            format!("[{}] {}{}{}", log_title.unwrap(), key, self, value)
        } else {
            format!("{}{}{}", key, self, value)
        }
    }
}
