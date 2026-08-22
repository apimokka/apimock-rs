//! `apimock get` — what would the server return for this request?
//! (RFC 055)
//!
//! # One implementation of matching
//!
//! This command calls the exact functions `apimock_server::server::service`
//! calls — `handle_options`, `rule_set_response`'s underlying
//! `RuleSet::find_matched` + `respond_response`, and `dyn_route_content`
//! — on a `ParsedRequest` built the same way the server's own
//! `parsed_request_from` builds one (including `normalize_url_path`,
//! which `match-test`'s synthetic request does **not** apply — a
//! pre-existing, out-of-scope gap in `match-test` this command does not
//! inherit, because "the answer is true" is this command's entire
//! reason to exist).
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! |---|---|
//! | 0 | Answered — including a 404 or "no rule matched"; per RFC 053, that is a legitimate result, not a failure |
//! | 2 | Bad invocation, or the configuration couldn't be loaded |

use std::path::Path;

use apimock_config::Config;
use apimock_routing::ParsedRequest;
use apimock_routing::util::http::normalize_url_path;
use apimock_server::dyn_route::dyn_route_content;
use apimock_server::respond_response::respond_response;
use apimock_server::response::confine::canonical_dir;
use apimock_server::server::handle_options;
use apimock_server::types::CollectedResponse;

use super::envelope::{self, Format};
use super::flags::{flag_present, flag_value, flag_values_all, reject_unknown_flags};
use super::rule_check::{ConditionCheck, evaluate_rule};

const CONFIG_NAMES: &[&str] = &["--config", "-c"];
const METHOD_NAMES: &[&str] = &["--method", "-m"];
const HEADER_NAMES: &[&str] = &["--header", "-H"];
const BODY_NAMES: &[&str] = &["--body", "-b"];
const BODY_FILE_NAMES: &[&str] = &["--body-file"];
const WHY_FLAG: &str = "--why";
const FORMAT_FLAG: &str = "--format";
const DEFAULT_CONFIG_FILE_PATH: &str = "./apimock.toml";
/// Flags that take no value — every other known flag does.
const NO_VALUE_FLAG_NAMES: &[&str] = &[WHY_FLAG];

fn known_flag_names() -> Vec<&'static str> {
    [
        CONFIG_NAMES,
        METHOD_NAMES,
        HEADER_NAMES,
        BODY_NAMES,
        BODY_FILE_NAMES,
    ]
    .into_iter()
    .flatten()
    .copied()
    .chain([WHY_FLAG, FORMAT_FLAG])
    .collect()
}

// ── Argument model ──────────────────────────────────────────────────────

struct GetArgs {
    path: String,
    config_path: Option<String>,
    method: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    body_file: Option<String>,
    why: Option<bool>,
    format: Option<Format>,
}

impl GetArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        // The path is positional — the first token that isn't itself a
        // flag's name or a flag's value. Every other argument here is
        // named, so the simplest correct rule is "the first token not
        // preceded by a flag name that consumes it".
        let path =
            positional_path(args).ok_or_else(|| "missing required argument <path>".to_owned())?;

        let config_path = flag_value(args, CONFIG_NAMES);
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

        let why_flag_present = flag_present(args, &[WHY_FLAG]);
        let why = if why_flag_present { Some(true) } else { None };

        let format_raw = flag_value(args, &[FORMAT_FLAG]);
        let format = match format_raw.as_deref() {
            None => None,
            Some("text") => Some(Format::Text),
            Some("json") => Some(Format::Json),
            Some(other) => {
                return Err(format!(
                    "invalid value for --format: '{}' (expected 'text' or 'json')",
                    other
                ));
            }
        };

        Ok(Self {
            path,
            config_path,
            method,
            headers,
            body,
            body_file,
            why,
            format,
        })
    }

    fn body_json(&self) -> Result<Option<serde_json::Value>, String> {
        if let Some(s) = &self.body {
            let v: serde_json::Value =
                serde_json::from_str(s).map_err(|e| format!("--body is not valid JSON: {}", e))?;
            return Ok(Some(v));
        }
        if let Some(p) = &self.body_file {
            let content = std::fs::read_to_string(p)
                .map_err(|e| format!("cannot read --body-file {}: {}", p, e))?;
            let v: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| format!("--body-file {} is not valid JSON: {}", p, e))?;
            return Ok(Some(v));
        }
        Ok(None)
    }

    /// Off in text, on in JSON (RFC 055 § 2 Q3) — U1 gets a clean answer
    /// and asks for `--why` when wanted; U2 gets the explanation without
    /// having to know to ask for it.
    fn why_enabled(&self, format: Format) -> bool {
        self.why.unwrap_or(format == Format::Json)
    }
}

