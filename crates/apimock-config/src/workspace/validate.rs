//! `Workspace::validate()` and the per-node validation walker.
//!
//! # Why per-node validation lives here, not in `Respond` itself
//!
//! The routing crate's `Respond::validate()` writes errors to
//! `log::error!` and returns a bool. That's good enough for startup
//! validation (where the user reads stderr), but a GUI needs
//! structured `(severity, message, target_id)` triples it can render
//! inline. We replicate the rule logic here so the GUI gets diagnostic
//! objects without flooding the log every snapshot.
//!
//! # Used by both `validate()` and `snapshot()`
//!
//! The same `respond_node_validation` function backs both code paths,
//! so a node rendered with a red underline in the snapshot will also
//! appear in `ValidationReport::diagnostics`. Single source of truth.

use std::path::{Path, PathBuf};

use apimock_routing::RuleSet;

use crate::view::{Diagnostic, NodeValidation, Severity, ValidationIssue, ValidationReport};

use super::Workspace;
use super::id_index::NodeAddress;

impl Workspace {
    /// Walk every node, asking it for its validation state, and return
    /// the flat list of issues. Used at apply-time and on demand from
    /// `validate()`.
    pub(super) fn collect_diagnostics(&self) -> Vec<Diagnostic> {
        let mut out: Vec<Diagnostic> = Vec::new();
        for (rs_idx, rule_set) in self.config.service.rule_sets.iter().enumerate() {
            for (rule_idx, rule) in rule_set.rules.iter().enumerate() {
                let nv = respond_node_validation(&rule.respond, rule_set, rule_idx, rs_idx);
                if nv.ok {
                    continue;
                }
                let resp_id = self.ids.id_for(NodeAddress::Respond {
                    rule_set: rs_idx,
                    rule: rule_idx,
                });
                for issue in nv.issues {
                    out.push(Diagnostic {
                        node_id: resp_id,
                        file: Some(PathBuf::from(rule_set.file_path.as_str())),
                        severity: issue.severity,
                        message: issue.message,
                    });
                }
            }
        }

        // Root-level check: fallback_respond_dir must exist.
        if !Path::new(self.config.service.fallback_respond_dir.as_str()).exists() {
            out.push(Diagnostic {
                node_id: self.ids.id_for(NodeAddress::FallbackRespondDir),
                file: Some(self.root_path.clone()),
                severity: Severity::Error,
                message: format!(
                    "fallback_respond_dir does not exist: {}",
                    self.config.service.fallback_respond_dir
                ),
            });
        }

        out
    }

    // --- Public API ----

    /// Validate the workspace and return a GUI-ready report.
    ///
    /// Uses the same per-node checks `snapshot()` does so the numbers
    /// line up: a node rendered with a red underline in the snapshot
    /// will appear in `report.diagnostics` with the same message.
    pub fn validate(&self) -> ValidationReport {
        let diagnostics = self.collect_diagnostics();
        let is_valid = !diagnostics
            .iter()
            .any(|d| matches!(d.severity, Severity::Error));
        ValidationReport {
            diagnostics,
            is_valid,
        }
    }
}

/// Build a `NodeValidation` for one `Respond` block.
pub(super) fn respond_node_validation(
    respond: &apimock_routing::Respond,
    rule_set: &RuleSet,
    rule_idx: usize,
    rs_idx: usize,
) -> NodeValidation {
    // `Respond::validate` logs errors but returns a bool. For 5.1
    // per-node validation we want structured messages — so we replicate
    // the specific checks here rather than piping through the logger.
    let mut issues: Vec<ValidationIssue> = Vec::new();

    let any = respond.file_path.is_some() || respond.text.is_some() || respond.status.is_some();
    if !any {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "response requires at least one of file_path, text, or status".to_owned(),
        });
    }
    if respond.file_path.is_some() && respond.text.is_some() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "file_path and text cannot both be set".to_owned(),
        });
    }
    if respond.file_path.is_some() && respond.status.is_some() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "status cannot be combined with file_path (only with text)".to_owned(),
        });
    }

    // file-existence validation: this is the same behaviour the old
    // `Respond::validate(dir_prefix, …)` performed. We don't call it
    // directly because it writes to `log::error!`, which would flood
    // the console during every GUI snapshot.
    if let Some(file_path) = respond.file_path.as_ref() {
        let dir_prefix = rule_set.dir_prefix();
        let p = Path::new(dir_prefix.as_str()).join(file_path);
        if !p.exists() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!(
                    "file not found: {} (rule #{} in rule set #{})",
                    p.to_string_lossy(),
                    rule_idx + 1,
                    rs_idx + 1,
                ),
            });
        }
    }

    NodeValidation {
        ok: issues.is_empty(),
        issues,
    }
}
