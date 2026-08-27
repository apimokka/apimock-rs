//! The `[service]` section of `apimock.toml`.
//!
//! # What was here before 5.0
//!
//! Pre-5.0, `ServiceConfig` also held `Vec<MiddlewareHandler>` and had
//! `middleware_response` / `rule_set_response` methods that built
//! `hyper::Response` values. Those methods violated the 5.0 crate
//! boundary (config owns declarative data, not HTTP responses), so
//! they moved:
//!
//! - Compiled Rhai middlewares live in `apimock-server` now, inside a
//!   new `LoadedMiddlewares` type built from this struct's
//!   `middlewares_file_paths` at startup.
//! - HTTP dispatch methods live on the server's request-handling path.
//!
//! What stays here is the editable, serde-deserialisable data that a
//! GUI would show in its `[service]` panel.

use apimock_routing::{RuleSet, Strategy};
use console::style;
use serde::Deserialize;
use util::canonicalized_fallback_respond_dir_to_print;

use std::path::Path;

mod util;

use super::constant::{PRINT_DELIMITER, SERVICE_DEFAULT_FALLBACK_RESPOND_DIR};

#[derive(Clone, Deserialize)]
#[non_exhaustive]
pub struct ServiceConfig {
    /// How multiple matching rules in a set are resolved. Currently
    /// `first_match` is the only recognised value.
    pub strategy: Option<Strategy>,

    /// Paths to rule-set TOML files. User-editable via the `[service]`
    /// table.
    #[serde(rename = "rule_sets")]
    pub rule_sets_file_paths: Option<Vec<String>>,

    /// Loaded rule sets in the same order as `rule_sets_file_paths`.
    /// Populated at startup by `Config::new` (this field is
    /// `#[serde(skip)]` because it comes from loading the files listed
    /// above, not from the config TOML itself).
    #[serde(skip)]
    pub rule_sets: Vec<RuleSet>,

    /// Paths to Rhai middleware files. Compilation happens in the
    /// server crate — config only holds the paths.
    #[serde(rename = "middlewares")]
    pub middlewares_file_paths: Option<Vec<String>>,

    /// Filesystem directory served by the dyn-route fallback.
    pub fallback_respond_dir: String,
}

impl ServiceConfig {
    /// Validate that every rule set and the fallback respond dir are
    /// internally consistent.
    ///
    /// # Why validation stays in config, not routing
    ///
    /// Validation inspects cross-cutting state (file existence of the
    /// fallback dir, every rule's respond.file_path, etc.). All that
    /// state is assembled by config loading. Routing has per-rule
    /// validators, which this method calls.
    ///
    /// # Why `Result<(), String>` and not `bool`
    ///
    /// See `apimock_routing::Respond::validate`'s own doc comment (RFC
    /// 065) — the short version: a `bool` failure reached
    /// `apimock validate`/`get`/`set`/`match-test` as a bare
    /// "configuration validation failed," since none of them install a
    /// logger. Returning the first failure's own reason and threading
    /// it up to `ConfigError::Validation { reason }` fixes that for
    /// every validator here, not only the respond-side ones this RFC
    /// actually touches — `prefix`/`default`/`guard`/`when` still only
    /// log a *more* detailed reason at their own call site (out of this
    /// RFC's scope to change), but the caller now gets *some* concrete
    /// reason either way, instead of none.
    pub fn validate(&self) -> Result<(), String> {
        for (rule_set_idx, rule_set) in self.rule_sets.iter().enumerate() {
            if let Some(prefix) = rule_set.prefix.as_ref()
                && !prefix.validate(rule_set.dir_prefix().as_str(), rule_set_idx)
            {
                return Err(format!("invalid prefix (rule set #{})", rule_set_idx + 1));
            }

            if let Some(default) = rule_set.default.as_ref()
                && !default.validate()
            {
                return Err(format!("invalid default (rule set #{})", rule_set_idx + 1));
            }

            if let Some(guard) = rule_set.guard.as_ref()
                && !guard.validate()
            {
                return Err(format!("invalid guard (rule set #{})", rule_set_idx + 1));
            }

            let dir_prefix = rule_set.dir_prefix();
            for (rule_idx, rule) in rule_set.rules.iter().enumerate() {
                rule.validate(dir_prefix.as_str(), rule_idx, rule_set_idx)?;
            }
        }

        if !Path::new(self.fallback_respond_dir.as_str()).exists() {
            return Err(format!(
                "invalid fallback_respond_dir: {}",
                self.fallback_respond_dir
            ));
        }

        Ok(())
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        ServiceConfig {
            strategy: Some(Strategy::default()),
            rule_sets_file_paths: None,
            rule_sets: vec![],
            middlewares_file_paths: None,
            fallback_respond_dir: SERVICE_DEFAULT_FALLBACK_RESPOND_DIR.to_owned(),
        }
    }
}

impl std::fmt::Display for ServiceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let has_rule_sets = !self.rule_sets.is_empty();

        if has_rule_sets {
            let _ = writeln!(
                f,
                "[rule_sets.strategy] {}",
                self.strategy.clone().unwrap_or_default()
            );
            let _ = writeln!(f);
        }

        for (idx, rule_set) in self.rule_sets.iter().enumerate() {
            let _ = writeln!(
                f,
                "@ rule_set #{} ({})\n",
                idx + 1,
                style(rule_set.file_path.as_str()).green()
            );
            let _ = writeln!(f, "{}", rule_set);
        }

        if has_rule_sets {
            let _ = writeln!(f, "{}", PRINT_DELIMITER);
        }

        let _ = writeln!(
            f,
            "[fallback_respond_dir] {}",
            canonicalized_fallback_respond_dir_to_print(self.fallback_respond_dir.as_str())
        );

        Ok(())
    }
}
