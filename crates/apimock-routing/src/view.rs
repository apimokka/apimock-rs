//! Read-only views on routing state for GUI tooling.
//!
//! # Stage-1 scope (5.0.0)
//!
//! Per the 5.0.0 brief, this module defines the *shape* of the read-only
//! API that a future GUI will depend on. The types are declared with
//! their fields and rustdoc-annotated responsibilities; populating them
//! from a live `RuleSet` is stage-2 work. We deliberately ship the
//! signatures first so that:
//!
//! 1. the GUI work can start against a frozen type surface,
//! 2. any downstream code (docs site, dashboards) can begin modelling
//!    against stable identifiers, and
//! 3. field additions in later stages are additive rather than
//!    reshaping.
//!
//! Every type here is `#[non_exhaustive]` so adding fields later is not
//! a breaking change.
//!
//! # What these types deliberately hide
//!
//! A `RouteCatalogSnapshot` does NOT include execution state (compiled
//! Rhai AST, open file handles, etc.). It is a photograph of the
//! declarative routing configuration at one moment — the kind of
//! information a GUI shows in a "routes" panel, not the live runtime.

use serde::Serialize;

pub mod build;

/// A complete snapshot of the server's routing configuration at one moment.
///
/// # Why a snapshot rather than a live reference
///
/// GUIs navigate, filter, and diff. A borrowed live reference would
/// require the GUI to hold a lock on the running server's state — not
/// feasible across an async boundary or an IPC channel. Snapshots are
/// cheap to clone, cheap to send, and never block the server.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RouteCatalogSnapshot {
    /// Rule sets in the same order as they would be evaluated at request time.
    pub rule_sets: Vec<RuleSetView>,
    /// Fallback respond dir (file-based zero-config responder).
    pub fallback_respond_dir: Option<String>,
    /// Top-level entries of the fallback respond dir, depth-1 eager.
    /// `None` when no fallback dir is configured or it doesn't exist
    /// on disk. Subdirectory contents are not pre-populated; the
    /// embedder calls `Workspace::list_directory(parent_id)` to expand
    /// nodes on demand.
    pub file_tree: Option<FileTreeView>,
    /// Middleware-script routes, keyed by `service.middlewares` order.
    pub script_routes: Vec<ScriptRouteView>,
}

impl RouteCatalogSnapshot {
    /// Return a snapshot with no content.
    pub fn empty() -> Self {
        Self {
            rule_sets: Vec::new(),
            fallback_respond_dir: None,
            file_tree: None,
            script_routes: Vec::new(),
        }
    }
}

/// GUI-facing view of one rule set.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RuleSetView {
    /// Index within the parent [`RouteCatalogSnapshot::rule_sets`] list.
    /// Identifies the rule set across edit commands.
    pub index: usize,
    /// Source file this rule set was loaded from (relative to the project).
    pub source_path: String,
    /// Optional URL path prefix shared across every rule in this set.
    pub url_path_prefix: Option<String>,
    /// Optional respond-dir prefix shared across every rule in this set.
    pub respond_dir_prefix: Option<String>,
    /// The rules, in evaluation order.
    pub rules: Vec<RuleView>,
}

/// GUI-facing view of one rule.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RuleView {
    /// Zero-based index within the parent rule set.
    pub index: usize,
    /// Structured match conditions. Spec §5.3 — URL / method /
    /// headers / JSON conditions. Each field is `Option`-typed so a
    /// rule that only matches on URL doesn't carry stub values in the
    /// other slots.
    pub when: WhenView,
    /// The declarative response shape.
    pub respond: RespondView,
}

impl RuleView {
    /// One-line text label for list-row rendering. Backwards-compat
    /// helper that produces the same shape as 5.0–5.2's
    /// `when_summary: String` field.
    pub fn summary(&self) -> String {
        self.when.summary()
    }
}

/// Structured representation of a rule's `when` clause (spec §5.3).
///
/// # Why each field is `Option`
///
/// A rule with `when.request.url_path = "/api"` and no other clauses
/// matches every request whose URL is `/api`, regardless of method or
/// headers. Carrying explicit `None`s for unset fields keeps the
/// distinction between "not constrained" and "constrained to empty"
/// — the GUI renders the former as a blank field and the latter as
/// "method = (none)".
#[derive(Clone, Debug, Default, Serialize)]
#[non_exhaustive]
pub struct WhenView {
    /// URL-path predicate as written in the rule. Carries the matching
    /// operator (`equals` / `starts_with` / `contains` / `wild_card`
    /// / `pattern`).
    pub url_path: Option<UrlPathView>,
    /// HTTP method — uppercase string like `"GET"` to match the
    /// underlying `HttpMethod::as_str()` representation.
    pub method: Option<String>,
    /// `true` iff the rule restricts on request headers. We don't
    /// surface the actual header conditions yet (the routing crate's
    /// `Headers` type isn't publicly inspectable at this stage); the
    /// boolean tells the GUI whether to render a "headers constraint
    /// present" badge.
    pub has_header_conditions: bool,
    /// `true` iff the rule restricts on request body JSON.
    pub has_body_conditions: bool,
}

