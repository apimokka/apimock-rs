//! Read-only views on workspace state, and the command + result types
//! the editing API uses.
//!
//! # 5.1.0 — spec alignment
//!
//! In 5.0.0 this module carried a placeholder shape defined only by
//! rustdoc; 5.1.0 re-aligns it with the 5.1 spec:
//!
//! - `WorkspaceSnapshot { files, routes, diagnostics }` (spec §4.2)
//! - each node carries `id: NodeId` + `source_file` + `toml_path` +
//!   `display_name` + `kind` + `validation`
//! - `EditCommand` is eight variants covering every editable action
//!   (spec §4.3)
//! - `ApplyResult { changed_nodes, diagnostics, requires_reload }`
//!   (spec §4.4)
//! - `SaveResult { changed_files, diff_summary, requires_reload }`
//!   (spec §4.5) — populated in Step 4
//! - `ValidationReport { diagnostics, is_valid }` (spec §4.6)
//! - `Diagnostic { node_id, file, severity, message }` (spec §4.7)
//!
//! # Why UUIDs and not positional IDs
//!
//! The spec's §4.3 says "すべて NodeId で対象を指定". Positional IDs
//! (`rule_sets[0].rules[3]`) would shift on every insert / delete /
//! move, forcing the GUI to re-index its selection set after every
//! edit. UUIDs are stable within a `Workspace` instance regardless of
//! reordering.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use std::path::PathBuf;

use apimock_routing::view::RouteCatalogSnapshot;

/// Stable identifier for an editable node.
///
/// # Stability contract
///
/// Stable within one `Workspace` instance — that is, across any
/// sequence of `apply()` calls. IDs are reassigned on fresh `load()`,
/// which matches spec §10 "Workspace はメモリ上に独立インスタンスを持つ".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Complete snapshot of the workspace state.
///
/// Shape matches spec §4.2 exactly. Consumed read-only by the GUI;
/// mutated indirectly via `Workspace::apply`.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct WorkspaceSnapshot {
    /// All editable TOML files in the workspace, flattened. Each file
    /// carries its own list of editable nodes.
    pub files: Vec<ConfigFileView>,
    /// Route overview pulled from the routing crate.
    pub routes: RouteCatalogSnapshot,
    /// Workspace-scoped issues (e.g. a root file that failed to load).
    /// Per-node diagnostics live inside each `ConfigNodeView.validation`.
    pub diagnostics: Vec<Diagnostic>,
}

