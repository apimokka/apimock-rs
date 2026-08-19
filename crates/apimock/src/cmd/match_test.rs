//! `apimock match-test` — dry-run rule matching against a synthetic request.
//!
//! # Usage
//!
//! ```text
//! apimock match-test \
//!   --rule-set apimock-rule-set.toml \
//!   [--rule <1-based index>] \
//!   [--path /api/orders] \
//!   [--method POST] \
//!   [--header "Content-Type: application/json"] \
//!   [--body '{"action":"create"}'] \
//!   [--body-file request.json] \
//!   [--quiet]
//! ```
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0 | At least one rule matched (or the specified rule matched) |
//! | 1 | No rule matched |
//! | 2 | Error (file not found, invalid JSON, etc.) |

use std::process;

use anyhow::{Context, Result as AppResult};
use hyper::Request as HyperRequest;

use apimock_routing::parsed_request::ParsedRequest;
use apimock_routing::rule_set::RuleSet;

// ── CLI flag names ────────────────────────────────────────────────────

const RULE_SET_NAMES: &[&str] = &["--rule-set", "-r"];
const RULE_NAMES: &[&str] = &["--rule"];
const PATH_NAMES: &[&str] = &["--path", "-p"];
const METHOD_NAMES: &[&str] = &["--method", "-m"];
const HEADER_NAMES: &[&str] = &["--header", "-H"];
const BODY_NAMES: &[&str] = &["--body", "-b"];
const BODY_FILE_NAMES: &[&str] = &["--body-file"];
const QUIET_NAMES: &[&str] = &["--quiet", "-q"];

// ── Entry point ───────────────────────────────────────────────────────

/// Run `match-test` from the raw argument slice that follows the
/// `match-test` token (i.e. `env::args()[2..]`).
///
/// Calls `process::exit` on completion; only propagates an error if
/// argument parsing or I/O itself fails before we can begin matching.
pub fn run(raw_args: &[String]) -> AppResult<()> {
    let args = MatchTestArgs::parse(raw_args)?;

    let rule_set = RuleSet::new(&args.rule_set, "", 0)
        .with_context(|| format!("failed to load rule set: {}", args.rule_set))?;

    let body_json = args.body_json()?;
    let parsed = build_parsed_request(&args, body_json)?;

    let code = run_match(&rule_set, &parsed, args.rule_index, args.quiet);
    process::exit(code);
}

// ── Argument model ────────────────────────────────────────────────────

struct MatchTestArgs {
    rule_set: String,
    /// 0-based rule index (converted from CLI 1-based).
    rule_index: Option<usize>,
    path: String,
    method: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    body_file: Option<String>,
    quiet: bool,
}

impl MatchTestArgs {
    fn parse(args: &[String]) -> AppResult<Self> {
        let rule_set = flag_value(args, RULE_SET_NAMES)
            .ok_or_else(|| anyhow::anyhow!("--rule-set <path> is required"))?;

        let rule_index: Option<usize> = if let Some(s) = flag_value(args, RULE_NAMES) {
            let n: usize = s
                .parse()
                .with_context(|| format!("--rule must be a positive integer, got: {}", s))?;
            if n == 0 {
                anyhow::bail!("--rule is 1-based; use 1 for the first rule");
            }
            Some(n - 1)
        } else {
            None
        };

        let path = flag_value(args, PATH_NAMES).unwrap_or_else(|| "/".to_owned());
        let method = flag_value(args, METHOD_NAMES)
            .unwrap_or_else(|| "GET".to_owned())
            .to_uppercase();

        let headers = flag_values_all(args, HEADER_NAMES)
            .into_iter()
            .filter_map(|h| {
                let idx = h.find(':')?;
                let name = h[..idx].trim().to_lowercase();
                let value = h[idx + 1..].trim().to_owned();
                Some((name, value))
            })
            .collect();

        let body = flag_value(args, BODY_NAMES);
        let body_file = flag_value(args, BODY_FILE_NAMES);
        let quiet = flag_present(args, QUIET_NAMES);

        Ok(Self {
            rule_set,
            rule_index,
            path,
            method,
            headers,
            body,
            body_file,
            quiet,
        })
    }