/// The first argument that isn't a recognised flag's name or the value
/// consumed by one — `get`'s one positional argument.
fn positional_path(args: &[String]) -> Option<String> {
    let all_flag_names: &[&[&str]] = &[
        CONFIG_NAMES,
        METHOD_NAMES,
        HEADER_NAMES,
        BODY_NAMES,
        BODY_FILE_NAMES,
        &[WHY_FLAG],
        &[FORMAT_FLAG],
    ];
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if let Some(names) = all_flag_names
            .iter()
            .find(|names| names.contains(&arg.as_str()))
        {
            // `--why` takes no value; every other known flag does.
            skip_next = !std::ptr::eq(*names, &[WHY_FLAG]);
            continue;
        }
        return Some(arg.clone());
    }
    None
}

// ── Entry point ───────────────────────────────────────────────────────

const USAGE: &str = "Usage: apimock get <path> [-c <config>] [-m <METHOD>] [-H \"Name: value\"]... [-b <json>|--body-file <p>] [--why] [--format text|json]";

fn usage_error(message: &str) -> i32 {
    eprintln!("apimock get: {}", message);
    eprintln!("{}", USAGE);
    2
}

pub fn run(raw_args: &[String]) -> i32 {
    // RFC 059: rejected before `GetArgs::parse` even runs, same as
    // `set` — an unrecognised flag is a `usage` error, not something a
    // positional-argument scan should ever be asked to silently absorb.
    if let Err(e) = reject_unknown_flags(raw_args, &known_flag_names(), NO_VALUE_FLAG_NAMES) {
        return usage_error(&e);
    }
    let args = match GetArgs::parse(raw_args) {
        Ok(a) => a,
        Err(e) => return usage_error(&e),
    };
    let format = args.format.unwrap_or(Format::Text);
    let is_envelope = format == Format::Json;
    let why_enabled = args.why_enabled(format);

    let body_json = match args.body_json() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("apimock get: {}", e);
            return 2;
        }
    };

    let resolved_config_path = args.config_path.clone().or_else(|| {
        Path::new(DEFAULT_CONFIG_FILE_PATH)
            .exists()
            .then(|| DEFAULT_CONFIG_FILE_PATH.to_owned())
    });

    // Mirrors `apimock_config::Config::new`'s own resolution exactly —
    // `None` is zero-config, not an error, since `Config::init(None)`
    // returns `Config::default()`. `get` must answer for zero-config
    // workspaces the same way the server does (RFC 055's central trap).
    let config = match Config::new(resolved_config_path.as_ref(), None) {
        Ok(c) => c,
        Err(e) => {
            if is_envelope {
                print_envelope(&envelope::err(
                    envelope::kind_for_config_error(&e),
                    format!("failed to load config: {}", e),
                ));
            } else {
                eprintln!("apimock get: failed to load config: {}", e);
            }
            return 2;
        }
    };

    let url_path = normalize_url_path(args.path.as_str(), None);
    let parsed_request =
        match build_parsed_request(url_path.clone(), args.method.as_str(), &args.headers) {
            Ok(p) => p.with_body(body_json, None),
            Err(e) => {
                eprintln!("apimock get: {}", e);
                return 2;
            }
        };

    let DispatchOutcome {
        stage,
        response,
        middleware_configured,
    } = dispatch(&config, &parsed_request);
    let collected = tokio_run(CollectedResponse::collect(response));

    let why = why_enabled.then(|| explain(&config, &parsed_request, &stage));

    if is_envelope {
        print_envelope(&envelope::ok(result_json(
            &args,
            &config,
            &collected,
            &stage,
            middleware_configured,
            why.as_ref(),
        )));
    } else {
        print_text(
            &args,
            &collected,
            &stage,
            middleware_configured,
            why.as_ref(),
        );
    }

    0
}

