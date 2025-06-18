use console::style;
use hyper::{header::HeaderValue, HeaderMap};
use serde::Deserialize;

use std::collections::HashMap;

use super::util::fmt_condition_connector;
use crate::core::server::routing::rule_set::rule::{
    when::condition_statement::ConditionStatement, ConditionKey,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct Headers(pub HashMap<ConditionKey, ConditionStatement>);

impl Headers {
    /// check if `headers` in `when` matches
    pub fn is_match(
        &self,
        parsed_request_headers: &HeaderMap<HeaderValue>,
        rule_idx: usize,
        rule_set_idx: usize,
    ) -> bool {
        self.0
            .iter()
            .all(|(matcher_header_key, matcher_header_value)| {
                let parsed_request_header_value = match parsed_request_headers.get(matcher_header_key) {
                    Some(x) => x,
                    None => return false,
                };

                let parsed_request_header_value = match parsed_request_header_value.to_str() {
                    Ok(x) => x,
                    Err(err) => {
                        log::error!(
                            "{} to get request header value by key (rule #{} in rule set #{}):\n`{}`\n({})",
                            style("failed").red(),
                            rule_idx + 1,
                            rule_set_idx + 1,
                            matcher_header_key,
                            err,
                        );
                        return true;
                    }
                };

                let ret = matcher_header_value
                    .op
                    .clone()
                    .unwrap_or_default()
                    .is_match(parsed_request_header_value, &matcher_header_value.value);
                ret
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
            .map(|(header_key, header_statement)| format!("{}{}", header_key, header_statement))
            .collect::<Vec<String>>()
            .join(fmt_condition_connector().as_str());

        let _ = write!(f, "[headers] {}", s);

        Ok(())
    }
}
