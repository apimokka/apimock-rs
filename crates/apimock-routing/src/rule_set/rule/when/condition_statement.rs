use serde::Deserialize;

use super::request::rule_op::RuleOp;

#[derive(Clone, Debug, Deserialize)]
pub struct ConditionStatement {
    pub op: Option<RuleOp>,
    pub value: String,
}

impl std::fmt::Display for ConditionStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}`{}`", self.op.clone().unwrap_or_default(), self.value)
    }
}
