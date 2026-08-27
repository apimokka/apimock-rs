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
//!   [--quiet] \
//!   [--format text|json]
//! ```
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0 | At least one rule matched (or the specified rule matched) |
//! | 1 | No rule matched |
//! | 2 | Error (unrecognised flag, file not found, invalid JSON, etc.) |
//!
//! # `--format json` (RFC 059)
//!
//! `match-test` was the one command outside RFC 053's envelope — no
//! `--format` support at all, so an agent driving it had to scrape text.
//! Added additively: text stays the default and is byte-identical to
//! before; `--format json` emits the same `{schema, apimock, result}` /
//! `{schema, apimock, error}` envelope `get` and `validate` already do,
//! via the same `envelope` helper.

use anyhow::Context;
use hyper::Request as HyperRequest;

use apimock_routing::parsed_request::ParsedRequest;
use apimock_routing::rule_set::RuleSet;

use super::envelope::{self, ErrorKind, Format};
use super::flags::reject_unknown_flags;

// ── CLI flag names ────────────────────────────────────────────────────

const RULE_SET_NAMES: &[&str] = &["--rule-set", "-r"];
const RULE_NAMES: &[&str] = &["--rule"];
const PATH_NAMES: &[&str] = &["--path", "-p"];
const METHOD_NAMES: &[&str] = &["--method", "-m"];
const HEADER_NAMES: &[&str] = &["--header", "-H"];
const BODY_NAMES: &[&str] = &["--body", "-b"];
const BODY_FILE_NAMES: &[&str] = &["--body-file"];
const QUIET_NAMES: &[&str] = &["--quiet", "-q"];
const FORMAT_FLAG: &str = "--format";
/// Flags that take no value — every other known flag does.
const NO_VALUE_FLAG_NAMES: &[&str] = QUIET_NAMES;

fn known_flag_names() -> Vec<&'static str> {
    [
        RULE_SET_NAMES,
        RULE_NAMES,
        PATH_NAMES,
        METHOD_NAMES,
        HEADER_NAMES,
        BODY_NAMES,
        BODY_FILE_NAMES,
        QUIET_NAMES,
    ]
    .into_iter()
    .flatten()
    .copied()
    .chain([FORMAT_FLAG])
    .collect()
}

// ── Entry point ───────────────────────────────────────────────────────

const USAGE: &str = "Usage: apimock match-test --rule-set <path> [--rule <n>] [--path <url_path>] [--method <METHOD>] [--header \"Name: value\"]... [--body <json>|--body-file <path>] [--quiet] [--format text|json]";

fn usage_error(message: &str) -> i32 {
    eprintln!("apimock match-test: {}", message);
    eprintln!("{}", USAGE);
    2
}

/// A failure after `--format` is known: enveloped under `--format json`,
/// plain text to stderr otherwise — the same split `get`/`validate` use
/// once their own parsing has succeeded far enough to know which.
fn fail(is_envelope: bool, kind: ErrorKind, message: String) -> i32 {
    if is_envelope {
        print_envelope(&envelope::err(kind, message));
    } else {
        eprintln!("apimock match-test: {}", message);
    }
    2
}

/// Run `match-test` from the raw argument slice that follows the
/// `match-test` token (i.e. `env::args()[2..]`). Returns the process
/// exit code — matches `get`/`validate`/`set`'s own convention, so the
/// caller in `args.rs` treats all four commands identically.
pub fn run(raw_args: &[String]) -> i32 {
    // RFC 059: rejected before `MatchTestArgs::parse` — mirrors
    // `get`/`validate`. Previously nothing here noticed an unrecognised
    // flag at all: `--bogus` reached `MatchTestArgs::parse`, matched
    // none of the known names, and was silently ignored — the missing
    // `--rule-set` in the RFC's own repro then failed for an unrelated
    // reason, propagated as `anyhow::Error` all the way to `main`,
    // surfacing as a generic exit 1 rather than a `usage` exit 2.
    if let Err(e) = reject_unknown_flags(raw_args, &known_flag_names(), NO_VALUE_FLAG_NAMES) {
        return usage_error(&e);
    }
    let args = match MatchTestArgs::parse(raw_args) {
        Ok(a) => a,
        Err(e) => return usage_error(&format!("{}", e)),
    };
    let format = args.format.unwrap_or(Format::Text);
    let is_envelope = format == Format::Json;

    let rule_set = match RuleSet::new(&args.rule_set, "", 0) {
        Ok(rs) => rs,
        Err(e) => {
            return fail(
                is_envelope,
                envelope::kind_for_routing_error(&e),
                format!("failed to load rule set {}: {}", args.rule_set, e),
            );
        }
    };

    if let Err(message) = check_rule_index_in_range(&rule_set, args.rule_index) {
        return fail(is_envelope, ErrorKind::Usage, message);
    }

    let body_json = match args.body_json() {
        Ok(b) => b,
        Err(e) => return fail(is_envelope, ErrorKind::Usage, format!("{}", e)),
    };
    let parsed = match build_parsed_request(&args, body_json) {
        Ok(p) => p,
        Err(e) => return fail(is_envelope, ErrorKind::Usage, format!("{}", e)),
    };

    let outcome = compute_outcome(&rule_set, &parsed, args.rule_index);

    if is_envelope {
        print_envelope(&envelope::ok(result_json(
            &args, &outcome, &rule_set, &parsed,
        )));
    } else if !args.quiet {
        print_text_outcome(&outcome, &rule_set, &parsed);
    }

    if outcome.first_match.is_some() { 0 } else { 1 }
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
    format: Option<Format>,
}

