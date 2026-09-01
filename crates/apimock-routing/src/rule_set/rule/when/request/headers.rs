use console::style;
use hyper::{HeaderMap, header::HeaderValue};
use serde::Deserialize;

use indexmap::IndexMap;

pub mod header_operator;

use header_operator::HeaderOperator;

use super::util::fmt_condition_connector;
use crate::rule_set::rule::ConditionKey;

/// One header condition (operator + expected value) stored inside [`Headers`].
///
/// Replaces the former `ConditionStatement` (which carried a [`super::rule_op::RuleOp`])
/// with a type that can represent presence operators as well as value operators.
/// Mirrors the shape of `BodyConditionStatement` in the body module (RFC 017).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderConditionStatement {
    #[serde(default)]
    pub op: Option<HeaderOperator>,
    #[serde(default)]
    pub value: String,
}

impl std::fmt::Display for HeaderConditionStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = self.op.clone().unwrap_or_default();
        match op {
            HeaderOperator::Exists | HeaderOperator::Absent => write!(f, "{}", op),
            _ => write!(f, "{}`{}`", op, self.value),
        }
    }
}

/// Header match conditions keyed by header name (lower-cased).
///
/// Uses [`IndexMap`] instead of `HashMap` to preserve the order in which
/// conditions were written in the TOML rule file. This means
/// [`crate::view::WhenView::headers`] arrives in authoring order rather
/// than arbitrary hash order (RFC 014).
#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct Headers(pub IndexMap<ConditionKey, HeaderConditionStatement>);

impl Headers {
    /// Returns `true` iff the request's headers satisfy all conditions.
    ///
    /// Conditions are ANDed: every key-condition pair in `self.0` must match.
    /// Presence operators (`Exists`, `Absent`) only check key presence;
    /// value operators compare the request header's value string.
    ///
    /// # A header that cannot be read as UTF-8 (RFC 072)
    ///
    /// `Exists`/`Absent` are unaffected — they check `contains_key` before
    /// ever attempting to decode the value, so a present-but-undecodable
    /// header still correctly satisfies `Exists` and fails `Absent`: the
    /// header genuinely *is* present, which is all those two operators
    /// ask. "cannot be read" and "not present" are different things, and
    /// this is the deliberate, tested answer to which one a decode
    /// failure counts as — not present is reserved for a header that
    /// really is absent.
    ///
    /// A **value** operator (`equal`, `contains`, …) against an
    /// undecodable header does **not** satisfy the condition, regardless
    /// of which operator it is — a gate that cannot evaluate its input
    /// fails closed rather than silently opening (the previous
    /// behaviour: `return true` here, matching *any* condition on a
    /// header sent as invalid UTF-8, unconditionally). See
    /// `crates/apimock/src/cmd/rule_check.rs`'s `check_headers`, which
    /// this must agree with — see that agreement test.
    pub fn is_match(
        &self,
        parsed_request_headers: &HeaderMap<HeaderValue>,
        rule_idx: usize,
        rule_set_idx: usize,
    ) -> bool {
        self.0
            .iter()
            .all(|(matcher_key, stmt)| {
                let op = stmt.op.clone().unwrap_or_default();

                // ── presence operators ────────────────────────────────
                match op {
                    HeaderOperator::Exists => {
                        return parsed_request_headers.contains_key(matcher_key.as_str())
                    }
                    HeaderOperator::Absent => {
                        return !parsed_request_headers.contains_key(matcher_key.as_str())
                    }
                    _ => {}
                }

                // ── value operators ────────────────────────────────────
                let header_value = match parsed_request_headers.get(matcher_key.as_str()) {
                    Some(v) => v,
                    None => return false,
                };

                let header_str = match header_value.to_str() {
                    Ok(s) => s,
                    Err(err) => {
                        log::error!(
                            "{} to get request header value by key (rule #{} in rule set #{}):\n`{}`\n({})",
                            style("failed").red(),
                            rule_idx + 1,
                            rule_set_idx + 1,
                            matcher_key,
                            err,
                        );
                        return false;
                    }
                };

                op.to_rule_op().is_match(header_str, &stmt.value)
            })
    }

    /// validate
    pub fn validate(&self) -> bool {
        !self.0.is_empty()
    }
}

impl std::fmt::Display for Headers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .0
            .iter()
            .map(|(key, stmt)| format!("{}{}", key, stmt))
            .collect::<Vec<String>>()
            .join(fmt_condition_connector().as_str());

        let _ = write!(f, "[headers] {}", s);

        Ok(())
    }
}

#[cfg(test)]
mod tests;
