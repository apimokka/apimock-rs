use serde::Deserialize;

use std::{fs, path::Path};

mod default_respond;
mod guard;
pub mod prefix;
pub mod rule;

use crate::{
    error::{RoutingError, RoutingResult},
    parsed_request::ParsedRequest,
    strategy::Strategy,
    util::http::normalize_url_path,
};
use default_respond::DefaultRespond;
use guard::Guard;
use prefix::Prefix;
use rule::{Rule, respond::Respond};

/// A named collection of routing rules, loaded from one TOML file.
///
/// # Why rule sets, not a single flat rule list
///
/// Large mock APIs tend to group related endpoints (e.g. all of `/api/v1`
/// under one auth scheme). A rule set lets operators share a URL prefix
/// and a respond-dir prefix across many rules, and to split their config
/// across multiple files that can be enabled/disabled independently.
/// Match order across sets is determined by the order in
/// `service.rule_sets`, so the most specific set can be listed first.
#[derive(Clone, Deserialize, Debug)]
pub struct RuleSet {
    pub prefix: Option<Prefix>,
    pub default: Option<DefaultRespond>,
    pub guard: Option<Guard>,
    pub rules: Vec<Rule>,
    #[serde(skip)]
    pub file_path: String,
}

impl RuleSet {
    /// Load a rule set from a TOML file on disk.
    ///
    /// # Why errors are typed and not panics
    ///
    /// In 4.6.x this used `expect` + `panic!`, so a missing or malformed
    /// rule set aborted the process. Because rule sets are edited
    /// frequently during development, those panics were a common papercut.
    /// Now any failure becomes an `RoutingError::RuleSetRead` / `::RuleSetParse`
    /// that the caller can surface cleanly.
    pub fn new(
        rule_set_file_path: &str,
        current_dir_to_config_dir_relative_path: &str,
        rule_set_idx: usize,
    ) -> RoutingResult<Self> {
        let path = Path::new(rule_set_file_path);
        let toml_string = fs::read_to_string(rule_set_file_path).map_err(|e| {
            RoutingError::RuleSetRead {
                path: path.to_path_buf(),
                source: e,
            }
        })?;

        let mut ret: Self = toml::from_str(&toml_string).map_err(|e| RoutingError::RuleSetParse {
            path: path.to_path_buf(),
            canonical: path.canonicalize().ok(),
            source: e,
        })?;

        // - prefix: fill in defaults and normalize
        let mut prefix = ret.prefix.clone().unwrap_or_default();

        // normalize `url_path` so later matching doesn't have to deal with
        // leading/trailing slash variations
        prefix.url_path_prefix = prefix
            .url_path_prefix
            .as_deref()
            .map(|p| normalize_url_path(p, None));

        // respond_dir prefix: default to "." and anchor it under the
        // config-file directory so relative paths in rule sets are
        // relative to the rule-set file, not the working directory
        let respond_dir_prefix = prefix.respond_dir_prefix.as_deref().unwrap_or(".");

        let respond_dir_prefix =
            Path::new(current_dir_to_config_dir_relative_path).join(respond_dir_prefix);
        let respond_dir_prefix = respond_dir_prefix.to_str().ok_or_else(|| {
            RoutingError::RuleSetRead {
                path: path.to_path_buf(),
                // We synthesize an io::Error here only because the variant
                // needs one; the real failure is "path contains non-UTF-8
                // bytes", which is vanishingly rare but not impossible on
                // Unix. Using `InvalidData` keeps it distinguishable.
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "respond_dir path contains non-UTF-8 bytes: {}",
                        respond_dir_prefix.to_string_lossy()
                    ),
                ),
            }
        })?;

        prefix.respond_dir_prefix = Some(respond_dir_prefix.to_owned());
        ret.prefix = Some(prefix);

        // - rules: compute any derived fields (normalized URL path with
        //   prefix already applied, resolved status code, etc.) so the
        //   request-time hot path doesn't have to repeat the work
        ret.rules = ret
            .rules
            .iter()
            .enumerate()
            .map(|(rule_idx, rule)| rule.compute_derived_fields(&ret, rule_idx, rule_set_idx))
            .collect();

        // - file path (kept for log/display only)
        ret.file_path = rule_set_file_path.to_owned();

        Ok(ret)
    }

    /// find rule matching request and return its respond content
    pub fn find_matched(
        &self,
        parsed_request: &ParsedRequest,
        strategy: Option<&Strategy>,
        rule_set_idx: usize,
    ) -> Option<Respond> {
        let _ = match self.prefix.as_ref() {
            Some(prefix) if prefix.url_path_prefix.is_some() => {
                if !parsed_request
                    .url_path
                    .starts_with(prefix.url_path_prefix.as_ref().unwrap())
                {
                    return None;
                }
            }
            _ => (),
        };

        for (rule_idx, rule) in self.rules.iter().enumerate() {
            let is_match = rule.when.is_match(parsed_request, rule_idx, rule_set_idx);
            if is_match {
                // todo: last match in the future ?
                match strategy {
                    Some(&Strategy::FirstMatch) | None => return Some(rule.respond.to_owned()),
                }
            }
        }

        None
    }

    /// validate
    pub fn validate(&self) -> bool {
        true
    }

    /// dir_prefix as string possibly as empty
    pub fn dir_prefix(&self) -> String {
        if let Some(dir_prefix) = self.prefix.clone().unwrap_or_default().respond_dir_prefix {
            dir_prefix
        } else {
            String::new()
        }
    }
}

impl std::fmt::Display for RuleSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(x) = self.prefix.as_ref() {
            let _ = write!(f, "{}", x);
        }
        if let Some(x) = self.guard.as_ref() {
            let _ = write!(f, "{}", x);
        }
        if let Some(x) = self.default.as_ref() {
            let _ = write!(f, "{}", x);
        }
        for rule in self.rules.iter() {
            let _ = write!(f, "{}", rule);
        }
        Ok(())
    }
}
