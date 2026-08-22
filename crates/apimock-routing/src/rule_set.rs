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
        // - round-robin counter (starts at 0; shared across clones via Arc)
        ret.round_robin_counter = Arc::new(AtomicUsize::new(0));

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

                // Relaxed ordering: atomicity without sequential consistency
                // is sufficient for a mock server (slight counter reorder
                // on concurrent requests is acceptable).
                let idx = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % matches.len();

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
