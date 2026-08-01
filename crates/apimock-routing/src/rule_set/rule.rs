use console::style;
use hyper::StatusCode;
use serde::Deserialize;

pub mod respond;
mod util;
pub mod when;

use super::RuleSet;
use respond::Respond;
use util::url_path_with_prefix;
use when::{
    When,
    request::url_path::{UrlPath, UrlPathConfig},
};

type ConditionKey = String;

#[derive(Clone, Deserialize, Debug)]
pub struct Rule {
    pub when: When,
    pub respond: Respond,
    /// Optional weight for `WeightedRandom` strategy (default: 1).
    /// Ignored under other strategies.
    #[serde(default)]
    pub weight: Option<u32>,
    /// Optional priority for `Priority` strategy. Higher values win.
    /// Ignored under other strategies.
    #[serde(default)]
    pub priority: Option<i32>,
}

impl Rule {
    /// Pre-compute derived fields that don't change per request.
    ///
    /// # Why this is a separate pass
    ///
    /// Deserialization gives us the raw TOML structure. A few fields need
    /// post-processing: URL paths must be joined with the rule-set prefix
    /// and normalized, and the HTTP status code stored as a `u16` in the
    /// config needs to be validated into a `StatusCode`. Doing it once at
    /// startup keeps the per-request match path allocation-free and
    /// panic-free.
    ///
    /// If the configured status is invalid (e.g. `9999`) we log and leave
    /// `status_code` as `None`; the surrounding validate pass will catch
    /// it and prevent the server from starting. Previously this used
    /// `expect` and aborted immediately — that's worse for debugging
    /// because the user only gets one error at a time.
    pub fn compute_derived_fields(
        &self,
        rule_set: &RuleSet,
        rule_idx: usize,
        rule_set_idx: usize,
    ) -> Self {
        let mut ret = self.to_owned();

        // - url_path_with_prefix
        let url_path = match ret.when.request.url_path_config.as_ref() {
            Some(url_path_config) => match url_path_config {
                UrlPathConfig::Simple(s) => Some(UrlPath {
                    value: s.clone(),
                    value_with_prefix: url_path_with_prefix(s.as_str(), rule_set.prefix.as_ref()),
                    op: None,
                }),

                UrlPathConfig::Detailed(url_path) => Some(UrlPath {
                    value: url_path.value.clone(),
                    value_with_prefix: url_path_with_prefix(
                        url_path.value.as_str(),
                        rule_set.prefix.as_ref(),
                    ),
                    op: url_path.op.clone(),
                }),
            },
            None => None,
        };
        ret.when.request.url_path = url_path;

        // - status_code: fail soft here, fail hard in validate()
        if let Some(status) = ret.respond.status {
            match StatusCode::from_u16(status) {
                Ok(status_code) => ret.respond.status_code = Some(status_code),
                Err(err) => {
                    log::error!(
                        "{} status code {} (rule #{} in rule set #{}): {}",
                        style("invalid").red(),
                        status,
                        rule_idx + 1,
                        rule_set_idx + 1,
                        err,
                    );
                    ret.respond.status_code = None;
                }
            }
        }

        ret
    }

    pub fn validate(&self, dir_prefix: &str, rule_idx: usize, rule_set_idx: usize) -> bool {
        self.when.validate(rule_idx, rule_set_idx)
            && self.respond.validate(dir_prefix, rule_idx, rule_set_idx)
    }
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = write!(f, "- ");
        let _ = write!(f, "{} {}", style("[when]").yellow(), self.when);
        let _ = write!(f, "{} {}", style("[respond]").yellow(), self.respond);
        Ok(())
    }
}