    fn body_json(&self) -> AppResult<Option<serde_json::Value>> {
        if let Some(s) = &self.body {
            let v: serde_json::Value = serde_json::from_str(s)
                .with_context(|| format!("--body is not valid JSON: {}", s))?;
            return Ok(Some(v));
        }
        if let Some(p) = &self.body_file {
            let content = std::fs::read_to_string(p)
                .with_context(|| format!("cannot read --body-file: {}", p))?;
            let v: serde_json::Value = serde_json::from_str(&content)
                .with_context(|| format!("--body-file {} is not valid JSON", p))?;
            return Ok(Some(v));
        }
        Ok(None)
    }
}

// ── Request builder ───────────────────────────────────────────────────

fn build_parsed_request(
    args: &MatchTestArgs,
    body_json: Option<serde_json::Value>,
) -> AppResult<ParsedRequest> {
    let mut builder = HyperRequest::builder()
        .method(args.method.as_str())
        .uri(&args.path);

    for (name, value) in &args.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }

    let (parts, _) = builder
        .body(())
        .with_context(|| format!("invalid URI or headers for path {}", args.path))?
        .into_parts();

    // No real body-collection step here (`--body`/`--body-file` go
    // straight to parsed JSON), and `match-test` never reaches the
    // trace channel — `body_len`'s only consumer — so there is nothing
    // meaningful to report (RFC 050); hence `None` even though a body
    // may be attached.
    Ok(ParsedRequest::new(args.path.clone(), parts).with_body(body_json, None))
}

// ── Match runner ──────────────────────────────────────────────────────

/// Returns the process exit code: 0 = match, 1 = no match, 2 = error.
fn run_match(
    rule_set: &RuleSet,
    parsed: &ParsedRequest,
    rule_index: Option<usize>,
    quiet: bool,
) -> i32 {
    let rules_to_check: Vec<(usize, &apimock_routing::rule_set::rule::Rule)> = match rule_index {
        Some(idx) if idx >= rule_set.rules.len() => {
            eprintln!(
                "error: rule #{} does not exist (rule set has {} rules)",
                idx + 1,
                rule_set.rules.len()
            );
            return 2;
        }
        Some(idx) => vec![(idx, &rule_set.rules[idx])],
        None => rule_set.rules.iter().enumerate().collect(),
    };

    let mut first_match: Option<usize> = None;

    for (idx, rule) in &rules_to_check {
        let matched = rule.when.is_match(parsed, *idx, 0);
        if matched && first_match.is_none() {
            first_match = Some(*idx);
        }
        if !quiet {
            print_rule_result(*idx, rule, parsed, matched, first_match == Some(*idx));
        }
    }

    if !quiet {
        println!();
        match first_match {
            Some(w) => println!("Result: MATCH (rule #{})", w + 1),
            None => println!("Result: NO MATCH"),
        }
    }

    if first_match.is_some() { 0 } else { 1 }
}

// ── Per-rule output ───────────────────────────────────────────────────

/// Prints each condition's `legacy_text` from the shared evaluator
/// (`super::rule_check`) — byte-identical to what this function printed
/// before RFC 055 factored the checks out for `get --why` to reuse.
fn print_rule_result(
    idx: usize,
    rule: &apimock_routing::rule_set::rule::Rule,
    parsed: &ParsedRequest,
    matched: bool,
    is_winner: bool,
) {
    use apimock_routing::rule_set::rule::when::request::url_path::UrlPathConfig;

    let winner = if is_winner { " ★" } else { "" };
    let tag = if matched { "MATCH" } else { "NO MATCH" };
    let req = &rule.when.request;

    let label = match req.url_path_config.as_ref() {
        Some(UrlPathConfig::Simple(p)) => p.clone(),
        Some(UrlPathConfig::Detailed(u)) => u.value.clone(),
        None => "(any path)".to_owned(),
    };
    println!("\nRule #{}: {}  {}{}", idx + 1, label, tag, winner);

    for check in super::rule_check::evaluate_rule(rule, parsed) {
        println!("  {}  {}", tick(check.matched), check.legacy_text);
    }
}