impl WhenView {
    /// Compact human-readable summary built from the populated fields
    /// in priority order (method → path → constraint badges).
    pub fn summary(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(method) = self.method.as_deref() {
            parts.push(method.to_owned());
        }
        if let Some(url) = self.url_path.as_ref() {
            parts.push(url.summary());
        }
        if self.has_header_conditions {
            parts.push("+headers".to_owned());
        }
        if self.has_body_conditions {
            parts.push("+body".to_owned());
        }
        if parts.is_empty() {
            "(matches everything)".to_owned()
        } else {
            parts.join(" ")
        }
    }
}

/// URL-path predicate detail.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct UrlPathView {
    /// The path string from the rule, e.g. `"/api/v1/users"`.
    pub value: String,
    /// Matching operator name in lowercase TOML form, e.g.
    /// `"equals"` or `"starts_with"`.
    pub op: String,
}

impl UrlPathView {
    pub fn summary(&self) -> String {
        format!("{} {}", self.op, self.value)
    }
}

/// GUI-facing view of one response shape.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub enum RespondView {
    /// Serve a file. The path is resolved against the rule set's
    /// `respond_dir_prefix` at request time.
    File { path: String, csv_records_key: Option<String> },
    /// Return a literal text body. `status` is the response code to use
    /// (defaults to 200 when absent).
    Text { text: String, status: Option<u16> },
    /// Return an empty body with just this status code.
    Status { code: u16 },
}

/// Shown to the user when they ask "what rule would match *this* request?".
///
/// # Why we carry both the match and the non-matches
///
/// A GUI debugger doesn't just answer "which rule matched" — it answers
/// "why didn't the rule I expected match?". Surfacing the mismatches
/// lets the UI highlight the first failing predicate on each rule.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RouteMatchView {
    /// Matching rule, if any. `None` means the request would fall through
    /// to the dynamic-route fallback.
    pub matched: Option<MatchedRule>,
    /// Every rule the matcher considered before deciding, with the
    /// reason it was skipped. Order matches evaluation order.
    pub considered: Vec<MatchConsidered>,
}

#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct MatchedRule {
    pub rule_set_index: usize,
    pub rule_index: usize,
}

#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct MatchConsidered {
    pub rule_set_index: usize,
    pub rule_index: usize,
    /// Free-form text describing why this rule was skipped
    /// (e.g. `"url_path mismatch"`, `"header 'authorization' missing"`).
    pub reason: String,
}

/// Summary of every validation issue found in a [`RouteCatalogSnapshot`].
///
/// `ok` iff `issues` is empty; a GUI can render `ok = true` as a green
/// banner and iterate `issues` otherwise.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RouteValidation {
    pub ok: bool,
    pub issues: Vec<RouteValidationIssue>,
}

impl RouteValidation {
    pub fn ok() -> Self {
        Self {
            ok: true,
            issues: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RouteValidationIssue {
    /// Which rule-set this issue came from.
    pub rule_set_index: usize,
    /// Which rule within the set, if the issue is rule-scoped.
    pub rule_index: Option<usize>,
    /// Severity as the GUI should render it.
    pub severity: ValidationSeverity,
    /// Human-readable description.
    pub message: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

// ---------------------------------------------------------------------
// File-tree view (spec §5.5)
// ---------------------------------------------------------------------

/// Top-level view of the fallback respond directory, depth-1 eager.
///
/// # Why depth-1 and not full recursion
///
/// Fallback dirs in real projects can hold thousands of files; full
/// recursive enumeration would make `snapshot()` expensive. The
/// `Workspace` provides a separate `list_directory(parent_id)` API
/// the GUI calls when a user clicks to expand a subdirectory.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct FileTreeView {
    /// Absolute path to the fallback respond directory.
    pub root_path: String,
    /// Direct children of `root_path`. Subdirectories carry no
    /// children (`children: None`) — the embedder loads them on demand.
    pub entries: Vec<FileNodeView>,
}

#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct FileNodeView {
    /// Display name (just the last path component, e.g. `"users.json"`).
    pub name: String,
    /// Absolute path on disk.
    pub path: String,
    /// What kind of filesystem node this is.
    pub kind: FileNodeKind,
    /// For files only — the URL path that would serve this file under
    /// the dyn-route fallback (e.g. `"/users"` for `users.json`).
    /// `None` for directories.
    pub route_hint: Option<String>,
    /// `Some(empty)` for an unexpanded subdirectory, populated when
    /// the embedder calls `list_directory` to expand. Always `None`
    /// for files.
    pub children: Option<Vec<FileNodeView>>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum FileNodeKind {
    File,
    Directory,
}

// ---------------------------------------------------------------------
// Script-route view (spec §5)
// ---------------------------------------------------------------------

/// Minimal display info for a Rhai middleware script route.
///
/// # Why fields are intentionally minimal
///
/// A Rhai middleware can run arbitrary logic to decide whether to
/// match. Static analysis of "what URLs does this script handle" isn't
/// feasible without parsing Rhai (and would be unreliable even then).
/// The view reports only what we *do* know statically — file path and
/// display label — and leaves any deeper inspection to a hypothetical
/// future editor feature.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ScriptRouteView {
    /// Index within `service.middlewares_file_paths`.
    pub index: usize,
    /// Source file path as recorded in `service.middlewares`.
    pub source_file: String,
    /// Human-readable label (typically the file's basename).
    pub display_name: String,
}