impl MatchTestArgs {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        // A dangling `--rule-set` (no value) and an absent one both mean
        // "no rule set was given" from a required flag's point of view —
        // the same message applies to both (RFC 064: this is the one
        // place a required flag's dangling form must keep saying
        // exactly what it already said, not `flag_value`'s new generic
        // "requires a value").
        let rule_set = match flag_value(args, RULE_SET_NAMES) {
            Ok(Some(v)) => v,
            Ok(None) | Err(_) => {
                return Err(anyhow::anyhow!("--rule-set <path> is required"));
            }
        };

        let rule_index: Option<usize> =
            if let Some(s) = flag_value(args, RULE_NAMES).map_err(|e| anyhow::anyhow!(e))? {
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

        let path = flag_value(args, PATH_NAMES)
            .map_err(|e| anyhow::anyhow!(e))?
            .unwrap_or_else(|| "/".to_owned());
        let method = flag_value(args, METHOD_NAMES)
            .map_err(|e| anyhow::anyhow!(e))?
            .unwrap_or_else(|| "GET".to_owned())
            .to_uppercase();

        let headers = flag_values_all(args, HEADER_NAMES)
            .map_err(|e| anyhow::anyhow!(e))?
            .into_iter()
            .filter_map(|h| {
                let idx = h.find(':')?;
                let name = h[..idx].trim().to_lowercase();
                let value = h[idx + 1..].trim().to_owned();
                Some((name, value))
            })
            .collect();

        let body = flag_value(args, BODY_NAMES).map_err(|e| anyhow::anyhow!(e))?;
        let body_file = flag_value(args, BODY_FILE_NAMES).map_err(|e| anyhow::anyhow!(e))?;
        let quiet = flag_present(args, QUIET_NAMES);

        let format = match flag_value(args, &[FORMAT_FLAG])
            .map_err(|e| anyhow::anyhow!(e))?
            .as_deref()
        {
            None => None,
            Some("text") => Some(Format::Text),
            Some("json") => Some(Format::Json),
            Some(other) => {
                anyhow::bail!(
                    "invalid value for --format: '{}' (expected 'text' or 'json')",
                    other
                );
            }
        };

        Ok(Self {
            rule_set,
            rule_index,
            path,
            method,
            headers,
            body,
            body_file,
            quiet,
            format,
        })
    }

