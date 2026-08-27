//! `apimock validate` — validate a workspace config without starting the server.
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0    | No errors (warnings are printed but not fatal). |
//! | 1    | At least one `Severity::Error` diagnostic (or `--strict` and warnings present). |
//! | 2    | Config could not be loaded (parse / file-read error), or a bad invocation. |
//!
//! **Exit `1` is not reachable today, and `--strict` has nothing to
//! act on.** `Workspace::load` (called below, before `report` exists)
//! already checks — identically — every condition
//! `respond_node_validation` would otherwise turn into a
//! `Severity::Error` diagnostic, so a config with such a problem fails
//! to *load* (exit 2) rather than loading and being reported. Nothing
//! anywhere constructs `Severity::Warning`/`Info` either. So
//! `report.diagnostics` is always empty by the time this module sees
//! it, through the CLI. Found 2026-08-17 building RFC 054's test
//! fixtures; documented here rather than fixed, since a real fix
//! changes config-load validation shared with server startup — larger
//! than this file's scope. The table above is left describing the
//! *design* (unchanged by RFC 054, per its Non-goals), not current
//! reachability.
//!
//! # `--json` is removed (RFC 054 → 6.0.0)
//!
//! `--json` (the bare diagnostics array RFC 054 deprecated in 5.19.0)
//! is gone. Using it now fails loudly rather than being silently
//! absorbed: exit 2, a message naming `--format json` as the
//! replacement, on stderr by default or inside RFC 053's envelope
//! (`error.kind: "usage"`) when `--format json` was also given — see
//! `json_removed_error` below. This is the sole exercise, across the
//! whole 6.0.0 release, of RFC 048 § 7's loud-failure requirement for a
//! removed flag.

use apimock_config::{Severity, Workspace};

use crate::cmd::envelope::{self, Format};

/// Flags parsed from the `apimock validate` command line.
pub struct ValidateArgs {
    pub config_path: String,
    pub strict: bool,
    pub quiet: bool,
    pub format: Option<Format>,
}

const CONFIG_NAMES: &[&str] = &["--config", "-c"];
const STRICT_FLAG: &str = "--strict";
const QUIET_FLAG: &str = "--quiet";
/// Removed in 6.0.0 (RFC 054). Still listed in `known_flag_names()` /
/// `NO_VALUE_FLAG_NAMES` below — not because it's accepted, but so
/// `reject_unknown_flags` lets it through to `run()`'s own check
/// instead of reporting it as an unrecognised argument. `--json` and
/// `--format` are too far apart for `near_match` (RFC 059) to ever
/// suggest one for the other, so the generic "unrecognized argument"
/// path would never name the replacement — `run()`'s dedicated check
/// does, unconditionally.
const JSON_FLAG: &str = "--json";
const FORMAT_FLAG: &str = "--format";
/// Flags that take no value — every other known flag does.
const NO_VALUE_FLAG_NAMES: &[&str] = &[STRICT_FLAG, QUIET_FLAG, JSON_FLAG];

fn known_flag_names() -> Vec<&'static str> {
    CONFIG_NAMES
        .iter()
        .copied()
        .chain([STRICT_FLAG, QUIET_FLAG, JSON_FLAG, FORMAT_FLAG])
        .collect()
}

/// `--json`'s removal error (RFC 054 → 6.0.0, RFC 048 § 7). Named after
/// what it reports, not the flag itself — this command has no other
/// removed flag today, but a second one later would want its own
/// message, not a generically-named one this reads oddly for.
const JSON_REMOVED_MESSAGE: &str = "--json was removed in 6.0.0; use --format json instead, which emits the RFC 053 response envelope";

impl ValidateArgs {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        // A dangling `-c` (no value) and an absent `-c` both mean "no
        // config path was given" from a required flag's point of view —
        // the same message applies to both (RFC 064: this is the one
        // place a required flag's dangling form must keep saying
        // exactly what it already said, not `flag_value`'s new generic
        // "requires a value"). An *explicit* empty value (`-c=`, RFC 064
        // Amendment 1) is a third, distinct case — not "no path given",
        // but "the path given is empty" — reviewed as its own error
        // rather than folded into either of the two above, so the
        // message names the flag instead of surfacing an empty path
        // several layers downstream.
        let config_path = match super::flags::flag_value(args, CONFIG_NAMES) {
            Ok(Some(v)) if v.is_empty() => {
                return Err("--config / -c must be a non-empty path, got ''".to_owned());
            }
            Ok(Some(v)) => v,
            Ok(None) | Err(_) => {
                return Err("missing required flag --config / -c".to_owned());
            }
        };

