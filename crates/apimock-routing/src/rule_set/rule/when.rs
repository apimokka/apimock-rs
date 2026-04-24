use serde::Deserialize;

mod condition_statement;
pub mod request;

use crate::parsed_request::ParsedRequest;
use request::Request;

#[derive(Clone, Deserialize, Debug)]
pub struct When {
    pub request: Request,
}

impl When {
    /// match with condition
    pub fn is_match(
        &self,
        parsed_request: &ParsedRequest,
        rule_idx: usize,
        rule_set_idx: usize,
    ) -> bool {
        self.request
            .is_match(parsed_request, rule_idx, rule_set_idx)
    }

    /// validate
    pub fn validate(&self, rule_idx: usize, rule_set_idx: usize) -> bool {
        self.request.validate(rule_idx, rule_set_idx)
    }
}

impl std::fmt::Display for When {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = write!(f, "{}", self.request);
        Ok(())
    }
}
