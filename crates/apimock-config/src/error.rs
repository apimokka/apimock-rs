//! Errors surfaced by the config crate.
//!
//! See `apimock_routing::error` for the rationale on per-crate error
//! types. `ConfigError` wraps `RoutingError` via `#[from]` so rule-set
//! load failures flow through without the caller pattern-matching on
//! origin.
//!
//! # 5.1.0 additions
//!
//! - `WorkspaceError` — surfaced by `Workspace::load`.
//! - `ApplyError` — surfaced by `Workspace::apply`.
//! - `SaveError` — surfaced by `Workspace::save`.
//!
//! Each of the three "operation" errors wraps `ConfigError` via
//! `#[from]` because the underlying cause of most workspace failures
//! is a plain config load / write problem.
//!
//! # `#[non_exhaustive]` and `kind()` (RFC 041)
//!
//! All four enums here are `#[non_exhaustive]` — a public error type is
//! exactly where a new variant is most likely to be added later, so
//! leaving it exhaustively matchable freezes today's variant set as a
//! public contract by accident. Each gains a `kind()` accessor
//! returning its own `#[non_exhaustive]` `*Kind` enum, mechanically one
//! kind per variant, so a caller forced into a wildcard match arm by
//! `#[non_exhaustive]` still has a stable way to branch on failure
//! class instead of falling back to matching on `Display` text.
//!
//! **This is a different taxonomy from `apimock::cmd::envelope::ErrorKind`**,
//! deliberately. That one is the CLI's published contract, with a
//! schema version and a stability promise to agents. These `kind()`
//! methods describe library failures for library callers. Neither
//! delegates to the other — fusing them would tie a published CLI
//! contract to internal error refactoring.
//!
//! A variant that wraps another crate's error (`ConfigError::RuleSet`,
//! `WorkspaceError::Config`, …) gets its own kind naming *that it
//! wrapped something*, not the inner error's kind — `kind()` describes
//! *this* enum's variant, one-to-one, never a second hop into a nested
//! type's own taxonomy. A caller wanting the inner detail already has
//! `Error::source()` for that.

use std::{io, path::PathBuf};

use crate::view::NodeId;

pub type ConfigResult<T> = Result<T, ConfigError>;

/// # `#[non_exhaustive]` (RFC 041)
///
/// An enum's own fields stay constructible across the crate boundary —
/// `#[non_exhaustive]` on an `enum` restricts matching, not building
/// its existing variants. What it forbids is an exhaustive `match`
/// with no wildcard arm, since a future variant would otherwise make
/// this a silent non-breaking-looking compile error downstream:
///
/// ```compile_fail
/// use apimock_config::ConfigError;
///
/// fn describe(e: &ConfigError) -> &'static str {
///     match e {
///         ConfigError::ConfigRead { .. } => "read",
///         ConfigError::ConfigParse { .. } => "parse",
///         ConfigError::PathResolve { .. } => "path",
///         ConfigError::Validation { .. } => "validation",
///         ConfigError::RuleSet(_) => "rule_set",
///         // no `_` arm — exhaustive matches outside the crate must
///         // carry one once the enum is `#[non_exhaustive]`.
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
        // Boxed (RFC 041): `toml::de::Error` is 88 bytes, making this
        // variant 136 — the sole cause of every
        // `clippy::result_large_err` suppression this crate carried.
        // `#[source]` still reaches through the box unchanged; this is
        // a representation change, not a behavioural one.
        #[source]
        source: Box<toml::de::Error>,
    },

    /// A path on disk could not be resolved.
    #[error("failed to resolve path `{path}`: {source}")]
    PathResolve {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Startup-time validation failed. `reason` is the first failure
    /// encountered (RFC 065) — previously a bare unit variant whose
    /// only detail lived in a `log::error!` call at the failing
    /// validator's own site, which never reached a caller with no
    /// logger installed (`apimock validate`/`get`/`set`/`match-test`,
    /// none of which do).
    #[error("configuration validation failed: {reason}")]
    Validation { reason: String },

    /// A rule-set file failed to load or parse. Wraps the routing
    /// crate's error type.
    #[error(transparent)]
    RuleSet(#[from] apimock_routing::RoutingError),
}

/// `ConfigError`'s failure class, one variant per `ConfigError` variant.
/// See the module doc for why this exists and how it differs from
/// `apimock::cmd::envelope::ErrorKind`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorKind {
    Read,
    Parse,
    PathResolve,
    Validation,
    RuleSet,
}

impl ConfigError {
    pub fn kind(&self) -> ConfigErrorKind {
        match self {
            ConfigError::ConfigRead { .. } => ConfigErrorKind::Read,
            ConfigError::ConfigParse { .. } => ConfigErrorKind::Parse,
            ConfigError::PathResolve { .. } => ConfigErrorKind::PathResolve,
            ConfigError::Validation { .. } => ConfigErrorKind::Validation,
            ConfigError::RuleSet(_) => ConfigErrorKind::RuleSet,
        }
    }
}

