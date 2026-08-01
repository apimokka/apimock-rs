//! `compute_diff_summary` and the per-rule diff walker.
//!
//! # Why this isn't a textual diff
//!
//! A line-by-line text diff would surface noise from formatting
//! (key reordering, comment loss). The GUI wants to know which
//! *logical* nodes the user changed — so we walk the node-address
//! space, compare the in-memory state to a re-parsed snapshot of
//! the baseline, and emit `DiffItem`s keyed by `NodeId`.
//!
//! # Granularity
//!
//! The current diff emits at three granularities:
//!
//! 1. **Per-rule** `Updated` / `Added` / `Removed` for changes inside
//!    a rule set whose top-level structure is otherwise stable.
//! 2. **Per-rule-set** `Added` for newly-introduced rule sets the
//!    baseline didn't have at all.
//! 3. **Root file** `Updated` when listener / log / service-level
//!    fields changed.

use std::path::PathBuf;

use crate::view::NodeId;

use super::Workspace;
use super::id_index::NodeAddress;
use super::path_helpers::file_basename;

impl Workspace {
    /// Compute the diff summary for the most recent save: one entry
    /// per node whose rendered representation has changed since load.
    ///
    /// # Why this isn't a textual diff
    ///
    /// A line-by-line text diff would surface noise from formatting
    /// (key reordering, comment loss). The GUI wants to know which
    /// *logical* nodes the user changed — so we walk the node-address
    /// space, compare the in-memory state to a re-parsed snapshot of
    /// the baseline, and emit `DiffItem`s keyed by `NodeId`.
    ///
    /// # Granularity
    ///
    /// 5.3.0 emits diffs at three granularities, in this order:
    ///
    /// 1. **Per-rule** `Updated` / `Added` / `Removed` for changes
    ///    inside a rule set whose top-level structure (rule count,
    ///    prefixes) is otherwise stable.
    /// 2. **Per-rule-set** `Added` for newly-introduced rule sets the
    ///    baseline didn't have at all.
    /// 3. **Root file** `Updated` when listener / log / service-level
    ///    fields changed.
    pub(super) fn compute_diff_summary(&self) -> Vec<crate::view::DiffItem> {
        use crate::view::{DiffItem, DiffKind};

        let mut out = Vec::new();

        // Per-rule diffs for rule sets that exist in baseline and current.
        for (rs_idx, rule_set) in self.config.service.rule_sets.iter().enumerate() {
            let path = PathBuf::from(rule_set.file_path.as_str());
            let rendered = crate::toml_writer::render_rule_set_toml(rule_set);
            let baseline_text = self.baseline_files.get(&path);
            let baseline_matches = baseline_text
                .map(|s| s.as_str() == rendered.as_str())
                .unwrap_or(false);
            if baseline_matches {
                continue;
            }

            if let Some(baseline) = baseline_text {
                // Both baseline and current exist — try a per-rule diff.
                self.append_per_rule_diff(rs_idx, rule_set, baseline, &mut out);
            } else {
                // Newly added rule set (no baseline file). Surface as
                // a single rule-set-level Added.
                if let Some(rs_id) = self.ids.id_for(NodeAddress::RuleSet { rule_set: rs_idx }) {
                    out.push(DiffItem {
                        kind: DiffKind::Added,
                        target: rs_id,
                        summary: format!(
                            "rule set #{} ({}): rules={}",
                            rs_idx + 1,
                            file_basename(&path),
                            rule_set.rules.len(),
                        ),
                    });
                }
            }
        }

        // Did the root file diverge?
        let root_rendered = crate::toml_writer::render_apimock_toml(&self.config);
        let root_baseline_matches = self
            .baseline_files
            .get(&self.root_path)
            .map(|s| s.as_str() == root_rendered.as_str())
            .unwrap_or(false);
        if !root_baseline_matches {
            if let Some(root_id) = self.ids.id_for(NodeAddress::Root) {
                out.push(DiffItem {
                    kind: DiffKind::Updated,
                    target: root_id,
                    summary: format!(
                        "{}: listener / log / service",
                        file_basename(&self.root_path)
                    ),
                });
            }
        }

        out
    }