/// `get::run` is a plain sync `fn`, dispatched from `EnvArgs::default()`
/// (also sync) — but that call happens from inside `main.rs`'s
/// `#[tokio::main] async fn main()`, i.e. **already on a Tokio worker
/// thread**. Building a second, nested runtime and calling `block_on`
/// on it panics ("Cannot start a runtime from within a runtime") —
/// established by hitting exactly that panic before writing this.
/// `block_in_place` + the *current* runtime's handle is the documented
/// way to block synchronously on async work from inside one, and it's
/// safe here because `#[tokio::main]`'s default flavour is
/// multi-threaded (the macro carries no `flavor = "current_thread"`
/// override in `main.rs`), so there's another worker thread to hand the
/// blocked one's remaining work to.
fn tokio_run<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

fn build_parsed_request(
    url_path: String,
    method: &str,
    headers: &[(String, String)],
) -> Result<ParsedRequest, String> {
    let mut builder = hyper::Request::builder()
        .method(method)
        .uri(url_path.as_str());
    for (name, value) in headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let (parts, _) = builder
        .body(())
        .map_err(|e| format!("invalid method or headers: {}", e))?
        .into_parts();
    Ok(ParsedRequest::new(url_path, parts))
}

// ── Dispatch (reusing the server's own functions) ───────────────────────

/// Which dispatch stage produced the answer, and enough detail from it
/// for `--why` and provenance — mirrors `server.rs`'s own
/// `OPTIONS → middleware → rule sets → dyn_route` order exactly (RFC 055
/// § 3's trap: zero-config is served entirely by the last stage).
enum Stage {
    Options,
    /// Middleware is configured; not simulated (RFC 055 § 2 Q1). Dispatch
    /// continues to the remaining stages regardless, so the answer is
    /// disclosed as incomplete rather than withheld.
    RuleSet {
        rule_set_index: usize,
        rule_index: usize,
        consulted: Vec<usize>,
    },
    DynRoute {
        consulted: Vec<usize>,
    },
}

struct DispatchOutcome {
    stage: Stage,
    response: hyper::Response<apimock_server::types::BoxBody>,
    middleware_configured: usize,
}

fn dispatch(config: &Config, parsed_request: &ParsedRequest) -> DispatchOutcome {
    let middleware_configured = config
        .service
        .middlewares_file_paths
        .as_deref()
        .unwrap_or(&[])
        .len();

    if parsed_request.component_parts.method == hyper::Method::OPTIONS {
        let response = handle_options(&parsed_request.component_parts.headers)
            .expect("a fixed 204 response cannot fail to build");
        return DispatchOutcome {
            stage: Stage::Options,
            response,
            middleware_configured,
        };
    }

    let mut consulted = Vec::new();
    for (rule_set_idx, rule_set) in config.service.rule_sets.iter().enumerate() {
        consulted.push(rule_set_idx);
        if let Some((rule_index, respond)) = rule_set.find_matched(
            parsed_request,
            config.service.strategy.as_ref(),
            rule_set_idx,
        ) {
            let dir_prefix = rule_set.dir_prefix();
            let rule_set_default_delay_ms = rule_set
                .default
                .as_ref()
                .and_then(|d| d.delay_response_milliseconds);
            let confine_to = canonical_dir(dir_prefix.as_str());
            let response = tokio_run(respond_response(
                &respond,
                dir_prefix.as_str(),
                parsed_request,
                rule_set_default_delay_ms,
                confine_to.as_deref(),
            ))
            .expect("respond_response only fails on a headers-construction bug");
            return DispatchOutcome {
                stage: Stage::RuleSet {
                    rule_set_index: rule_set_idx,
                    rule_index,
                    consulted,
                },
                response,
                middleware_configured,
            };
        }
    }

    let confine_to = canonical_dir(config.service.fallback_respond_dir.as_str());
    let response = tokio_run(dyn_route_content(
        parsed_request.url_path.as_str(),
        config.service.fallback_respond_dir.as_str(),
        &parsed_request.component_parts.headers,
        confine_to.as_deref(),
    ))
    .expect("dyn_route_content only fails on a headers-construction bug");
    DispatchOutcome {
        stage: Stage::DynRoute { consulted },
        response,
        middleware_configured,
    }
}

