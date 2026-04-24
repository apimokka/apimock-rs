use serde::Deserialize;
use serde_json::Value;

use std::collections::HashMap;

mod body_kind;

use super::util::fmt_condition_connector;
use crate::{
    parsed_request::ParsedRequest,
    rule_set::rule::{ConditionKey, when::condition_statement::ConditionStatement},
    util::json::json_value_by_jsonpath,
};
use body_kind::BodyKind;

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct Body(pub HashMap<BodyKind, HashMap<ConditionKey, ConditionStatement>>);

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

        let ret = matcher_body_json_condition.iter().all(
            |(matcher_json_condition_key, matcher_json_condition_statement)| {
                let request_body_json_value =
                    match json_value_by_jsonpath(request_body_json, matcher_json_condition_key) {
                        Some(x) => match x {
                            Value::String(s) => s.to_owned(),
                            _ => x.to_string(),
                        },
                        None => return false,
                    };

                let ret = matcher_json_condition_statement
                    .op
                    .clone()
                    .unwrap_or_default()
                    .is_match(
                        request_body_json_value.as_str(),
                        &matcher_json_condition_statement.value,
                    );
                ret
            },
        );

        ret
    }

    /// validate
    pub fn validate(&self) -> bool {
        if self.0.is_empty() {
            return false;
        }

        for (_, body_kind_map) in self.0.iter() {
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
