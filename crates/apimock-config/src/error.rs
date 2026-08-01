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

use std::{io, path::PathBuf};

use crate::view::NodeId;

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

/// Failure during `Workspace::load`. Currently a thin wrapper around
/// `ConfigError` — kept as its own type so the `Workspace` API signals
/// intent at the type level and has room to grow (e.g. "path is not a
/// directory", "no root config found").
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// Root path was not found or was not a regular file/directory.
    #[error("workspace root `{path}` is not a valid apimock workspace: {reason}")]
    InvalidRoot { path: PathBuf, reason: String },
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

/// Failure during `Workspace::save`.
#[derive(Debug, thiserror::Error)]
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
}