// ── `--why` ───────────────────────────────────────────────────────────

struct RuleSetExplanation {
    rule_set_index: usize,
    rule_set_file: String,
    rules: Vec<RuleExplanation>,
}

struct RuleExplanation {
    rule_index: usize,
    matched: bool,
    conditions: Vec<ConditionCheck>,
}

struct Explanation {
    stage_note: String,
    rule_sets: Vec<RuleSetExplanation>,
}

/// Re-derive, for every rule set dispatch actually consulted (`Stage`'s
/// `consulted` list — nothing beyond where dispatch stopped, since the
/// server itself never reaches those either), which of that rule set's
/// rules matched and why each condition held or didn't. Does not
/// re-decide the answer — `dispatch` already did that; this is
/// explanation only, using the same `RuleSet::find_matched`-visited
/// rules re-walked through `evaluate_rule` for their per-condition detail.
fn explain(config: &Config, parsed_request: &ParsedRequest, stage: &Stage) -> Explanation {
    let (stage_note, consulted): (String, &[usize]) = match stage {
        Stage::Options => (
            "OPTIONS request — handled directly, no rule-set matching involved.".to_owned(),
            &[],
        ),
        Stage::RuleSet { consulted, .. } => {
            ("Answered from a rule set.".to_owned(), consulted.as_slice())
        }
        Stage::DynRoute { consulted } => (
            "No rule set matched; answered from the fallback directory (dyn_route).".to_owned(),
            consulted.as_slice(),
        ),
    };

    let rule_sets = consulted
        .iter()
        .map(|&rule_set_idx| {
            let rule_set = &config.service.rule_sets[rule_set_idx];
            let rules = rule_set
                .rules
                .iter()
                .enumerate()
                .map(|(rule_index, rule)| RuleExplanation {
                    rule_index,
                    matched: rule.when.is_match(parsed_request, rule_index, rule_set_idx),
                    conditions: evaluate_rule(rule, parsed_request),
                })
                .collect();
            RuleSetExplanation {
                rule_set_index: rule_set_idx,
                rule_set_file: rule_set.file_path.clone(),
                rules,
            }
        })
        .collect();

    Explanation {
        stage_note,
        rule_sets,
    }
}

// ── Provenance ───────────────────────────────────────────────────────

fn absolute(path: &str) -> String {
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_owned())
}

fn source_json(config: &Config) -> serde_json::Value {
    let config_path = config
        .file_path
        .as_deref()
        .map(absolute)
        .unwrap_or_else(|| "(zero-config, no file)".to_owned());
    let rule_sets: Vec<String> = config
        .service
        .rule_sets
        .iter()
        .map(|rs| absolute(rs.file_path.as_str()))
        .collect();
    serde_json::json!({ "config": config_path, "rule_sets": rule_sets })
}

// ── Output ───────────────────────────────────────────────────────────