/// Failure during `Workspace::load`. Currently a thin wrapper around
/// `ConfigError` — kept as its own type so the `Workspace` API signals
/// intent at the type level and has room to grow (e.g. "path is not a
/// directory", "no root config found").
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WorkspaceError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// Root path was not found or was not a regular file/directory.
    #[error("workspace root `{path}` is not a valid apimock workspace: {reason}")]
    InvalidRoot { path: PathBuf, reason: String },
}

/// `WorkspaceError`'s failure class. **Does not delegate to
/// `ConfigErrorKind`** (RFC 041's handoff § 4, decided explicitly):
/// `WorkspaceError` exists so the `Workspace` API signals intent at the
/// type level, and delegating its `kind()` would leak `ConfigError`'s
/// taxonomy through the type whose whole purpose is to have its own.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceErrorKind {
    Config,
    InvalidRoot,
}

impl WorkspaceError {
    pub fn kind(&self) -> WorkspaceErrorKind {
        match self {
            WorkspaceError::Config(_) => WorkspaceErrorKind::Config,
            WorkspaceError::InvalidRoot { .. } => WorkspaceErrorKind::InvalidRoot,
        }
    }
}

/// Failure during `Workspace::apply`.
///
/// # Why these particular variants
///
/// Every `EditCommand` variant targets a node by NodeId. The two
/// failure modes are "that ID doesn't exist" and "the ID exists but
/// refers to a node of the wrong kind for this command". Everything
/// else (file-not-found when `AddRuleSet` with a missing path) is a
/// validation issue reported via `ApplyResult::diagnostics`, not an
/// error return.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApplyError {
    /// The NodeId in the command wasn't found in the workspace.
    #[error("unknown node id: {id}")]
    UnknownNode { id: NodeId },

    /// The NodeId exists but names a node of the wrong kind for this
    /// command (e.g. `DeleteRule` pointing at a rule-set ID).
    #[error("node {id} is not of the expected kind for this command: {reason}")]
    WrongNodeKind { id: NodeId, reason: String },

    /// Invalid command payload (e.g. `MoveRule` with `new_index` past
    /// end of parent's rule list).
    #[error("invalid edit payload: {reason}")]
    InvalidPayload { reason: String },
}

/// `ApplyError`'s failure class.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyErrorKind {
    UnknownNode,
    WrongNodeKind,
    InvalidPayload,
}

impl ApplyError {
    pub fn kind(&self) -> ApplyErrorKind {
        match self {
            ApplyError::UnknownNode { .. } => ApplyErrorKind::UnknownNode,
            ApplyError::WrongNodeKind { .. } => ApplyErrorKind::WrongNodeKind,
            ApplyError::InvalidPayload { .. } => ApplyErrorKind::InvalidPayload,
        }
    }
}

/// Failure during `Workspace::save`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SaveError {
    /// A TOML file failed to serialise.
    #[error("failed to serialise `{path}`: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: toml::ser::Error,
    },
    /// Writing the serialised TOML to disk failed.
    #[error("failed to write `{path}`: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// The workspace's internal state was inconsistent at save time —
    /// usually a programmer error in the edit layer.
    #[error("internal inconsistency: {reason}")]
    Inconsistent { reason: String },
    /// The file changed on disk since it was last loaded or saved.
    /// In-place editing (RFC 056) re-reads the text it mutates, so it
    /// notices this where the old rebuild-from-model path could not.
    /// Overwriting would silently discard whatever changed it made —
    /// the caller must reload and reapply instead.
    #[error("`{path}` changed on disk since it was loaded; reload before saving")]
    Conflict { path: PathBuf },
    /// Re-reading a file to check it for external changes (RFC 056 §2
    /// Q3, ahead of an in-place save) failed — permission denied, the
    /// file deleted out from under us, etc. Distinct from `Conflict`:
    /// this is "we couldn't tell whether it changed," not "we could
    /// tell, and it did." Reloading — `Conflict`'s remedy — will not
    /// fix a permission error, so the two need different messages.
    #[error("failed to read `{path}` to check for external changes: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// `SaveError`'s failure class.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveErrorKind {
    Serialize,
    Write,
    Inconsistent,
    Conflict,
    Read,
}

