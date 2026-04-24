//! Configuration model for apimock.
//!
//! # Responsibilities
//!
//! - Read `apimock.toml` and every file it references.
//! - Resolve relative paths against the config file's parent directory.
//! - Validate that paths exist, rules are consistent, etc.
//! - (Stage 2) apply structured edit commands from a GUI, and save the
//!   result back to disk preserving structure as much as possible.
//!
//! # What is deliberately *not* here
//!
//! - Compiling Rhai middlewares — that's `apimock-server`'s job. This
//!   crate only records the paths listed under `service.middlewares`.
//! - HTTP response construction. Fully in `apimock-server`.
//! - Rule-set parsing. Delegated to `apimock-routing::RuleSet::new`.
//!
//! # Stage-1 surface
//!
//! The `view` module defines the stage-1 GUI-facing API shape
//! ([`WorkspaceSnapshot`] etc.). In 5.0.0 these types carry their
//! definition and rustdoc but no populating logic; the stage-2 work
//! fills in construction + edit / save behaviour.

pub mod config;
pub mod error;
pub mod path_util;
pub mod view;

pub use config::{
    Config,
    constant::{LISTENER_DEFAULT_IP_ADDRESS, LISTENER_DEFAULT_PORT},
    listener_config::ListenerConfig,
    log_config::LogConfig,
    service_config::ServiceConfig,
};
pub use error::{ConfigError, ConfigResult};
pub use view::{
    ApplyResult, ConfigFileView, ConfigNodeView, Diagnostic, DiagnosticSeverity, EditCommand,
    EditTarget, EditValue, NodeKind, ReloadHint, SaveResult, WorkspaceSnapshot,
};