fn print_envelope(v: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

fn result_json(
    args: &GetArgs,
    config: &Config,
    collected: &CollectedResponse,
    stage: &Stage,
    middleware_configured: usize,
    why: Option<&Explanation>,
) -> serde_json::Value {
    let matched = match stage {
        Stage::RuleSet {
            rule_set_index,
            rule_index,
            ..
        } => {
            let rule_set_file = config
                .service
                .rule_sets
                .get(*rule_set_index)
                .map(|rs| rs.file_path.as_str())
                .unwrap_or_default();
            serde_json::json!({
                "rule_set_index": rule_set_index,
                "rule_set_file": rule_set_file,
                "rule_index": rule_index,
            })
        }
        Stage::Options | Stage::DynRoute { .. } => serde_json::Value::Null,
    };
    let stage_name = match stage {
        Stage::Options => "options",
        Stage::RuleSet { .. } => "rule_set",
        Stage::DynRoute { .. } => "dyn_route",
    };

    let mut result = serde_json::json!({
        "request": { "method": args.method, "path": args.path },
        "response": {
            "status": collected.status.as_u16(),
            "headers": collected.headers.iter().map(|(k, v)| serde_json::json!({"name": k, "value": v})).collect::<Vec<_>>(),
            "body": String::from_utf8_lossy(&collected.body),
        },
        "matched": matched,
        "stage": stage_name,
        "source": source_json(config),
    });

    if middleware_configured > 0 {
        result["middleware"] = serde_json::json!({
            "configured": middleware_configured,
            "simulated": false,
            "note": "middleware is configured and was NOT executed; if a middleware would intercept this request, the running server's answer may differ",
        });
    }

    if let Some(why) = why {
        result["why"] = why_json(why);
    }

    result
}

fn why_json(why: &Explanation) -> serde_json::Value {
    serde_json::json!({
        "note": why.stage_note,
        "rule_sets": why.rule_sets.iter().map(|rs| serde_json::json!({
            "rule_set_index": rs.rule_set_index,
            "rule_set_file": rs.rule_set_file,
            "rules": rs.rules.iter().map(|r| serde_json::json!({
                "rule_index": r.rule_index,
                "matched": r.matched,
                "conditions": r.conditions.iter().map(|c| serde_json::json!({
                    "name": c.name,
                    "expectation": c.expectation,
                    "actual": c.actual,
                    "matched": c.matched,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn print_text(
    args: &GetArgs,
    collected: &CollectedResponse,
    stage: &Stage,
    middleware_configured: usize,
    why: Option<&Explanation>,
) {
    println!("{} {}", args.method, args.path);
    println!();
    println!("Status: {}", collected.status.as_u16());
    if !collected.headers.is_empty() {
        println!("Headers:");
        for (k, v) in &collected.headers {
            println!("  {}: {}", k, v);
        }
    }
    println!("Body:");
    println!("{}", String::from_utf8_lossy(&collected.body));

    match stage {
        Stage::Options => println!("\nAnswered: OPTIONS (handled directly)"),
        Stage::RuleSet {
            rule_set_index,
            rule_index,
            ..
        } => println!(
            "\nAnswered: rule set #{}, rule #{}",
            rule_set_index + 1,
            rule_index + 1
        ),
        Stage::DynRoute { .. } => {
            println!("\nAnswered: fallback directory (no rule set matched)")
        }
    }

    if middleware_configured > 0 {
        println!(
            "\nNote: {} middleware handler(s) are configured and were NOT simulated; if one would intercept this request, the running server's answer may differ.",
            middleware_configured
        );
    }

    if let Some(why) = why {
        println!("\n-- Why --");
        println!("{}", why.stage_note);
        for rs in &why.rule_sets {
            println!(
                "\nRule set #{} ({}):",
                rs.rule_set_index + 1,
                rs.rule_set_file
            );
            for r in &rs.rules {
                let tag = if r.matched { "MATCH" } else { "NO MATCH" };
                println!("  Rule #{}: {}", r.rule_index + 1, tag);
                for c in &r.conditions {
                    let tick = if c.matched { "✓" } else { "✗" };
                    println!(
                        "    {}  {} {} (actual: {})",
                        tick, c.name, c.expectation, c.actual
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_path_finds_the_bare_argument() {
        let args: Vec<String> = ["-m", "POST", "/users/1", "--why"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(positional_path(&args).as_deref(), Some("/users/1"));
    }

    #[test]
    fn positional_path_missing_returns_none() {
        let args: Vec<String> = ["-m", "POST"].iter().map(|s| s.to_string()).collect();
        assert_eq!(positional_path(&args), None);
    }

    #[test]
    fn parse_requires_path() {
        let args: Vec<String> = vec!["-m".to_owned(), "GET".to_owned()];
        assert!(GetArgs::parse(&args).is_err());
    }

    #[test]
    fn parse_minimal() {
        let args: Vec<String> = vec!["/users/1".to_owned()];
        let a = GetArgs::parse(&args).unwrap();
        assert_eq!(a.path, "/users/1");
        assert_eq!(a.method, "GET");
        assert!(a.why.is_none());
    }

    #[test]
    fn why_enabled_defaults_off_in_text_on_in_json() {
        let args = GetArgs::parse(&["/x".to_owned()]).unwrap();
        assert!(!args.why_enabled(Format::Text));
        assert!(args.why_enabled(Format::Json));
    }

    #[test]
    fn why_flag_forces_on_in_text() {
        let args = GetArgs::parse(&["/x".to_owned(), "--why".to_owned()]).unwrap();
        assert!(args.why_enabled(Format::Text));
    }
}
