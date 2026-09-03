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
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// RFC 070: `round_robin`'s per-match-group counters, keyed by the
/// exact set of matched rule indices for a request (in match order).
/// Two requests that select the same candidate rules share a counter;
/// requests selecting a different candidate set advance independently.
///
/// # Bounded by rule-set structure, not by traffic
///
/// A rule set of `N` rules has at most `2^N` distinct possible matched
/// subsets — the powerset of "which rules matched" — so this map's size
/// is capped by the rule set's own structure (how many rules it has,
/// and which subsets its conditions can actually produce, which is
/// usually far fewer than `2^N` in practice). It does **not** grow with
/// request *volume*: two requests that induce the same subset of
/// matching rules — however many requests, however varied their other
/// fields — always hash to the same key and share one counter entry.
/// There is no way for continued traffic against a fixed rule set to
/// keep adding new keys once every reachable subset has been seen once.
/// Established (not assumed) for the review package: see the
/// `round_robin_map_size_does_not_grow_with_request_volume` test below,
/// which runs many more requests than there are possible groups and
/// asserts the map's size stops changing.
type RoundRobinCounters = Arc<Mutex<HashMap<Vec<usize>, usize>>>;

fn default_round_robin_counters() -> RoundRobinCounters {
    Arc::new(Mutex::new(HashMap::new()))
}

/// `true` when `value`, interpreted as a path, is made up of nothing
/// but current-directory (`.`) components — `.`, `./.`, `././.`, and so
/// on, all provably the same directory. RFC 058's narrow repair for a
/// `respond_dir` grown by the pre-fix bug.
///
/// `Path::components()` already collapses any number of redundant `./`
/// segments into a single `CurDir` component (confirmed empirically:
/// `Path::new("././.").components()` yields exactly one `CurDir`), so
/// this is a direct, platform-correct check — no manual splitting on
/// `/` (which would be wrong on Windows) or `\` (which would be wrong
/// everywhere else) needed.
fn is_purely_current_dir(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|c| matches!(c, std::path::Component::CurDir))
}