impl SaveError {
    pub fn kind(&self) -> SaveErrorKind {
        match self {
            SaveError::Serialize { .. } => SaveErrorKind::Serialize,
            SaveError::Write { .. } => SaveErrorKind::Write,
            SaveError::Inconsistent { .. } => SaveErrorKind::Inconsistent,
            SaveError::Conflict { .. } => SaveErrorKind::Conflict,
            SaveError::Read { .. } => SaveErrorKind::Read,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::Error as _;
    use std::error::Error as _;

    fn a_toml_parse_error() -> toml::de::Error {
        toml::from_str::<toml::Value>("not valid toml =====")
            .expect_err("deliberately malformed TOML must fail to parse")
    }

    // ── RFC 041 § 6: boxing must not change Display / source() ────────

    #[test]
    fn config_parse_display_matches_pre_boxing_format() {
        let source = a_toml_parse_error();
        let expected_source_display = source.to_string();
        let err = ConfigError::ConfigParse {
            path: PathBuf::from("apimock.toml"),
            canonical: None,
            source: Box::new(source),
        };
        assert_eq!(
            err.to_string(),
            format!("invalid TOML in `apimock.toml`: {expected_source_display}")
        );
    }

    #[test]
    fn config_parse_display_includes_canonical_path_when_present() {
        let source = a_toml_parse_error();
        let expected_source_display = source.to_string();
        let err = ConfigError::ConfigParse {
            path: PathBuf::from("apimock.toml"),
            canonical: Some(PathBuf::from("/abs/apimock.toml")),
            source: Box::new(source),
        };
        assert_eq!(
            err.to_string(),
            format!(
                "invalid TOML in `apimock.toml` (/abs/apimock.toml): {expected_source_display}"
            )
        );
    }

    #[test]
    fn config_parse_source_reaches_the_boxed_toml_error() {
        let source = a_toml_parse_error();
        let source_display = source.to_string();
        let err = ConfigError::ConfigParse {
            path: PathBuf::from("apimock.toml"),
            canonical: None,
            source: Box::new(source),
        };
        let reached = err.source().expect("ConfigParse always carries a source");
        assert_eq!(reached.to_string(), source_display);
    }

    // ── kind() — one assertion per variant, all six enums ──────────────

    #[test]
    fn config_error_kind_matches_every_variant() {
        assert_eq!(
            ConfigError::ConfigRead {
                path: PathBuf::from("x"),
                source: io::Error::other("x"),
            }
            .kind(),
            ConfigErrorKind::Read
        );
        assert_eq!(
            ConfigError::ConfigParse {
                path: PathBuf::from("x"),
                canonical: None,
                source: Box::new(a_toml_parse_error()),
            }
            .kind(),
            ConfigErrorKind::Parse
        );
        assert_eq!(
            ConfigError::PathResolve {
                path: PathBuf::from("x"),
                source: io::Error::other("x"),
            }
            .kind(),
            ConfigErrorKind::PathResolve
        );
        assert_eq!(
            ConfigError::Validation {
                reason: "x".to_owned()
            }
            .kind(),
            ConfigErrorKind::Validation
        );
        assert_eq!(
            ConfigError::RuleSet(apimock_routing::RoutingError::RuleSetRead {
                path: PathBuf::from("x"),
                source: io::Error::other("x"),
            })
            .kind(),
            ConfigErrorKind::RuleSet
        );
    }

    #[test]
    fn workspace_error_kind_matches_every_variant() {
        assert_eq!(
            WorkspaceError::Config(ConfigError::Validation {
                reason: "x".to_owned()
            })
            .kind(),
            WorkspaceErrorKind::Config
        );
        assert_eq!(
            WorkspaceError::InvalidRoot {
                path: PathBuf::from("x"),
                reason: "x".to_owned(),
            }
            .kind(),
            WorkspaceErrorKind::InvalidRoot
        );
    }

    #[test]
    fn apply_error_kind_matches_every_variant() {
        assert_eq!(
            ApplyError::UnknownNode { id: NodeId::new() }.kind(),
            ApplyErrorKind::UnknownNode
        );
        assert_eq!(
            ApplyError::WrongNodeKind {
                id: NodeId::new(),
                reason: "x".to_owned(),
            }
            .kind(),
            ApplyErrorKind::WrongNodeKind
        );
        assert_eq!(
            ApplyError::InvalidPayload {
                reason: "x".to_owned(),
            }
            .kind(),
            ApplyErrorKind::InvalidPayload
        );
    }

    #[test]
    fn save_error_kind_matches_every_variant() {
        assert_eq!(
            SaveError::Serialize {
                path: PathBuf::from("x"),
                source: toml::ser::Error::custom("x"),
            }
            .kind(),
            SaveErrorKind::Serialize
        );
        assert_eq!(
            SaveError::Write {
                path: PathBuf::from("x"),
                source: io::Error::other("x"),
            }
            .kind(),
            SaveErrorKind::Write
        );
        assert_eq!(
            SaveError::Inconsistent {
                reason: "x".to_owned(),
            }
            .kind(),
            SaveErrorKind::Inconsistent
        );
        assert_eq!(
            SaveError::Conflict {
                path: PathBuf::from("x"),
            }
            .kind(),
            SaveErrorKind::Conflict
        );
        assert_eq!(
            SaveError::Read {
                path: PathBuf::from("x"),
                source: io::Error::other("x"),
            }
            .kind(),
            SaveErrorKind::Read
        );
    }
}
