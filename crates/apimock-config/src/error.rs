//! Errors surfaced by the config crate.
//!
//! See `apimock_routing::error` for the rationale on per-crate error
//! types. `ConfigError` wraps `RoutingError` via `#[from]` so rule-set
//! load failures flow through without the caller pattern-matching on
//! origin.

use std::{io, path::PathBuf};

pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The config TOML file could not be read from disk.
    #[error("failed to read config file `{path}`: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The config TOML file was read, but could not be parsed.
    #[error("invalid TOML in `{path}`{canonical_display}: {source}", canonical_display = match canonical {
        Some(p) => format!(" ({})", p.display()),
        None => String::new(),
    })]
    ConfigParse {
        path: PathBuf,
        canonical: Option<PathBuf>,
        #[source]
        source: toml::de::Error,
    },

    /// A path on disk could not be resolved.
    #[error("failed to resolve path `{path}`: {source}")]
    PathResolve {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Startup-time validation failed — each individual failure is
    /// already logged at its call site.
    #[error("configuration validation failed")]
    Validation,

    /// A rule-set file failed to load or parse. Wraps the routing
    /// crate's error type.
    #[error(transparent)]
    RuleSet(#[from] apimock_routing::RoutingError),
}