fn tick(ok: bool) -> &'static str {
    if ok { "✓" } else { "✗" }
}

// ── CLI parsing helpers ───────────────────────────────────────────────

use super::flags::{flag_present, flag_value, flag_values_all};

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use apimock_routing::rule_set::RuleSet;
    use hyper::Request as HyperRequest;

    fn make_rule_set(toml: &str) -> RuleSet {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rs.toml");
        std::fs::write(&path, toml).unwrap();
        RuleSet::new(path.to_str().unwrap(), "", 0).expect("parse ok")
    }

    fn req(path: &str, method: &str, body: Option<serde_json::Value>) -> ParsedRequest {
        let (parts, _) = HyperRequest::builder()
            .method(method)
            .uri(path)
            .body(())
            .unwrap()
            .into_parts();
        ParsedRequest::new(path.to_owned(), parts).with_body(body, None)
    }

    #[test]
    fn match_simple_path() {
        let rs =
            make_rule_set("[[rules]]\nwhen.request.url_path = \"/api\"\nrespond.text = \"ok\"\n");
        assert_eq!(run_match(&rs, &req("/api", "GET", None), None, true), 0);
    }

    #[test]
    fn no_match_wrong_path() {
        let rs =
            make_rule_set("[[rules]]\nwhen.request.url_path = \"/api\"\nrespond.text = \"ok\"\n");
        assert_eq!(run_match(&rs, &req("/other", "GET", None), None, true), 1);
    }

    #[test]
    fn specific_rule_index_match() {
        let toml = concat!(
            "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"a\"\n",
            "[[rules]]\nwhen.request.url_path = \"/b\"\nrespond.text = \"b\"\n",
        );
        let rs = make_rule_set(toml);
        assert_eq!(run_match(&rs, &req("/b", "GET", None), Some(1), true), 0);
        assert_eq!(run_match(&rs, &req("/a", "GET", None), Some(1), true), 1);
    }

    #[test]
    fn out_of_range_rule_index_returns_2() {
        let rs = make_rule_set("[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"a\"\n");
        assert_eq!(run_match(&rs, &req("/a", "GET", None), Some(99), true), 2);
    }

    #[test]
    fn flag_value_parses_correctly() {
        let args: Vec<String> = ["--rule-set", "foo.toml", "--path", "/api"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            flag_value(&args, RULE_SET_NAMES).as_deref(),
            Some("foo.toml")
        );
        assert_eq!(flag_value(&args, PATH_NAMES).as_deref(), Some("/api"));
        assert_eq!(flag_value(&args, METHOD_NAMES), None);
    }

    #[test]
    fn flag_values_all_collects_multiple() {
        let args: Vec<String> = [
            "--header",
            "Content-Type: application/json",
            "--header",
            "X-Api-Key: secret",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let vals = flag_values_all(&args, HEADER_NAMES);
        assert_eq!(vals.len(), 2);
        assert!(vals[0].contains("Content-Type"));
    }

    #[test]
    fn parse_args_requires_rule_set() {
        let args: Vec<String> = vec!["--path".to_owned(), "/api".to_owned()];
        assert!(MatchTestArgs::parse(&args).is_err());
    }

    #[test]
    fn body_json_match() {
        let rs = make_rule_set(concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/api\"\n",
            "when.request.body.json.\"action\" = { op = \"equal\", value = \"create\" }\n",
            "respond.text = \"ok\"\n",
        ));
        let body = serde_json::json!({"action": "create"});
        assert_eq!(
            run_match(&rs, &req("/api", "POST", Some(body)), None, true),
            0
        );
        let bad = serde_json::json!({"action": "delete"});
        assert_eq!(
            run_match(&rs, &req("/api", "POST", Some(bad)), None, true),
            1
        );
    }
}