    /// Walk the rules in `rule_set` against the baseline TOML's `rules`
    /// array, emitting one `DiffItem` per rule that changed.
    ///
    /// # Pairing strategy
    ///
    /// Matching by *index* (rule[0] vs baseline_rule[0], rule[1] vs
    /// baseline_rule[1], ...). After an insert / delete in the middle
    /// of a list, this would over-report — every rule past the
    /// insertion point would look "updated". A stage-5 candidate is to
    /// match by NodeId so insertions don't fan out. For 5.3.0,
    /// index-pairing is the simplest correct choice; the over-report
    /// is observably accurate (all those rules' on-disk positions
    /// *did* change) just not minimal.
    fn append_per_rule_diff(
        &self,
        rs_idx: usize,
        rule_set: &apimock_routing::RuleSet,
        baseline_text: &str,
        out: &mut Vec<crate::view::DiffItem>,
    ) {
        use crate::view::{DiffItem, DiffKind};
        use toml::Value;

        // Parse baseline back to a TOML value to walk its rules array.
        let baseline_value: Value = match toml::from_str(baseline_text) {
            Ok(v) => v,
            Err(_) => return, // baseline malformed; skip per-rule detail
        };
        let baseline_rules: &[Value] = match baseline_value
            .get("rules")
            .and_then(|v| v.as_array())
        {
            Some(arr) => arr.as_slice(),
            None => &[],
        };

        let cur_len = rule_set.rules.len();
        let base_len = baseline_rules.len();
        let common = cur_len.min(base_len);

        // Compare overlapping rules.
        for rule_idx in 0..common {
            let cur_rendered = rule_to_string(&rule_set.rules[rule_idx]);
            let base_rendered = toml::to_string_pretty(&baseline_rules[rule_idx])
                .unwrap_or_default();
            if cur_rendered == base_rendered {
                continue;
            }
            let rule_id = self
                .ids
                .id_for(NodeAddress::Rule { rule_set: rs_idx, rule: rule_idx })
                .unwrap_or_else(NodeId::new);
            out.push(DiffItem {
                kind: DiffKind::Updated,
                target: rule_id,
                summary: format!("rule #{} in rule set #{}", rule_idx + 1, rs_idx + 1),
            });

            // RFC 029: per-condition items within the changed rule.
            self.append_condition_diff(
                rs_idx, rule_idx,
                &rule_set.rules[rule_idx],
                &baseline_rules[rule_idx],
                out,
            );
        }

        // Rules added in the current model that weren't in baseline.
        for rule_idx in common..cur_len {
            let target = self
                .ids
                .id_for(NodeAddress::Rule {
                    rule_set: rs_idx,
                    rule: rule_idx,
                })
                .unwrap_or_else(NodeId::new);
            out.push(DiffItem {
                kind: DiffKind::Added,
                target,
                summary: format!(
                    "added rule #{} in rule set #{}",
                    rule_idx + 1,
                    rs_idx + 1
                ),
            });
        }

        // Rules removed: present in baseline, not in current. We
        // can't attribute these to a NodeId (the rule's id was deleted
        // from the index when DeleteRule ran), so we emit a fresh id
        // and a clear summary; the GUI surfaces these as removals.
        for rule_idx in common..base_len {
            out.push(DiffItem {
                kind: DiffKind::Removed,
                target: NodeId::new(),
                summary: format!(
                    "removed rule #{} from rule set #{}",
                    rule_idx + 1,
                    rs_idx + 1
                ),
            });
        }
    }
    /// Emit per-condition diff items for a single rule that is known to
    /// have changed. Called from `append_per_rule_diff` for overlapping
    /// rules only (added/removed rules don't need condition-level detail).
    fn append_condition_diff(
        &self,
        rs_idx: usize,
        rule_idx: usize,
        cur_rule: &apimock_routing::Rule,
        baseline_rule: &toml::Value,
        out: &mut Vec<crate::view::DiffItem>,
    ) {
        use crate::view::{DiffItem, DiffKind};

        // ── Header conditions ─────────────────────────────────────────

        // Collect current header keys.
        let cur_header_keys: std::collections::HashSet<&str> = cur_rule
            .when.request.headers.as_ref()
            .map(|h| h.0.keys().map(String::as_str).collect())
            .unwrap_or_default();

        // Collect baseline header keys from the TOML value.
        let base_header_keys: std::collections::HashSet<&str> = baseline_rule
            .get("when")
            .and_then(|w| w.get("request"))
            .and_then(|r| r.get("headers"))
            .and_then(|h| h.as_table())
            .map(|t| t.keys().map(String::as_str).collect())
            .unwrap_or_default();

        for key in cur_header_keys.difference(&base_header_keys) {
            let id = self.ids
                .id_for(NodeAddress::HeaderCondition {
                    rule_set: rs_idx, rule: rule_idx, header_name: key.to_string(),
                })
                .unwrap_or_else(NodeId::new);
            out.push(DiffItem {
                kind: DiffKind::HeaderConditionAdded,
                target: id,
                summary: format!("header `{}` added in rule #{}", key, rule_idx + 1),
            });
        }
        for key in base_header_keys.difference(&cur_header_keys) {
            out.push(DiffItem {
                kind: DiffKind::HeaderConditionRemoved,
                target: NodeId::new(),
                summary: format!("header `{}` removed from rule #{}", key, rule_idx + 1),
            });
        }

        // ── Body (JSON) conditions ────────────────────────────────────

        use apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind;
        let cur_body_paths: std::collections::HashSet<&str> = cur_rule
            .when.request.body.as_ref()
            .and_then(|b| b.0.get(&BodyKind::Json))
            .map(|m| m.keys().map(String::as_str).collect())
            .unwrap_or_default();

        let base_body_paths: std::collections::HashSet<&str> = baseline_rule
            .get("when")
            .and_then(|w| w.get("request"))
            .and_then(|r| r.get("body"))
            .and_then(|b| b.get("json"))
            .and_then(|j| j.as_table())
            .map(|t| t.keys().map(String::as_str).collect())
            .unwrap_or_default();

        for path in cur_body_paths.difference(&base_body_paths) {
            let id = self.ids
                .id_for(NodeAddress::BodyCondition {
                    rule_set: rs_idx, rule: rule_idx, path: path.to_string(),
                })
                .unwrap_or_else(NodeId::new);
            out.push(DiffItem {
                kind: DiffKind::BodyConditionAdded,
                target: id,
                summary: format!("body path `{}` added in rule #{}", path, rule_idx + 1),
            });
        }
        for path in base_body_paths.difference(&cur_body_paths) {
            out.push(DiffItem {
                kind: DiffKind::BodyConditionRemoved,
                target: NodeId::new(),
                summary: format!("body path `{}` removed from rule #{}", path, rule_idx + 1),
            });
        }
    }
}

/// Render a single rule to canonical TOML text. Used by per-rule diff
/// to compare baseline rules to current rules in a format-agnostic
/// way (the same canonicalisation `toml_writer` applies to whole
/// files).
fn rule_to_string(rule: &apimock_routing::Rule) -> String {
    let table = crate::toml_writer::rule_table(rule);
    toml::to_string_pretty(&toml::Value::Table(table)).unwrap_or_default()
}