impl WorkspaceSnapshot {
    pub fn empty() -> Self {
        Self {
            files: Vec::new(),
            routes: RouteCatalogSnapshot::empty(),
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
    /// Display name — typically the file name. Used as a tab title in
    /// the GUI.
    pub display_name: String,
    /// What kind of file this is (root config, rule set, middleware).
    pub kind: ConfigFileKind,
    /// Editable nodes extracted from the file.
    pub nodes: Vec<ConfigNodeView>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ConfigFileKind {
    Root,
    RuleSet,
    Middleware,
}

/// One editable value inside a `ConfigFileView`.
///
/// Each node carries the six fields spec §4.2 makes mandatory.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ConfigNodeView {
    /// Stable identifier — survives moves / renames within a Workspace
    /// instance.
    pub id: NodeId,
    /// File the node was loaded from.
    pub source_file: PathBuf,
    /// Dotted TOML path inside `source_file` (e.g. `"listener.port"`,
    /// `"rules[2].respond"`).
    pub toml_path: String,
    /// Human-readable label for UI list rendering (e.g. the rule's
    /// `url_path` value, or `"Rule #3"` for a rule without one).
    pub display_name: String,
    /// Shape of the underlying value.
    pub kind: NodeKind,
    /// Per-node validation results.
    pub validation: NodeValidation,
}

/// What shape of value a node holds. The variants are what the
/// spec-defined `EditCommand` variants act on.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum NodeKind {
    /// Root config node — listener / log / service fields.
    RootSetting,
    /// One rule set loaded from a referenced TOML file.
    RuleSet,
    /// One rule inside a rule set.
    Rule,
    /// The `respond` block of a rule.
    Respond,
    /// File-based response node (fallback dir entry).
    FileNode,
    /// Script / middleware route.
    Script,
}

/// Per-node validation result.
///
/// # Why validation is a field on the node and not a separate pass
///
/// GUIs render validation inline ("this field has a red underline").
/// Keeping the validation result stapled to the node the GUI is about
/// to render avoids a second lookup step in every render frame.
#[derive(Clone, Debug, Default, Serialize)]
pub struct NodeValidation {
    /// Convenience flag — true iff `issues` is empty.
    pub ok: bool,
    /// Human-readable issues scoped to this node.
    pub issues: Vec<ValidationIssue>,
}

impl NodeValidation {
    pub fn ok() -> Self {
        Self {
            ok: true,
            issues: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
}

/// Structured edit command applied via `Workspace::apply`.
///
/// # Shape comes straight from spec §4.3
///
/// Each variant targets a node by NodeId (never by positional index).
/// This guarantees edits remain well-defined across previous inserts /
/// removes in the same GUI session.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EditCommand {
    /// Add a rule set file to the workspace.
    ///
    /// `path` is relative to the root config's directory — the same
    /// convention as the value stored in `service.rule_sets`.
    AddRuleSet {
        path: String,
    },
    /// Remove a rule set by its NodeId. The underlying TOML file is
    /// NOT deleted from disk — the workspace only removes the reference.
    RemoveRuleSet {
        id: NodeId,
    },
    /// Add a rule to an existing rule set.
    AddRule {
        parent: NodeId,
        rule: RulePayload,
    },
    /// Update a rule's `when` / `respond` block.
    ///
    /// # Preservation of unspecified fields
    ///
    /// `RulePayload` carries `url_path`, `method`, and `respond` —
    /// the fields a stage-1 GUI form exposes. A rule may also carry
    /// `headers` and `body.json` match conditions that aren't part of
    /// the payload shape. Those clauses are **preserved** across an
    /// `UpdateRule`: the new rule keeps whatever headers / body
    /// conditions the previous rule had, even though the payload
    /// doesn't mention them.
    ///
    /// Without this preservation, every `UpdateRule` would silently
    /// strip the unsurfaced clauses, which is a save-time bug when a
    /// GUI re-saves a rule it loaded from a hand-edited TOML file.
    UpdateRule {
        id: NodeId,
        rule: RulePayload,
    },
    /// Remove a rule by NodeId.
    DeleteRule {
        id: NodeId,
    },
    /// Reorder a rule within its parent rule set.
    MoveRule {
        id: NodeId,
        new_index: usize,
    },
    /// Update the `respond` block of a rule.
    UpdateRespond {
        id: NodeId,
        respond: RespondPayload,
    },
    /// Update a root-level setting (listener, log, service-level flags).
    UpdateRootSetting {
        key: RootSettingKey,
        value: EditValue,
    },
}

/// Payload for `AddRule` / `UpdateRule`.
#[derive(Clone, Debug, Default)]
pub struct RulePayload {
    pub url_path: Option<String>,
    pub method: Option<String>,
    pub respond: RespondPayload,
}

/// Payload for `UpdateRespond`.
///
/// The three fields are mutually specialised: exactly one of
/// `file_path` / `text` / `status` should be populated. Validation
/// catches cases that violate this.
#[derive(Clone, Debug, Default)]
pub struct RespondPayload {
    pub file_path: Option<String>,
    pub text: Option<String>,
    pub status: Option<u16>,
    pub delay_milliseconds: Option<u32>,
}

/// Enumerated root-level setting. Typed enum rather than free-form
/// path so the apply-layer can exhaustively match without parsing.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum RootSettingKey {
    ListenerIpAddress,
    ListenerPort,
    ServiceFallbackRespondDir,
    ServiceStrategy,
}

/// Value provided with an edit command.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EditValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    StringList(Vec<String>),
    /// For settings whose domain is a small enum value (e.g.
    /// `ServiceStrategy` → `"first_match"`).
    Enum(String),
    /// For completeness — callers can pass a raw JSON value when the
    /// spec-defined key set is extended by stage-3 tooling. Currently
    /// reserved; no stage-1 setting uses it.
    Json(JsonValue),
}

/// Outcome of a successful `apply`.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ApplyResult {
    /// Node IDs whose content (or position) changed.
    pub changed_nodes: Vec<NodeId>,
    /// Issues surfaced by applying the command (validation during apply
    /// may add diagnostics — e.g. a new rule pointing at a missing file).
    pub diagnostics: Vec<Diagnostic>,
    /// `true` iff the server should reload to see this change. An edit
    /// that changes the listener port needs a restart, not just a
    /// reload — see `Workspace::save` for the richer `ReloadHint`.
    pub requires_reload: bool,
}

/// Outcome of `Workspace::save`.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct SaveResult {
    /// TOML files actually written to disk.
    pub changed_files: Vec<PathBuf>,
    /// One entry per node that changed since last load.
    pub diff_summary: Vec<DiffItem>,
    pub requires_reload: bool,
}

/// One summary row in a `SaveResult::diff_summary`.
#[derive(Clone, Debug, Serialize)]
pub struct DiffItem {
    pub kind: DiffKind,
    pub target: NodeId,
    pub summary: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum DiffKind {
    Added,
    Updated,
    Removed,
}

/// Workspace-wide validation result. Mirrors spec §4.6.
#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
    pub is_valid: bool,
}

impl ValidationReport {
    pub fn ok() -> Self {
        Self {
            diagnostics: Vec::new(),
            is_valid: true,
        }
    }
}

/// Human-readable notice about the workspace.
#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    /// Target node, if any. `None` means "workspace-wide".
    pub node_id: Option<NodeId>,
    /// Target file, if the diagnostic is best reported at file level
    /// (e.g. "could not read apimock-rule-set.toml"). May be `None` for
    /// purely in-memory errors.
    pub file: Option<PathBuf>,
    pub severity: Severity,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

// ---------------------------------------------------------------------------
// Reload hint — spec §9. The same enum shape was defined in 5.0.0;
// 5.1 reuses it unchanged so existing consumers keep working.
// ---------------------------------------------------------------------------

/// Advisory indicating what, if anything, the server needs to do in
/// response to a config change.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ReloadHint {
    pub requires_reload: bool,
    pub requires_restart: bool,
}

impl ReloadHint {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn reload() -> Self {
        Self {
            requires_reload: true,
            requires_restart: false,
        }
    }

    pub fn restart() -> Self {
        Self {
            requires_reload: false,
            requires_restart: true,
        }
    }
}
