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
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn default_counter() -> Arc<AtomicUsize> {
    Arc::new(AtomicUsize::new(0))
}

#[derive(Clone, Deserialize, Debug)]
pub struct RuleSet {
    pub prefix: Option<Prefix>,
    pub default: Option<DefaultRespond>,
    pub guard: Option<Guard>,
    pub rules: Vec<Rule>,
    /// Per-rule-set strategy override (RFC 025).
    /// When `Some`, this strategy is used instead of the service-level one.
    /// When `None`, the service-level strategy applies.
    #[serde(default)]
    pub strategy: Option<Strategy>,
    #[serde(skip)]
    pub file_path: String,
    /// Per-rule-set round-robin counter. Shared across clones via `Arc`.
    #[serde(skip, default = "default_counter")]
    pub round_robin_counter: Arc<AtomicUsize>,
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
    // clippy: RoutingError::RuleSetParse is a public error type (RoutingResult
    // is part of this crate's stable surface); boxing its large variant would
    // change that type's shape, which RFC 030 §6 requires escalating rather
    // than fixing inline. See ESCALATION-002 in the RFC 030 review-request
    // package for the design request this raises.
    #[allow(clippy::result_large_err)]
    pub fn new(
        rule_set_file_path: &str,
        current_dir_to_config_dir_relative_path: &str,
        rule_set_idx: usize,
    ) -> RoutingResult<Self> {
        let path = Path::new(rule_set_file_path);
        let toml_string =
            fs::read_to_string(rule_set_file_path).map_err(|e| RoutingError::RuleSetRead {
                path: path.to_path_buf(),
                source: e,
            })?;

        let mut ret: Self =
            toml::from_str(&toml_string).map_err(|e| RoutingError::RuleSetParse {
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
        // - round-robin counter (starts at 0; shared across clones via Arc)
        ret.round_robin_counter = Arc::new(AtomicUsize::new(0));

        Ok(ret)
    }

    /// find rule matching request and return its respond content
    pub fn find_matched(
        &self,
        parsed_request: &ParsedRequest,
        strategy: Option<&Strategy>,
        rule_set_idx: usize,
    ) -> Option<Respond> {
        match self.prefix.as_ref() {
            Some(prefix)
                if prefix.url_path_prefix.is_some()
                    && !parsed_request
                        .url_path
                        .starts_with(prefix.url_path_prefix.as_ref().unwrap()) =>
            {
                return None;
            }
            _ => (),
        }

        // RFC 025: per-rule-set strategy override.
        // The rule set's own strategy takes precedence over the service-level one.
        let effective_strategy = self
            .strategy
            .as_ref()
            .or(strategy)
            .unwrap_or(&Strategy::FirstMatch);
        let strategy = effective_strategy;

        match strategy {
            Strategy::FirstMatch => {
                for (rule_idx, rule) in self.rules.iter().enumerate() {
                    if rule.when.is_match(parsed_request, rule_idx, rule_set_idx) {
                        return Some(rule.respond.clone());
                    }
                }
                None
            }

            Strategy::UniformRandom { seed } => {
                // Collect all matching rules, then pick uniformly at random.
                let matches: Vec<&Rule> = self
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
                    .map(|(_, r)| r)
                    .collect();

                if matches.is_empty() {
                    return None;
                }
                let mut rng = crate::strategy::make_rng(*seed);
                let idx = rng.next_index(matches.len());
                Some(matches[idx].respond.clone())
            }

            Strategy::WeightedRandom { seed } => {
                // Collect matching rules with their effective weights.
                let candidates: Vec<(&Rule, u32)> = self
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
                    .map(|(_, r)| (r, r.weight.unwrap_or(1)))
                    .filter(|(_, w)| *w > 0)
                    .collect();

                if candidates.is_empty() {
                    return None;
                }

                let total: u32 = candidates.iter().map(|(_, w)| w).sum();
                let mut rng = crate::strategy::make_rng(*seed);
                let pick = (rng.next() % total as u64) as u32;
                let mut acc = 0u32;
                for (rule, weight) in &candidates {
                    acc += weight;
                    if pick < acc {
                        return Some(rule.respond.clone());
                    }
                }
                // Fallback (rounding edge): return last candidate.
                candidates.last().map(|(r, _)| r.respond.clone())
            }

            Strategy::Priority { tiebreaker } => {
                // Collect matching rules with their priority.
                let matches: Vec<(&Rule, i32)> = self
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
                    .map(|(_, r)| (r, r.priority.unwrap_or(0)))
                    .collect();

                if matches.is_empty() {
                    return None;
                }

                let max_priority = matches.iter().map(|(_, p)| *p).max().unwrap();
                let top: Vec<&Rule> = matches
                    .into_iter()
                    .filter(|(_, p)| *p == max_priority)
                    .map(|(r, _)| r)
                    .collect();

                match tiebreaker {
                    crate::strategy::PriorityTiebreaker::FirstMatch => {
                        top.into_iter().next().map(|r| r.respond.clone())
                    }
                    crate::strategy::PriorityTiebreaker::UniformRandom => {
                        let mut rng = crate::strategy::make_rng(None);
                        let idx = rng.next_index(top.len());
                        Some(top[idx].respond.clone())
                    }
                }
            }

            Strategy::RoundRobin => {
                let matches: Vec<&Rule> = self
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
                    .map(|(_, r)| r)
                    .collect();

                if matches.is_empty() {
                    return None;
                }

                // Relaxed ordering: atomicity without sequential consistency
                // is sufficient for a mock server (slight counter reorder
                // on concurrent requests is acceptable).
                let idx = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % matches.len();

                Some(matches[idx].respond.clone())
            }
        }
    }

    /// validate
    pub fn validate(&self) -> bool {
        true
    }

    /// dir_prefix as string possibly as empty
    pub fn dir_prefix(&self) -> String {
        self.prefix
            .clone()
            .unwrap_or_default()
            .respond_dir_prefix
            .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsed_request::ParsedRequest;
    use crate::rule_set::rule::{
        Rule,
        respond::Respond,
        when::{When, request::Request},
    };
    use crate::strategy::Strategy;

    /// Build a minimal `ParsedRequest` matching `url_path`.
    fn get_req(url_path: &str) -> ParsedRequest {
        let req = hyper::Request::builder()
            .method("GET")
            .uri(url_path)
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();
        ParsedRequest {
            url_path: url_path.to_owned(),
            component_parts: parts,
            body_json: None,
        }
    }

    /// Build a `RuleSet` with `n` rules, all matching `url_path`,
    /// responding with `"response_0"`, `"response_1"`, …
    fn make_round_robin_set(n: usize, url_path: &str) -> RuleSet {
        use crate::rule_set::rule::when::request::url_path::{UrlPath, UrlPathConfig};

        let rules = (0..n)
            .map(|i| Rule {
                when: When {
                    request: Request {
                        url_path_config: Some(UrlPathConfig::Simple(url_path.to_owned())),
                        url_path: Some(UrlPath {
                            value: url_path.to_owned(),
                            value_with_prefix: url_path.to_owned(),
                            op: None,
                        }),
                        http_method: None,
                        headers: None,
                        body: None,
                    },
                },
                respond: Respond {
                    text: Some(format!("response_{}", i)),
                    file_path: None,
                    csv_records_key: None,
                    status: None,
                    status_code: None,
                    headers: None,
                    delay_response_milliseconds: None,
                },
                weight: None,
                priority: None,
            })
            .collect();

        RuleSet {
            prefix: None,
            default: None,
            guard: None,
            rules,
            strategy: None,
            file_path: String::new(),
            round_robin_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[test]
    fn round_robin_cycles_through_matching_rules() {
        let rs = make_round_robin_set(2, "/api");
        let req = get_req("/api");
        let strategy = Strategy::RoundRobin;

        let r0 = rs.find_matched(&req, Some(&strategy), 0).expect("match 0");
        let r1 = rs.find_matched(&req, Some(&strategy), 0).expect("match 1");
        let r2 = rs.find_matched(&req, Some(&strategy), 0).expect("match 2");

        assert_eq!(r0.text.as_deref(), Some("response_0"));
        assert_eq!(r1.text.as_deref(), Some("response_1"));
        assert_eq!(r2.text.as_deref(), Some("response_0"), "cycle back");
    }

    #[test]
    fn round_robin_three_rules_full_cycle() {
        let rs = make_round_robin_set(3, "/api");
        let req = get_req("/api");
        let strategy = Strategy::RoundRobin;

        let texts: Vec<String> = (0..6)
            .map(|_| {
                rs.find_matched(&req, Some(&strategy), 0)
                    .unwrap()
                    .text
                    .clone()
                    .unwrap()
            })
            .collect();

        assert_eq!(
            texts,
            vec![
                "response_0",
                "response_1",
                "response_2",
                "response_0",
                "response_1",
                "response_2",
            ]
        );
    }

    #[test]
    fn round_robin_no_match_does_not_advance_counter() {
        let rs = make_round_robin_set(2, "/api");
        let strategy = Strategy::RoundRobin;

        // Non-matching request must not advance counter.
        let miss = rs.find_matched(&get_req("/other"), Some(&strategy), 0);
        assert!(miss.is_none(), "non-matching path should miss");

        // Counter at 0 still — first hit returns response_0.
        let hit = rs
            .find_matched(&get_req("/api"), Some(&strategy), 0)
            .expect("should match");
        assert_eq!(hit.text.as_deref(), Some("response_0"));
    }

    #[test]
    fn round_robin_single_match_always_same() {
        let rs = make_round_robin_set(1, "/api");
        let req = get_req("/api");
        let strategy = Strategy::RoundRobin;

        for _ in 0..5 {
            let r = rs.find_matched(&req, Some(&strategy), 0).expect("match");
            assert_eq!(r.text.as_deref(), Some("response_0"));
        }
    }
}
