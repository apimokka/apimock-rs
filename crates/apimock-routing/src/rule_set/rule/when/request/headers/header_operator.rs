use serde::Deserialize;

use crate::rule_set::rule::when::request::rule_op::RuleOp;

/// Operator for a header match condition (RFC 017).
///
/// A flat enum — mirrors the shape of [`crate::rule_set::rule::when::request::body::body_operator::BodyOperator`]
/// rather than wrapping [`RuleOp`]. Each operator serialises directly to
/// its `snake_case` name in TOML (`equal`, `exists`, `absent`, …).
///
/// # Operator groups
///
/// | Group     | Variants                                                   |
/// |-----------|------------------------------------------------------------|
/// | value     | `equal`, `not_equal`, `starts_with`, `ends_with`, `contains`, `wild_card`, `regex` |
/// | presence  | `exists`, `absent`                                         |
///
/// Presence operators assert whether the header key is present in the
/// request, regardless of its value. Value operators compare the header's
/// value using the matching semantics of the corresponding [`RuleOp`]
/// variant.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeaderOperator {
    // ── value operators ───────────────────────────────────────────────
    Equal,
    NotEqual,
    StartsWith,
    EndsWith,
    Contains,
    WildCard,
    /// Regex match. Pattern compiled per request; cache if performance
    /// becomes a concern.
    Regex,
    // ── presence operators ────────────────────────────────────────────
    /// Header key must be present in the request (any value).
    Exists,
    /// Header key must be absent from the request.
    Absent,
}

impl Default for HeaderOperator {
    fn default() -> Self {
        Self::Equal
    }
}

impl std::fmt::Display for HeaderOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal      => write!(f, " == "),
            Self::NotEqual   => write!(f, " != "),
            Self::StartsWith => write!(f, " starts with "),
            Self::EndsWith   => write!(f, " ends with "),
            Self::Contains   => write!(f, " contains "),
            Self::WildCard   => write!(f, " wild card matches "),
            Self::Regex      => write!(f, " matches regex "),
            Self::Exists     => write!(f, " exists"),
            Self::Absent     => write!(f, " absent"),
        }
    }
}

impl HeaderOperator {
    /// Return the `snake_case` name used in TOML and view strings.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Equal      => "equal",
            Self::NotEqual   => "not_equal",
            Self::StartsWith => "starts_with",
            Self::EndsWith   => "ends_with",
            Self::Contains   => "contains",
            Self::WildCard   => "wild_card",
            Self::Regex      => "regex",
            Self::Exists     => "exists",
            Self::Absent     => "absent",
        }
    }

    /// Convert a value-comparison variant to its [`RuleOp`] equivalent.
    ///
    /// # Panics
    ///
    /// Panics if called on a presence operator (`Exists` / `Absent`).
    /// Callers must branch on presence operators before calling this.
    pub fn to_rule_op(&self) -> RuleOp {
        match self {
            Self::Equal      => RuleOp::Equal,
            Self::NotEqual   => RuleOp::NotEqual,
            Self::StartsWith => RuleOp::StartsWith,
            Self::EndsWith   => RuleOp::EndsWith,
            Self::Contains   => RuleOp::Contains,
            Self::WildCard   => RuleOp::WildCard,
            Self::Regex      => RuleOp::Regex,
            Self::Exists | Self::Absent => {
                panic!("HeaderOperator::to_rule_op called on presence operator {:?}", self)
            }
        }
    }
}
