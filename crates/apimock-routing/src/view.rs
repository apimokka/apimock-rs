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
}

impl RouteCatalogSnapshot {
    /// Return a snapshot with no content. Used as the stage-1 placeholder
    /// and as the "no rule sets configured" shape.
    pub fn empty() -> Self {
        Self {
            rule_sets: Vec::new(),
            fallback_respond_dir: None,
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
    /// Human-readable summary of the match conditions (e.g.
    /// `"GET /api/v1/users starts_with"`). GUI uses this for list rows.
    pub when_summary: String,
    /// The declarative response shape.
    pub respond: RespondView,
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
