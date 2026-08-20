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
//!
//! # `#[non_exhaustive]` and `kind()` (RFC 041)
//!
//! `RoutingError` is `#[non_exhaustive]` and gains a `kind()` accessor
//! returning `RoutingErrorKind`, one variant per `RoutingError` variant
//! — the same treatment applied to the other five public error enums
//! in this workspace. See `apimock_config::error`'s module doc for the
//! full reasoning (why `#[non_exhaustive]`, why `kind()`, and why it's
//! deliberately not the same taxonomy as `apimock::cmd::envelope::ErrorKind`).

use std::{io, path::PathBuf};

/// Result alias used inside this crate.
pub type RoutingResult<T> = Result<T, RoutingError>;

/// All fatal errors produced by routing-layer operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
        // Boxed (RFC 041): `toml::de::Error` is 88 bytes, making this
        // variant 136 — measured as the exact cause of this crate's
        // `clippy::result_large_err` suppression. `#[source]` still
        // reaches through the box; representation change only.
        #[source]
        source: Box<toml::de::Error>,
    },
}

/// `RoutingError`'s failure class.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingErrorKind {
    RuleSetRead,
    RuleSetParse,
}

impl RoutingError {
    pub fn kind(&self) -> RoutingErrorKind {
        match self {
            RoutingError::RuleSetRead { .. } => RoutingErrorKind::RuleSetRead,
            RoutingError::RuleSetParse { .. } => RoutingErrorKind::RuleSetParse,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    fn a_toml_parse_error() -> toml::de::Error {
        toml::from_str::<toml::Value>("not valid toml =====")
            .expect_err("deliberately malformed TOML must fail to parse")
    }

    // ── RFC 041 § 6: boxing must not change Display / source() ────────

    #[test]
    fn rule_set_parse_display_matches_pre_boxing_format() {
        let source = a_toml_parse_error();
        let expected_source_display = source.to_string();
        let err = RoutingError::RuleSetParse {
            path: PathBuf::from("rules.toml"),
            canonical: None,
            source: Box::new(source),
        };
        assert_eq!(
            err.to_string(),
            format!("invalid rule set TOML in `rules.toml`: {expected_source_display}")
        );
    }

    #[test]
    fn rule_set_parse_source_reaches_the_boxed_toml_error() {
        let source = a_toml_parse_error();
        let source_display = source.to_string();
        let err = RoutingError::RuleSetParse {
            path: PathBuf::from("rules.toml"),
            canonical: None,
            source: Box::new(source),
        };
        let reached = err.source().expect("RuleSetParse always carries a source");
        assert_eq!(reached.to_string(), source_display);
    }

    #[test]
    fn routing_error_kind_matches_every_variant() {
        assert_eq!(
            RoutingError::RuleSetRead {
                path: PathBuf::from("x"),
                source: io::Error::other("x"),
            }
            .kind(),
            RoutingErrorKind::RuleSetRead
        );
        assert_eq!(
            RoutingError::RuleSetParse {
                path: PathBuf::from("x"),
                canonical: None,
                source: Box::new(a_toml_parse_error()),
            }
            .kind(),
            RoutingErrorKind::RuleSetParse
        );
    }
}
