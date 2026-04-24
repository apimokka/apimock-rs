//! Read-only + edit-command views on config state for GUI tooling.
//!
//! # Stage-1 scope (5.0.0)
//!
//! Per the 5.0.0 brief (§4.1), this module defines the *shape* of the
//! editable API a future GUI will depend on. Types are declared with
//! their fields and rustdoc-annotated responsibilities. Populating them
//! from a live `Config` — and mutating a `Config` via `EditCommand` — is
//! stage-2 work.
//!
//! Every type here is `#[non_exhaustive]` so later stages can add
//! fields / variants without breaking downstream code that depends on
//! the stage-1 shape.
//!
//! # The model this API assumes
//!
//! A GUI sees a *workspace* — the root `apimock.toml` plus every file
//! referenced from it (rule sets, middlewares). Each editable value in
//! the workspace lives in exactly one of those files; tracking the
//! origin is essential so the GUI knows which file a "Save" button
//! should write.

use serde::Serialize;

use std::path::PathBuf;

/// A snapshot of the whole workspace — every loaded file and the
/// editable nodes inside each.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct WorkspaceSnapshot {
    /// The root `apimock.toml` file.
    pub root: ConfigFileView,
    /// Rule-set files referenced from `root`, in the same order they
    /// appear in `service.rule_sets`.
    pub rule_sets: Vec<ConfigFileView>,
    /// Middleware files referenced from `root`.
    pub middlewares: Vec<ConfigFileView>,
    /// Issues found during the most recent load. `ok` iff empty.
    pub diagnostics: Vec<Diagnostic>,
}

impl WorkspaceSnapshot {
    pub fn empty() -> Self {
        Self {
            root: ConfigFileView::empty(PathBuf::new()),
            rule_sets: Vec::new(),
            middlewares: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

/// One TOML file inside the workspace.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ConfigFileView {
    /// Absolute path on disk.
    pub path: PathBuf,
    /// Path as written inside the parent config (e.g.
    /// `"apimock-rule-set.toml"`). Useful for displaying without the
    /// full absolute path noise.
    pub source_relative: Option<String>,
    /// The editable nodes extracted from the file, flat for list-rendering.
    pub nodes: Vec<ConfigNodeView>,
}

impl ConfigFileView {
    pub fn empty(path: PathBuf) -> Self {
        Self {
            path,
            source_relative: None,
            nodes: Vec::new(),
        }
    }
}

/// One editable value inside a `ConfigFileView`.
///
/// # Why a flat list of nodes instead of a tree
///
/// The TOML the user writes is nested, but the UI surface a GUI renders
/// is usually flat (a properties panel, a form). Keeping the API flat
/// with a dotted `path` string trades a tiny bit of GUI cleverness for
/// a much simpler cross-language contract.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ConfigNodeView {
    /// Dotted TOML path (e.g. `"listener.port"`, `"service.rule_sets"`).
    pub path: String,
    /// Current value, stringified for display. A GUI can format this
    /// however it wants; the value's original type is in `kind`.
    pub display_value: String,
    /// What shape of value this node holds.
    pub kind: NodeKind,
    /// Whether the GUI should allow editing this field.
    pub editable: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum NodeKind {
    String,
    Integer,
    Boolean,
    StringList,
    Table,
}

/// A structured edit to apply to the workspace.
///
/// # Why commands instead of free-form text edits
///
/// The brief (§6.1, §6.2) spells this out: free-form text edits are too
/// coarse for a GUI that needs to reason about the change. Commands
/// carry exactly the intent — "add rule set", "update field at path" —
/// which the apply-layer can validate before writing anything.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EditCommand {
    /// Add a rule-set file to the workspace.
    AddRuleSet { path: String },
    /// Remove a rule-set file by its index in the list.
    RemoveRuleSet { index: usize },
    /// Update an editable field. `target` identifies which file; `path`
    /// is the dotted TOML path inside that file.
    UpdateField {
        target: EditTarget,
        path: String,
        value: EditValue,
    },
    /// Change the fallback respond dir.
    SetFallbackRespondDir { path: String },
}

/// Which file inside the workspace an edit applies to.
#[derive(Clone, Debug)]
pub enum EditTarget {
    Root,
    RuleSet { index: usize },
    Middleware { index: usize },
}

/// A value in an `UpdateField` edit. Kept intentionally small — this is
/// the union of types our TOML config actually uses.
#[derive(Clone, Debug)]
pub enum EditValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    StringList(Vec<String>),
}

/// What happened when an `EditCommand` was applied.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ApplyResult {
    /// Whether the edit succeeded.
    pub ok: bool,
    /// Files whose in-memory model was changed. Use this to drive the
    /// GUI's "unsaved changes" indicator.
    pub modified_files: Vec<PathBuf>,
    /// Problems found during apply. Non-empty when `ok` is false.
    pub diagnostics: Vec<Diagnostic>,
    /// Whether the running server would need a reload / restart to see
    /// this change. Informational — the server control layer decides
    /// what to do with the hint.
    pub reload_hint: ReloadHint,
}

/// How much of the server needs to restart after a change.
///
/// A GUI can show this as a banner ("restart required") and a supervisor
/// can use it to decide between reload-in-place and full restart.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum ReloadHint {
    /// No reload needed — change was purely cosmetic or to comments.
    None,
    /// Rule-set / middleware reload is sufficient.
    Reload,
    /// A full restart is needed (e.g. listener port changed).
    Restart,
}

/// Result of writing in-memory changes back to disk.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct SaveResult {
    pub ok: bool,
    /// Files actually written.
    pub written: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Human-readable notice about something in the workspace.
///
/// Severities follow the same convention as `RouteValidationIssue` in
/// the routing crate so a GUI can render them uniformly.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    /// Which file the diagnostic is about, if applicable.
    pub file: Option<PathBuf>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}
