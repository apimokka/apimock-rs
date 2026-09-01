use serde::Deserialize;

use indexmap::IndexMap;
use std::collections::HashMap;

pub mod body_kind;
pub mod body_operator;

use super::util::fmt_condition_connector;
use crate::{parsed_request::ParsedRequest, util::json::json_value_by_jsonpath};
use body_kind::BodyKind;
use body_operator::BodyOperator;

/// Body match conditions, keyed by [`BodyKind`] (currently only
/// [`BodyKind::Json`]) and then by a path identifying the value
/// inside the request body to compare.
///
/// # Path syntax
///
/// The inner key is a **dotted path** — *not* canonical JSONPath.
/// `a.b.c` reaches into nested object keys; numeric segments index
/// arrays (`items.0.name`). Wildcards, filters, and the canonical
/// `$.` prefix are not supported. A path written as `"$.foo"` will
/// only match a JSON document with a top-level `$` key, almost
/// never what the writer intended.
///
/// See [`crate::util::json::json_value_by_jsonpath`] for the full
/// contract.
///
/// # Extended operator set (RFC 008)
///
/// The `op` field in each condition statement accepts all
/// [`BodyOperator`] variants, including numeric comparisons
/// (`greater_than`, `less_than`, etc.), type-aware equality
/// (`equal_typed`), presence checks (`exists`, `absent`), and array
/// predicates (`array_length_equal`, `array_contains`, etc.).
#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct Body(pub HashMap<BodyKind, IndexMap<String, BodyConditionStatement>>);

/// Per-condition statement for body matching.
///
/// Uses [`BodyOperator`] instead of the shared
/// [`ConditionStatement`](crate::rule_set::rule::when::condition_statement::ConditionStatement)
/// type so the richer body operator set doesn't bleed into header
/// matching (which uses
/// [`ConditionStatement`](crate::rule_set::rule::when::condition_statement::ConditionStatement)
/// + [`super::rule_op::RuleOp`]).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyConditionStatement {
    pub op: Option<BodyOperator>,
    pub value: String,
}

impl std::fmt::Display for BodyConditionStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}`{}`", self.op.clone().unwrap_or_default(), self.value)
    }
}

impl Body {
    /// check if `body` in `when` matches
    pub fn is_match(&self, parsed_request: &ParsedRequest) -> bool {
        // todo: support other types than json (such as form value) in the future
        let request_body_json = match parsed_request.body_json.as_ref() {
            Some(x) => x,
            None => return false,
        };

        let matcher_body_json_condition = match self.0.get(&BodyKind::Json) {
            Some(x) if !x.is_empty() => x,
            _ => return false,
        };

        matcher_body_json_condition.iter().all(
            |(matcher_json_condition_key, matcher_json_condition_statement)| {
                let op = matcher_json_condition_statement
                    .op
                    .clone()
                    .unwrap_or_default();

                // Presence operators are handled before path resolution:
                // Absent means "path must NOT resolve".
                if op == BodyOperator::Absent {
                    return json_value_by_jsonpath(request_body_json, matcher_json_condition_key)
                        .is_none();
                }

                let resolved =
                    match json_value_by_jsonpath(request_body_json, matcher_json_condition_key) {
                        Some(v) => v,
                        None => return false,
                    };

                op.is_match(resolved, &matcher_json_condition_statement.value)
            },
        )
    }

    /// validate
    pub fn validate(&self) -> bool {
        if self.0.is_empty() {
            return false;
        }

        for body_kind_map in self.0.values() {
            if body_kind_map.is_empty() {
                return false;
            }
        }

        true
    }
}

impl std::fmt::Display for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (body_kind, body_condition) in self.0.iter() {
            let s = body_condition
                .iter()
                .map(|(key, statement)| format!("{}{}", key, statement))
                .collect::<Vec<String>>()
                .join(fmt_condition_connector().as_str());

            let _ = write!(f, "[{}] {}", body_kind, s);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
