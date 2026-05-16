//! Configuration model for apimock.
//!
//! # Responsibilities
//!
//! - Read `apimock.toml` and every file it references.
//! - Resolve relative paths against the config file's parent directory.
//! - Validate that paths exist, rules are consistent, etc.
//! - Provide the [`Workspace`] façade that a GUI uses to edit a
//!   loaded configuration by domain-level commands instead of TOML
//!   text.
//!
//! # What is deliberately *not* here
//!
//! - Compiling Rhai middlewares — that's `apimock-server`'s job. This
//!   crate only records the paths listed under `service.middlewares`.
//! - HTTP response construction. Fully in `apimock-server`.
//! - Rule-set parsing. Delegated to `apimock-routing::RuleSet::new`.
//!
//! # 5.1.0 — GUI-facing API
//!
//! 5.0.0 defined the *shape* of the types a future GUI would depend
//! on; 5.1 makes the `Workspace` value that wraps them actually
//! loadable and introspectable. Per the spec's §12 step plan, 5.1.0
//! implements Steps 1–3 (load + snapshot + validate), leaving Step 4
//! (save + diff) and Step 5 (richer routing snapshot) for 5.2+.

pub mod config;
pub mod error;
pub mod path_util;
mod toml_writer;
pub mod view;
pub mod workspace;

pub use config::{
    Config,
    constant::{LISTENER_DEFAULT_IP_ADDRESS, LISTENER_DEFAULT_PORT},
    listener_config::ListenerConfig,
    log_config::LogConfig,
    service_config::ServiceConfig,
};
pub use error::{ApplyError, ConfigError, ConfigResult, SaveError, WorkspaceError};
pub use view::{
    ApplyResult, ConfigFileKind, ConfigFileView, ConfigNodeView, DiffItem, DiffKind, Diagnostic,
    EditCommand, EditValue, NodeId, NodeKind, NodeValidation, ReloadHint, RespondPayload,
    RootSettingKey, RulePayload, SaveResult, Severity, ValidationIssue, ValidationReport,
    WorkspaceSnapshot,
};
pub use workspace::Workspace;