        // `--json` never reaches this point: `run()` intercepts its mere
        // presence, before parsing gets this far, with its own removal
        // error — see `JSON_FLAG`'s doc comment. So there is no
        // `--json`/`--format` conflict left to check here; `--format`
        // is just parsed on its own.
        let format_raw = super::flags::flag_value(args, &[FORMAT_FLAG])?;

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
            config_path,
            strict: args.iter().any(|a| a == STRICT_FLAG),
            quiet: args.iter().any(|a| a == QUIET_FLAG),
            format,
        })
    }
}

/// Build the `diagnostics` array carried inside `--format json`'s
/// `result` (RFC 053). Once the same shape `--json` used to emit bare,
/// before its 6.0.0 removal; kept as its own function since
/// `envelope::ok` still wraps a plain `Value`, not this report type
/// directly.
fn diagnostics_json(report: &apimock_config::ValidationReport) -> serde_json::Value {
    let items: Vec<serde_json::Value> = report
        .diagnostics
        .iter()
        .map(|d| {
            serde_json::json!({
                "severity": format!("{:?}", d.severity).to_lowercase(),
                "message": d.message,
                "node_id": d.node_id.map(|n| n.0.to_string()),
                "file": d.file.as_ref().map(|p| p.to_string_lossy().into_owned()),
            })
        })
        .collect();
    serde_json::Value::Array(items)
}

const USAGE: &str =
    "Usage: apimock validate --config <apimock.toml> [--strict] [--quiet] [--format text|json]";

fn usage_error(message: &str) -> i32 {
    eprintln!("apimock validate: {}", message);
    eprintln!("{}", USAGE);
    2
}

/// `--json`'s removal error. Enveloped (RFC 053, `error.kind: "usage"`)
/// when the caller also asked for `--format json` — the one caller
/// most likely to be parsing this programmatically still gets a
/// machine-readable answer back, not a plain-text error it can't parse
/// — plain stderr otherwise, matching every other usage error this
/// command reports.
fn json_removed_error(is_envelope: bool) -> i32 {
    if is_envelope {
        let envelope = envelope::err(envelope::ErrorKind::Usage, JSON_REMOVED_MESSAGE);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).unwrap_or_default()
        );
        2
    } else {
        usage_error(JSON_REMOVED_MESSAGE)
    }
}