/// `#[non_exhaustive]` (RFC 041): the only public constructor is
/// [`RuleSet::new`] (loads from a TOML file); nothing outside this
/// crate builds one by literal today.
#[derive(Clone, Deserialize, Debug)]
#[non_exhaustive]
#[serde(deny_unknown_fields)]
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
    /// Per-match-group round-robin counters (RFC 070). Shared across
    /// clones via `Arc`. See `RoundRobinCounters`'s own doc comment for
    /// the key and the bounded-growth argument.
    ///
    /// # Private, deliberately (REVIEW-001, `audit-t2-silent-wrongness`)
    ///
    /// This field used to be `pub` (as `round_robin_counter`, a plain
    /// `Arc<AtomicUsize>`) purely because `RuleSet` is otherwise a plain
    /// data struct — not because anything outside this crate ever read
    /// or wrote it (checked: nothing did). RFC 070's fix could not keep
    /// the old type (one `AtomicUsize` cannot express per-group state),
    /// so the field's *type* was always going to change regardless;
    /// making it private at the same time removes internal scheduling
    /// state from the public API entirely, rather than replacing one
    /// public representation with another and calling that additive.
    #[serde(skip, default = "default_round_robin_counters")]
    round_robin_counters: RoundRobinCounters,
    /// The response directory `Respond::file_path` resolves against,
    /// computed once by `RuleSet::new` (RFC 058). Never deserialised,
    /// never written back to `prefix.respond_dir_prefix` — see
    /// `dir_prefix()` and `Prefix`'s own doc comment for why this is a
    /// separate field rather than the old overwrite-in-place scheme.
    #[serde(skip)]
    pub resolved_respond_dir: String,
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
        let toml_string =
            fs::read_to_string(rule_set_file_path).map_err(|e| RoutingError::RuleSetRead {
                path: path.to_path_buf(),
                source: e,
            })?;

        let mut ret: Self =
            toml::from_str(&toml_string).map_err(|e| RoutingError::RuleSetParse {
                path: path.to_path_buf(),
                canonical: path.canonicalize().ok(),
                source: Box::new(e),
            })?;

        // - prefix (RFC 058): normalize what was authored, in place —
        //   never manufacture a `[prefix]` section that wasn't there.
        //   `ret.prefix` stays exactly `None` if the file never had one
        //   (Goal 2); a `Some` one keeps only what it already had.
        if let Some(prefix) = ret.prefix.as_mut() {
            // normalize `url_path` so later matching doesn't have to deal
            // with leading/trailing slash variations
            prefix.url_path_prefix = prefix
                .url_path_prefix
                .as_deref()
                .map(|p| normalize_url_path(p, None));

            // Narrow repair (RFC 058 Unresolved 2): a `respond_dir` that
            // is purely `./`-segments (`./.`, `././.`, …) is provably
            // the same directory as `.` — collapse it. This is how a
            // file already grown by the pre-fix bug heals, one save at
            // a time; an authored path like `responses` or
            // `./responses` is never touched, since it isn't purely
            // current-dir segments.
            if let Some(dir) = prefix.respond_dir_prefix.as_deref()
                && is_purely_current_dir(dir)
            {
                prefix.respond_dir_prefix = Some(".".to_owned());
            }
        }

        // - resolved_respond_dir: always computed, regardless of whether
        //   `[prefix]` was authored — "." (the config-relative directory)
        //   is the correct resolution for "no respond_dir was written",
        //   the same default `unwrap_or(".")` always meant. This is the
        //   field the matcher reads (`dir_prefix()`); it is never written
        //   back into `prefix.respond_dir_prefix`, which is the fix
        //   itself — see `Prefix`'s own doc comment for the mechanism
        //   this replaces.
        let authored_respond_dir = ret
            .prefix
            .as_ref()
            .and_then(|p| p.respond_dir_prefix.as_deref())
            .unwrap_or(".");
        let resolved_respond_dir =
            Path::new(current_dir_to_config_dir_relative_path).join(authored_respond_dir);
        let resolved_respond_dir = resolved_respond_dir.to_str().ok_or_else(|| {
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
                        resolved_respond_dir.to_string_lossy()
                    ),
                ),
            }
        })?;
        ret.resolved_respond_dir = resolved_respond_dir.to_owned();

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
        // - round-robin counters (starts empty; shared across clones via Arc)
        ret.round_robin_counters = default_round_robin_counters();

        Ok(ret)
    }

    /// Find the rule matching `parsed_request` and return its 0-based
    /// index alongside its respond content. The index is RFC 055's
    /// addition (`apimock get --why` needs to name which rule decided
    /// the answer); every selection branch below already computes it
    /// while filtering, so carrying it out costs nothing.
    pub fn find_matched(
        &self,
        parsed_request: &ParsedRequest,
        strategy: Option<&Strategy>,
        rule_set_idx: usize,
    ) -> Option<(usize, Respond)> {
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
                        return Some((rule_idx, rule.respond.clone()));
                    }
                }
                None
            }

            Strategy::UniformRandom { seed } => {
                // Collect all matching rules, then pick uniformly at random.
                let matches: Vec<(usize, &Rule)> = self
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
                    .collect();

                if matches.is_empty() {
                    return None;
                }
                let mut rng = crate::strategy::make_rng(*seed);
                let idx = rng.next_index(matches.len());
                let (rule_idx, rule) = matches[idx];
                Some((rule_idx, rule.respond.clone()))
            }

            Strategy::WeightedRandom { seed } => {
                // Collect matching rules with their effective weights.
                let candidates: Vec<(usize, &Rule, u32)> = self
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
                    .map(|(idx, r)| (idx, r, r.weight.unwrap_or(1)))
                    .filter(|(_, _, w)| *w > 0)
                    .collect();

                if candidates.is_empty() {
                    return None;
                }

                let total: u32 = candidates.iter().map(|(_, _, w)| w).sum();
                let mut rng = crate::strategy::make_rng(*seed);
                let pick = (rng.next() % total as u64) as u32;
                let mut acc = 0u32;
                for (rule_idx, rule, weight) in &candidates {
                    acc += weight;
                    if pick < acc {
                        return Some((*rule_idx, rule.respond.clone()));
                    }
                }
                // Fallback (rounding edge): return last candidate.
                candidates
                    .last()
                    .map(|(rule_idx, r, _)| (*rule_idx, r.respond.clone()))
            }

            Strategy::Priority { tiebreaker } => {
                // Collect matching rules with their priority.
                let matches: Vec<(usize, &Rule, i32)> = self
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
                    .map(|(idx, r)| (idx, r, r.priority.unwrap_or(0)))
                    .collect();

                if matches.is_empty() {
                    return None;
                }

                let max_priority = matches.iter().map(|(_, _, p)| *p).max().unwrap();
                let top: Vec<(usize, &Rule)> = matches
                    .into_iter()
                    .filter(|(_, _, p)| *p == max_priority)
                    .map(|(idx, r, _)| (idx, r))
                    .collect();

                match tiebreaker {
                    crate::strategy::PriorityTiebreaker::FirstMatch => top
                        .into_iter()
                        .next()
                        .map(|(idx, r)| (idx, r.respond.clone())),
                    crate::strategy::PriorityTiebreaker::UniformRandom => {
                        let mut rng = crate::strategy::make_rng(None);
                        let idx = rng.next_index(top.len());
                        let (rule_idx, rule) = top[idx];
                        Some((rule_idx, rule.respond.clone()))
                    }
                }
            }

            Strategy::RoundRobin => {
                let matches: Vec<(usize, &Rule)> = self
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
                    .collect();

                if matches.is_empty() {
                    return None;
                }

                // RFC 070: the counter is keyed by *which* rules matched,
                // not shared across the whole rule set — two requests
                // that select the same candidates advance the same
                // counter; requests selecting a different candidate set
                // do not interfere with each other. `matches` is already
                // in ascending rule order (built by iterating `self.rules`
                // in order), so the key is stable and comparable across
                // calls without needing to sort it here.
                let group_key: Vec<usize> = matches.iter().map(|(idx, _)| *idx).collect();
                let idx = {
                    let mut counters = self
                        .round_robin_counters
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    let counter = counters.entry(group_key).or_insert(0);
                    let current = *counter;
                    // Wrapping: a counter that reaches usize::MAX resumes
                    // at 0 rather than panicking. The sequence it produces
                    // from that point on is still a valid rotation (every
                    // value 0..matches.len() still occurs the same
                    // proportion of the time); only reachable after more
                    // requests against one group than fits in a usize,
                    // which is not a real concern on any platform this
                    // targets.
                    *counter = current.wrapping_add(1);
                    current % matches.len()
                };

                let (rule_idx, rule) = matches[idx];
                Some((rule_idx, rule.respond.clone()))
            }
        }
    }

    /// validate
    pub fn validate(&self) -> bool {
        true
    }

    /// The response directory, resolved against the process's CWD, that
    /// `Respond::file_path` is served relative to. Always populated by
    /// `RuleSet::new` — "." (the rule-set file's own directory) when no
    /// `respond_dir` was ever authored, the joined path otherwise.
    ///
    /// Deliberately reads `resolved_respond_dir`, not
    /// `prefix.respond_dir_prefix` (RFC 058) — that field holds only
    /// what the user wrote (or is absent entirely), so it cannot answer
    /// "where does the matcher actually look" on its own.
    pub fn dir_prefix(&self) -> String {
        self.resolved_respond_dir.clone()
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
            body_len: None,
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
                    json: None,
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
            round_robin_counters: default_round_robin_counters(),
            resolved_respond_dir: ".".to_owned(),
        }
    }

    #[test]
    fn round_robin_cycles_through_matching_rules() {
        let rs = make_round_robin_set(2, "/api");
        let req = get_req("/api");
        let strategy = Strategy::RoundRobin;

        let (idx0, r0) = rs.find_matched(&req, Some(&strategy), 0).expect("match 0");
        let (idx1, r1) = rs.find_matched(&req, Some(&strategy), 0).expect("match 1");
        let (idx2, r2) = rs.find_matched(&req, Some(&strategy), 0).expect("match 2");

        assert_eq!(r0.text.as_deref(), Some("response_0"));
        assert_eq!(r1.text.as_deref(), Some("response_1"));
        assert_eq!(r2.text.as_deref(), Some("response_0"), "cycle back");
        assert_eq!((idx0, idx1, idx2), (0, 1, 0));
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
                    .1
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
        let (hit_idx, hit) = rs
            .find_matched(&get_req("/api"), Some(&strategy), 0)
            .expect("should match");
        assert_eq!(hit.text.as_deref(), Some("response_0"));
        assert_eq!(hit_idx, 0);
    }

    #[test]
    fn round_robin_single_match_always_same() {
        let rs = make_round_robin_set(1, "/api");
        let req = get_req("/api");
        let strategy = Strategy::RoundRobin;

        for _ in 0..5 {
            let (idx, r) = rs.find_matched(&req, Some(&strategy), 0).expect("match");
            assert_eq!(r.text.as_deref(), Some("response_0"));
            assert_eq!(idx, 0);
        }
    }

    // -----------------------------------------------------------------
    // RFC 070 — round_robin rotates per match group, not per rule set.
    // -----------------------------------------------------------------

    /// Build a `RuleSet` from several `(url_path, rule_count)` groups —
    /// each group's rules match only that exact `url_path`, exclusively
    /// (RFC 070's reported scenario needs at least two groups that never
    /// both match the same request, which is exactly what distinct
    /// `url_path`s give for free). Responses are named `"<label><n>"`
    /// where `label` is the group's own url_path with the leading `/`
    /// stripped and `n` is 1-based *within that group* — `a1 a2` for
    /// `("/a", 2)`, matching the RFC's own example naming exactly.
    fn make_multi_group_round_robin_set(groups: &[(&str, usize)]) -> RuleSet {
        use crate::rule_set::rule::when::request::url_path::{UrlPath, UrlPathConfig};

        let mut rules = Vec::new();
        for (url_path, count) in groups {
            let label = url_path.trim_start_matches('/');
            for i in 1..=*count {
                rules.push(Rule {
                    when: When {
                        request: Request {
                            url_path_config: Some(UrlPathConfig::Simple((*url_path).to_owned())),
                            url_path: Some(UrlPath {
                                value: (*url_path).to_owned(),
                                value_with_prefix: (*url_path).to_owned(),
                                op: None,
                            }),
                            http_method: None,
                            headers: None,
                            body: None,
                        },
                    },
                    respond: Respond {
                        text: Some(format!("{label}{i}")),
                        file_path: None,
                        csv_records_key: None,
                        json: None,
                        status: None,
                        status_code: None,
                        headers: None,
                        delay_response_milliseconds: None,
                    },
                    weight: None,
                    priority: None,
                });
            }
        }

        RuleSet {
            prefix: None,
            default: None,
            guard: None,
            rules,
            strategy: None,
            file_path: String::new(),
            round_robin_counters: default_round_robin_counters(),
            resolved_respond_dir: ".".to_owned(),
        }
    }

    fn respond_text(rs: &RuleSet, strategy: &Strategy, url_path: &str) -> String {
        rs.find_matched(&get_req(url_path), Some(strategy), 0)
            .expect("request matches this fixture's rules")
            .1
            .text
            .clone()
            .expect("fixture rules always set text")
    }

    /// The RFC's own reported scenario, reproduced exactly: two groups
    /// of size 2 and 3, requested alternately. Before this fix, `/a`
    /// returned `a1` on every request — the shared counter only ever
    /// landed on an even (`/a`) or odd (`/b`-biased) value depending on
    /// interleaving, and for this exact shape `/a` never rotated at all.
    #[test]
    fn round_robin_alternating_two_groups_each_rotate_independently() {
        let rs = make_multi_group_round_robin_set(&[("/a", 2), ("/b", 3)]);
        let strategy = Strategy::RoundRobin;

        let mut a_seen = Vec::new();
        let mut b_seen = Vec::new();
        for _ in 0..4 {
            a_seen.push(respond_text(&rs, &strategy, "/a"));
            b_seen.push(respond_text(&rs, &strategy, "/b"));
        }

        assert_eq!(
            a_seen,
            vec!["a1", "a2", "a1", "a2"],
            "group /a must rotate through its own 2 rules independently of /b"
        );
        assert_eq!(
            b_seen,
            vec!["b1", "b2", "b3", "b1"],
            "group /b must rotate through its own 3 rules independently of /a"
        );
    }

    /// Single group, unchanged — the per-group implementation must not
    /// regress the case that already worked (RFC 070's own acceptance
    /// bullet, stated as its own scenario rather than folded into the
    /// pre-existing single-group tests above, since those predate the
    /// fix and this one is here specifically to keep meaning "the fix
    /// didn't break the base case" even if those get edited later).
    #[test]
    fn round_robin_single_group_still_a1_a2_a1_a2() {
        let rs = make_multi_group_round_robin_set(&[("/a", 2)]);
        let strategy = Strategy::RoundRobin;

        let seen: Vec<String> = (0..4).map(|_| respond_text(&rs, &strategy, "/a")).collect();

        assert_eq!(seen, vec!["a1", "a2", "a1", "a2"]);
    }

    /// Three or more groups, interleaved — not just the reported
    /// two-group case, since the defect's arithmetic (`counter %
    /// matches.len()`) could plausibly still misbehave for some N even
    /// if two groups happened to look right after a narrower fix.
    #[test]
    fn round_robin_three_groups_interleaved_each_rotate_independently() {
        let rs = make_multi_group_round_robin_set(&[("/a", 2), ("/b", 3), ("/c", 4)]);
        let strategy = Strategy::RoundRobin;

        let mut a_seen = Vec::new();
        let mut b_seen = Vec::new();
        let mut c_seen = Vec::new();
        for _ in 0..5 {
            a_seen.push(respond_text(&rs, &strategy, "/a"));
            b_seen.push(respond_text(&rs, &strategy, "/b"));
            c_seen.push(respond_text(&rs, &strategy, "/c"));
        }

        assert_eq!(a_seen, vec!["a1", "a2", "a1", "a2", "a1"]);
        assert_eq!(b_seen, vec!["b1", "b2", "b3", "b1", "b2"]);
        assert_eq!(c_seen, vec!["c1", "c2", "c3", "c4", "c1"]);
    }

    /// RFC 070 § Design's open question, established rather than
    /// assumed: the per-group counter map's size is bounded by the rule
    /// set's own structure, not by how many requests arrive. A fixed
    /// 3-group rule set, driven by far more requests than there are
    /// groups (and in an order designed to touch every group many
    /// times over, not just once each), must still end with exactly 3
    /// map entries — never more.
    #[test]
    fn round_robin_map_size_does_not_grow_with_request_volume() {
        let rs = make_multi_group_round_robin_set(&[("/a", 2), ("/b", 3), ("/c", 4)]);
        let strategy = Strategy::RoundRobin;
        let paths = ["/a", "/b", "/c"];

        for i in 0..300 {
            let _ = respond_text(&rs, &strategy, paths[i % paths.len()]);
        }

        let map_size = rs
            .round_robin_counters
            .lock()
            .expect("counters mutex not poisoned")
            .len();
        assert_eq!(
            map_size, 3,
            "300 requests across a fixed 3-group rule set must still produce exactly 3 counter \
             entries, not one that grows with request count"
        );
    }

    // -----------------------------------------------------------------
    // RFC 058 — `respond_dir_prefix` holds only what was authored;
    // `resolved_respond_dir` (never persisted) is what the matcher uses.
    // -----------------------------------------------------------------

    fn write_rule_set(dir: &std::path::Path, content: &str) -> String {
        let path = dir.join("rules.toml");
        std::fs::write(&path, content).unwrap();
        path.to_str().unwrap().to_owned()
    }

    #[test]
    fn is_purely_current_dir_matches_only_dot_segments() {
        assert!(is_purely_current_dir("."));
        assert!(is_purely_current_dir("./."));
        assert!(is_purely_current_dir("././."));
        assert!(is_purely_current_dir("./././."));
        assert!(!is_purely_current_dir("./responses"));
        assert!(!is_purely_current_dir("responses"));
        assert!(!is_purely_current_dir(""));
    }

    #[test]
    fn authored_respond_dir_is_not_overwritten_by_the_resolved_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rule_set(
            dir.path(),
            "[prefix]\nrespond_dir = \"responses\"\n\n[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n",
        );
        let rs = RuleSet::new(&path, ".", 0).expect("load");

        assert_eq!(
            rs.prefix.as_ref().unwrap().respond_dir_prefix.as_deref(),
            Some("responses"),
            "the authored value must survive RuleSet::new unchanged"
        );
        assert_ne!(
            rs.dir_prefix(),
            "responses",
            "dir_prefix() must be the resolved value, not the authored one"
        );
        assert!(
            rs.dir_prefix().ends_with("responses"),
            "the resolved value must still be anchored on the authored one: {}",
            rs.dir_prefix()
        );
    }

    #[test]
    fn no_prefix_section_stays_none_after_new() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rule_set(
            dir.path(),
            "[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n",
        );
        let rs = RuleSet::new(&path, ".", 0).expect("load");

        assert!(
            rs.prefix.is_none(),
            "a rule set with no [prefix] section must not gain one from RuleSet::new (Goal 2)"
        );
        // RFC 061: dir_prefix() is built from a `Path` join and rendered
        // with the platform's own separator (`.\.` on Windows, `./.`
        // elsewhere) — normalise before comparing rather than hardcoding
        // one platform's separator.
        assert_eq!(
            rs.dir_prefix().replace('\\', "/"),
            "./.",
            "dir_prefix() must still resolve sensibly with no [prefix] at all — \
             \"./.\"  because Path::join doesn't normalise \".\".join(\".\"), same \
             as it always did for the never-authored case"
        );
    }

    #[test]
    fn pure_dot_respond_dir_collapses_to_a_single_dot() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rule_set(
            dir.path(),
            "[prefix]\nrespond_dir = \"././.\"\n\n[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n",
        );
        let rs = RuleSet::new(&path, ".", 0).expect("load");

        assert_eq!(
            rs.prefix.as_ref().unwrap().respond_dir_prefix.as_deref(),
            Some("."),
            "a purely-./-segments respond_dir must collapse to a single '.'"
        );
    }

    #[test]
    fn non_dot_respond_dir_is_never_collapsed() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rule_set(
            dir.path(),
            "[prefix]\nrespond_dir = \"./responses\"\n\n[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n",
        );
        let rs = RuleSet::new(&path, ".", 0).expect("load");

        assert_eq!(
            rs.prefix.as_ref().unwrap().respond_dir_prefix.as_deref(),
            Some("./responses"),
            "a respond_dir with a real path component must never be touched by the collapse"
        );
    }

    #[test]
    fn prefix_validate_checks_the_resolved_directory_not_the_authored_one() {
        let dir = tempfile::tempdir().unwrap();
        let responses = dir.path().join("responses");
        std::fs::create_dir(&responses).unwrap();

        // `current_dir_to_config_dir_relative_path` must anchor at the
        // tempdir itself, not "." (which would resolve relative to
        // whatever directory `cargo test` actually runs from) — the
        // resolution this whole RFC is about only means anything
        // relative to *some* real base directory.
        let base = dir.path().to_str().unwrap();

        let ok_path = write_rule_set(
            dir.path(),
            "[prefix]\nrespond_dir = \"responses\"\n\n[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n",
        );
        let ok = RuleSet::new(&ok_path, base, 0).expect("load");
        assert!(
            ok.prefix
                .as_ref()
                .unwrap()
                .validate(ok.dir_prefix().as_str(), 0),
            "an existing resolved directory must validate"
        );

        let missing_dir_path = dir.path().join("missing_rules.toml");
        std::fs::write(
            &missing_dir_path,
            "[prefix]\nrespond_dir = \"does-not-exist\"\n\n[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n",
        )
        .unwrap();
        let missing = RuleSet::new(missing_dir_path.to_str().unwrap(), base, 0).expect("load");
        assert!(
            !missing
                .prefix
                .as_ref()
                .unwrap()
                .validate(missing.dir_prefix().as_str(), 0),
            "a resolved directory that doesn't exist on disk must fail validation, same as before RFC 058"
        );
    }
}
