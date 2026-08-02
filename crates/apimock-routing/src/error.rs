//! Errors surfaced by the routing crate.
//!
//! # Why routing has its own error type
//!
//! Before 5.0, every failure in apimock funnelled into a single
//! `AppError`. That's natural for a single-crate project but awkward
//! across a workspace, because `apimock-routing` shouldn't have to know
//! about TLS failures or listener-address parsing — those are
//! server-layer concerns. Each of the three crates now defines its own
//! error variants at the right abstraction level:
//!
//! - `apimock-routing::RoutingError`  — rule-set read / parse
//! - `apimock-config::ConfigError`    — config read / parse, middleware
//!   compile, path resolution; wraps `RoutingError` when the failure
//!   came from a rule set
//! - `apimock-server::ServerError`    — TLS load, listener address
//!
//! The façade crate (`apimock`) re-exports all three under one
//! convenience alias (`AppError`) for existing consumers.

use std::{io, path::PathBuf};

/// Result alias used inside this crate.
pub type RoutingResult<T> = Result<T, RoutingError>;

/// All fatal errors produced by routing-layer operations.
#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    /// A rule-set TOML file could not be read.
    #[error("failed to read rule set file `{path}`: {source}")]
    RuleSetRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A rule-set TOML file could not be parsed.
    #[error("invalid rule set TOML in `{path}`{canonical_display}: {source}", canonical_display = match canonical {
        Some(p) => format!(" ({})", p.display()),
        None => String::new(),
    })]
    RuleSetParse {
        path: PathBuf,
        canonical: Option<PathBuf>,
        #[source]
        source: toml::de::Error,
    },
}
