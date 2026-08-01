//! `apimock validate` — validate a workspace config without starting the server.
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0    | No errors (warnings are printed but not fatal). |
//! | 1    | At least one `Severity::Error` diagnostic (or `--strict` and warnings present). |
//! | 2    | Config could not be loaded (parse / file-read error). |

use apimock_config::{Severity, Workspace};

/// Flags parsed from the `apimock validate` command line.
pub struct ValidateArgs {
    pub config_path: String,
    pub strict: bool,
    pub quiet: bool,
    pub json: bool,
}

const CONFIG_NAMES: &[&str] = &["--config", "-c"];
const STRICT_FLAG: &str = "--strict";
const QUIET_FLAG: &str = "--quiet";
const JSON_FLAG: &str = "--json";

impl ValidateArgs {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let config_path = super::match_test::flag_value(args, CONFIG_NAMES)
            .ok_or_else(|| "missing required flag --config / -c".to_owned())?;
        Ok(Self {
            config_path,
            strict: args.iter().any(|a| a == STRICT_FLAG),
            quiet: args.iter().any(|a| a == QUIET_FLAG),
            json: args.iter().any(|a| a == JSON_FLAG),
        })
    }
}

/// Run validation and print results. Returns the process exit code.
pub fn run(args: &[String]) -> i32 {
    let parsed = match ValidateArgs::parse(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("apimock validate: {}", e);
            eprintln!(
                "Usage: apimock validate --config <apimock.toml> [--strict] [--quiet] [--json]"
            );
            return 2;
        }
    };

    let ws = match Workspace::load(parsed.config_path.clone().into()) {
        Ok(ws) => ws,
        Err(e) => {
            if !parsed.quiet {
                eprintln!("apimock validate: failed to load config: {}", e);
            }
            return 2;
        }
    };

    let report = ws.validate();

    // Collect rule-set and rule counts for the success banner.
    let snap = ws.snapshot();
    let rule_set_count = snap.routes.rule_sets.len();
    let rule_count: usize = snap.routes.rule_sets.iter().map(|rs| rs.rules.len()).sum();

    if parsed.json {
        // Emit diagnostics as a JSON array.
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
        println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_default()
        );
    } else if !parsed.quiet {
        for d in &report.diagnostics {
            let tag = match d.severity {
                Severity::Error => "[ERROR]",
                Severity::Warning => "[WARNING]",
                Severity::Info => "[INFO]",
            };
            let location = d
                .file
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| parsed.config_path.clone());
            println!("{}: {} {}", location, tag, d.message);
        }
    }

    let has_errors = report
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, Severity::Error));
    let has_warnings = report
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, Severity::Warning));

    if has_errors || (parsed.strict && has_warnings) {
        if !parsed.quiet {
            let e = report
                .diagnostics
                .iter()
                .filter(|d| matches!(d.severity, Severity::Error))
                .count();
            let w = report
                .diagnostics
                .iter()
                .filter(|d| matches!(d.severity, Severity::Warning))
                .count();
            eprintln!("Validation failed: {} error(s), {} warning(s).", e, w);
        }
        return 1;
    }

    if !parsed.quiet {
        let w = report
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Warning))
            .count();
        if w > 0 {
            println!(
                "Validation passed with {} warning(s) ({} rules across {} rule set(s)).",
                w, rule_count, rule_set_count
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
        assert!(!a.json);
    }

    #[test]
    fn parse_args_all_flags() {
        let args: Vec<String> = vec![
            "-c".to_owned(),
            "config.toml".to_owned(),
            "--strict".to_owned(),
            "--quiet".to_owned(),
            "--json".to_owned(),
        ];
        let a = ValidateArgs::parse(&args).unwrap();
        assert_eq!(a.config_path, "config.toml");
        assert!(a.strict);
        assert!(a.quiet);
        assert!(a.json);
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
}