/// Run validation and print results. Returns the process exit code.
pub fn run(args: &[String]) -> i32 {
    // RFC 059: rejected before `ValidateArgs::parse` — a typo'd flag
    // (`--strct`) must never be silently absorbed the way it previously
    // was, since `ValidateArgs::parse` only ever *looked for* its known
    // flags and had no path that noticed an unrecognised one.
    if let Err(e) = super::flags::reject_unknown_flags(
        args,
        &known_flag_names(),
        NO_VALUE_FLAG_NAMES,
        false,
        "unrecognized argument",
    ) {
        return usage_error(&e);
    }

    // RFC 054 → 6.0.0: checked before `ValidateArgs::parse` even runs,
    // and unconditionally — regardless of `--config`/`--format`/etc.
    // being valid — so a caller still using `--json` learns that,
    // specifically, rather than some other invocation problem. Whether
    // to envelope the answer is read directly off the raw args (not
    // `parsed.format`, since parsing never happens on this path).
    if args.iter().any(|a| a == JSON_FLAG) {
        let is_envelope = super::flags::flag_value(args, &[FORMAT_FLAG])
            .ok()
            .flatten()
            .as_deref()
            == Some("json");
        return json_removed_error(is_envelope);
    }

    let parsed = match ValidateArgs::parse(args) {
        Ok(a) => a,
        Err(e) => return usage_error(&e),
    };
    let is_envelope = parsed.format == Some(Format::Json);

    let ws = match Workspace::load(parsed.config_path.clone().into()) {
        Ok(ws) => ws,
        Err(e) => {
            if is_envelope {
                let envelope = envelope::err(
                    envelope::kind_for_workspace_error(&e),
                    format!("failed to load config: {}", e),
                );
                println!(
                    "{}",
                    serde_json::to_string_pretty(&envelope).unwrap_or_default()
                );
            } else if !parsed.quiet {
                eprintln!("apimock validate: failed to load config: {}", e);
            }
            return 2;
        }
    };

    let report = ws.validate();

    // Collect rule-set and rule counts for the success banner / summary.
    let snap = ws.snapshot();
    let rule_set_count = snap.routes.rule_sets.len();
    let rule_count: usize = snap.routes.rule_sets.iter().map(|rs| rs.rules.len()).sum();
    let error_count = report
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .count();
    let warning_count = report
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Warning))
        .count();
    let has_errors = error_count > 0;
    let has_warnings = warning_count > 0;

    if is_envelope {
        // RFC 053's envelope, with room this bare array never had: a
        // `summary` alongside `diagnostics`, addable later without
        // moving `diagnostics` or changing its type.
        let result = serde_json::json!({
            "diagnostics": diagnostics_json(&report),
            "summary": {
                "errors": error_count,
                "warnings": warning_count,
                "rule_sets": rule_set_count,
                "rules": rule_count,
            },
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope::ok(result)).unwrap_or_default()
        );
    } else if !parsed.quiet {
        for d in &report.diagnostics {
            let tag = match d.severity {
                Severity::Error => "[ERROR]",
                Severity::Warning => "[WARNING]",
                Severity::Info => "[INFO]",
                // `Severity` is `#[non_exhaustive]` (RFC 041) — a future
                // variant prints under the most conservative existing tag
                // rather than failing to compile or panicking.
                _ => "[ERROR]",
            };
            let location = d
                .file
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| parsed.config_path.clone());
            println!("{}: {} {}", location, tag, d.message);
        }
    }

    // The pass/fail banners are human text; `--format json` already
    // said all of this inside the envelope's `summary`; adding it again
    // as loose stderr/stdout lines would be an inconsistent second
    // source of truth for the one caller who asked for a single
    // machine-parseable answer.
    if has_errors || (parsed.strict && has_warnings) {
        if !is_envelope && !parsed.quiet {
            eprintln!(
                "Validation failed: {} error(s), {} warning(s).",
                error_count, warning_count
            );
        }
        return 1;
    }

    if !is_envelope && !parsed.quiet {
        if has_warnings {
            println!(
                "Validation passed with {} warning(s) ({} rules across {} rule set(s)).",
                warning_count, rule_count, rule_set_count
            );
        } else {
            println!(
                "Validation passed ({} rules across {} rule set(s)).",
                rule_count, rule_set_count
            );
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_requires_config() {
        let args: Vec<String> = vec!["--quiet".to_owned()];
        assert!(ValidateArgs::parse(&args).is_err());
    }

    #[test]
    fn parse_args_minimal() {
        let args: Vec<String> = vec!["--config".to_owned(), "apimock.toml".to_owned()];
        let a = ValidateArgs::parse(&args).unwrap();
        assert_eq!(a.config_path, "apimock.toml");
        assert!(!a.strict);
        assert!(!a.quiet);
    }

    #[test]
    fn parse_args_all_flags() {
        let args: Vec<String> = vec![
            "-c".to_owned(),
            "config.toml".to_owned(),
            "--strict".to_owned(),
            "--quiet".to_owned(),
        ];
        let a = ValidateArgs::parse(&args).unwrap();
        assert_eq!(a.config_path, "config.toml");
        assert!(a.strict);
        assert!(a.quiet);
    }

    #[test]
    fn run_missing_config_file_returns_2() {
        let args: Vec<String> = vec![
            "--config".to_owned(),
            "/nonexistent/apimock.toml".to_owned(),
            "--quiet".to_owned(),
        ];
        assert_eq!(run(&args), 2);
    }

    // ── `--json` removal (RFC 054 → 6.0.0) ─────────────────────────────

    #[test]
    fn json_flag_is_a_removal_error_not_an_unrecognised_argument() {
        let args: Vec<String> = vec![
            "--config".to_owned(),
            "apimock.toml".to_owned(),
            "--json".to_owned(),
        ];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn json_flag_is_rejected_even_with_no_config_given() {
        // Unconditional: the caller learns about `--json` regardless of
        // what else is wrong with the invocation.
        let args: Vec<String> = vec!["--json".to_owned()];
        assert_eq!(run(&args), 2);
    }
}
