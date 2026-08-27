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
///
/// # RFC 065 — kept in step with `Respond::validate` by hand
///
/// `json` is a third mutually-exclusive body source alongside
/// `file_path`/`text` — every check below that used to enumerate two
/// now enumerates three, and inline `json` / a referenced `.json`
/// file's content get the same JSON5-parse check `Respond::validate`
/// does at load time, so a GUI diagnostic and `apimock validate`'s own
/// pass never disagree about the same rule. Caught by this RFC's own
/// tests: a config using `respond.json` loaded fine (`Respond::validate`
/// already knew about `json`) but `apimock validate` still reported
/// "requires at least one of file_path, text, or status" and exited 1
/// — this function's own copy of the check hadn't been told `json`
/// existed.
pub(super) fn respond_node_validation(
    respond: &apimock_routing::Respond,
    rule_set: &RuleSet,
    rule_idx: usize,
    rs_idx: usize,
) -> NodeValidation {
    // `Respond::validate` logs errors but returns a `Result<(), String>`
    // (see its own doc comment). For 5.1 per-node validation we want
    // structured messages — so we replicate the specific checks here
    // rather than piping through that string.
    let mut issues: Vec<ValidationIssue> = Vec::new();

    let any = respond.file_path.is_some()
        || respond.text.is_some()
        || respond.json.is_some()
        || respond.status.is_some();
    if !any {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "response requires at least one of file_path, text, json, or status"
                .to_owned(),
        });
    }
    let body_sources_set = [
        respond.file_path.is_some(),
        respond.text.is_some(),
        respond.json.is_some(),
    ]
    .into_iter()
    .filter(|&set| set)
    .count();
    if body_sources_set > 1 {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "file_path, text and json are mutually exclusive; only one may be set"
                .to_owned(),
        });
    }
    if respond.file_path.is_some() && respond.status.is_some() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "status cannot be combined with file_path (only with text or json)".to_owned(),
        });
    }

    if let Some(json_str) = respond.json.as_ref()
        && let Err(e) = json5::from_str::<serde_json::Value>(json_str)
    {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: format!(
                "invalid json (rule #{} in rule set #{}): {}",
                rule_idx + 1,
                rs_idx + 1,
                e
            ),
        });
    }

    // file-existence (and, for `.json`/`.json5`, content) validation:
    // the same behaviour `Respond::validate(dir_prefix, …)` performs.
    // We don't call it directly because it writes to `log::error!`,
    // which would flood the console during every GUI snapshot.
    if let Some(file_path) = respond.file_path.as_ref() {
        let dir_prefix = rule_set.dir_prefix();
        let p = Path::new(dir_prefix.as_str()).join(file_path);
        if !p.exists() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!(
                    "file not found: {} (rule #{} in rule set #{})",
                    display_path(&p),
                    rule_idx + 1,
                    rs_idx + 1,
                ),
            });
        } else {
            let is_json_like = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .is_some_and(|e| e == "json" || e == "json5");
            if is_json_like {
                match std::fs::read_to_string(&p) {
                    Ok(content) => {
                        if let Err(e) = json5::from_str::<serde_json::Value>(content.as_str()) {
                            issues.push(ValidationIssue {
                                severity: Severity::Error,
                                message: format!(
                                    "`{}` is not valid JSON (rule #{} in rule set #{}): {}",
                                    display_path(&p),
                                    rule_idx + 1,
                                    rs_idx + 1,
                                    e
                                ),
                            });
                        }
                    }
                    Err(e) => {
                        issues.push(ValidationIssue {
                            severity: Severity::Error,
                            message: format!(
                                "failed to read `{}` (rule #{} in rule set #{}): {}",
                                display_path(&p),
                                rule_idx + 1,
                                rs_idx + 1,
                                e
                            ),
                        });
                    }
                }
            }
        }
    }

    NodeValidation {
        ok: issues.is_empty(),
        issues,
    }
}

/// `Path::join` performs no normalisation — `RuleSet::dir_prefix()` is
/// itself already `"./."` for the common case (no `[prefix]` block, a
/// config sitting in the current directory), so joining it with a bare
/// `file_path` renders as `"././bad.json"` in a diagnostic message
/// (RFC 065 review, F-1). Mirrors `apimock_routing::rule_set::rule::
/// respond`'s own private `display_path` — not shared across the crate
/// boundary for one small formatting helper neither side has any
/// reason to keep depending on the other for.
fn display_path(p: &Path) -> String {
    use std::path::Component;
    let cleaned: PathBuf = p
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    if cleaned.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        cleaned.to_string_lossy().into_owned()
    }
}