    fn body_json(&self) -> anyhow::Result<Option<serde_json::Value>> {
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
) -> anyhow::Result<ParsedRequest> {
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

/// One rule's outcome, in the order it was checked.
struct RuleOutcome {
    idx: usize,
    matched: bool,
}

/// The full result of checking a request against a rule set — computed
/// once, then either printed as text or serialised to JSON. `run`
/// already validated `rule_index` is in range before calling this, same
/// as the check `run_match` used to make inline.
struct MatchOutcome {
    rules: Vec<RuleOutcome>,
    first_match: Option<usize>,
}

/// `rule_index` (if given) must name a real rule — checked ahead of
/// `compute_outcome`, which indexes `rule_set.rules` directly and
/// assumes a valid index rather than re-checking it.
fn check_rule_index_in_range(rule_set: &RuleSet, rule_index: Option<usize>) -> Result<(), String> {
    if let Some(idx) = rule_index
        && idx >= rule_set.rules.len()
    {
        return Err(format!(
            "rule #{} does not exist (rule set has {} rules)",
            idx + 1,
            rule_set.rules.len()
        ));
    }
    Ok(())
}

fn compute_outcome(
    rule_set: &RuleSet,
    parsed: &ParsedRequest,
    rule_index: Option<usize>,
) -> MatchOutcome {
    let rules_to_check: Vec<(usize, &apimock_routing::rule_set::rule::Rule)> = match rule_index {
        Some(idx) => vec![(idx, &rule_set.rules[idx])],
        None => rule_set.rules.iter().enumerate().collect(),
    };

    let mut first_match: Option<usize> = None;
    let mut rules = Vec::with_capacity(rules_to_check.len());
    for (idx, rule) in &rules_to_check {
        let matched = rule.when.is_match(parsed, *idx, 0);
        if matched && first_match.is_none() {
            first_match = Some(*idx);
        }
        rules.push(RuleOutcome { idx: *idx, matched });
    }

    MatchOutcome { rules, first_match }
}

/// Byte-identical to `run_match`'s own printing before RFC 059 split
/// computing the outcome from printing it.
fn print_text_outcome(outcome: &MatchOutcome, rule_set: &RuleSet, parsed: &ParsedRequest) {
    for r in &outcome.rules {
        let rule = &rule_set.rules[r.idx];
        let is_winner = outcome.first_match == Some(r.idx);
        print_rule_result(r.idx, rule, parsed, r.matched, is_winner);
    }

    println!();
    match outcome.first_match {
        Some(w) => println!("Result: MATCH (rule #{})", w + 1),
        None => println!("Result: NO MATCH"),
    }
}

fn result_json(
    args: &MatchTestArgs,
    outcome: &MatchOutcome,
    rule_set: &RuleSet,
    parsed: &ParsedRequest,
) -> serde_json::Value {
    let rules: Vec<serde_json::Value> = outcome
        .rules
        .iter()
        .map(|r| {
            let rule = &rule_set.rules[r.idx];
            let conditions = super::rule_check::evaluate_rule(rule, parsed);
            serde_json::json!({
                "rule_index": r.idx,
                "matched": r.matched,
                "conditions": conditions.iter().map(|c| serde_json::json!({
                    "name": c.name,
                    "expectation": c.expectation,
                    "actual": c.actual,
                    "matched": c.matched,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    serde_json::json!({
        "request": { "method": args.method, "path": args.path },
        "rules": rules,
        "matched": outcome.first_match.is_some(),
        "match_rule_index": outcome.first_match,
    })
}

fn print_envelope(v: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
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
        let outcome = compute_outcome(&rs, &req("/api", "GET", None), None);
        assert!(outcome.first_match.is_some());
    }

    #[test]
    fn no_match_wrong_path() {
        let rs =
            make_rule_set("[[rules]]\nwhen.request.url_path = \"/api\"\nrespond.text = \"ok\"\n");
        let outcome = compute_outcome(&rs, &req("/other", "GET", None), None);
        assert!(outcome.first_match.is_none());
    }

    #[test]
    fn specific_rule_index_match() {
        let toml = concat!(
            "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"a\"\n",
            "[[rules]]\nwhen.request.url_path = \"/b\"\nrespond.text = \"b\"\n",
        );
        let rs = make_rule_set(toml);
        assert!(
            compute_outcome(&rs, &req("/b", "GET", None), Some(1))
                .first_match
                .is_some()
        );
        assert!(
            compute_outcome(&rs, &req("/a", "GET", None), Some(1))
                .first_match
                .is_none()
        );
    }

    #[test]
    fn out_of_range_rule_index_is_rejected() {
        let rs = make_rule_set("[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"a\"\n");
        assert!(check_rule_index_in_range(&rs, Some(99)).is_err());
        assert!(check_rule_index_in_range(&rs, Some(0)).is_ok());
        assert!(check_rule_index_in_range(&rs, None).is_ok());
    }

    #[test]
    fn flag_value_parses_correctly() {
        let args: Vec<String> = ["--rule-set", "foo.toml", "--path", "/api"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            flag_value(&args, RULE_SET_NAMES).unwrap().as_deref(),
            Some("foo.toml")
        );
        assert_eq!(
            flag_value(&args, PATH_NAMES).unwrap().as_deref(),
            Some("/api")
        );
        assert_eq!(flag_value(&args, METHOD_NAMES).unwrap(), None);
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
        let vals = flag_values_all(&args, HEADER_NAMES).unwrap();
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
        assert!(
            compute_outcome(&rs, &req("/api", "POST", Some(body)), None)
                .first_match
                .is_some()
        );
        let bad = serde_json::json!({"action": "delete"});
        assert!(
            compute_outcome(&rs, &req("/api", "POST", Some(bad)), None)
                .first_match
                .is_none()
        );
    }
}
