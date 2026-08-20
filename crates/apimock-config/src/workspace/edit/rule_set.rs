//! Rule-set-level edit commands: add/remove a rule set, and the
//! per-rule-set strategy override (RFC 025).
//!
//! Split out of `edit.rs` (RFC 043) — a pure move, along the seam its
//! own section comments already marked. See `edit.rs`'s module doc for
//! why the split follows this shape.

use std::path::Path;

use apimock_routing::RuleSet;

use crate::error::ApplyError;
use crate::view::NodeId;

use super::super::Workspace;
use super::super::id_index::NodeAddress;
use super::payload::internal_path_err;

impl Workspace {
    pub(super) fn cmd_add_rule_set(&mut self, path: String) -> Result<Vec<NodeId>, ApplyError> {
        // Resolve the path against the root's parent dir (same
        // convention as `Config::new`), then load the rule set.
        let relative_dir = self.config_relative_dir().map_err(internal_path_err)?;
        let joined = Path::new(&relative_dir).join(&path);
        let path_str = joined.to_str().ok_or_else(|| ApplyError::InvalidPayload {
            reason: format!(
                "path contains non-UTF-8 bytes: {}",
                joined.to_string_lossy()
            ),
        })?;

        let next_idx = self.config.service.rule_sets.len();
        let new_rule_set =
            RuleSet::new(path_str, relative_dir.as_str(), next_idx).map_err(|e| {
                ApplyError::InvalidPayload {
                    reason: format!("failed to load rule set `{}`: {}", path, e),
                }
            })?;

        // Record the path in service.rule_sets_file_paths too so
        // `save()` persists the change later.
        let file_paths = self
            .config
            .service
            .rule_sets_file_paths
            .get_or_insert_with(Vec::new);
        file_paths.push(path.clone());

        let new_len = self.config.service.rule_sets.len() + 1;
        self.config.service.rule_sets.push(new_rule_set);

        // Mint IDs for the new rule set + its rules + responds.
        let rs_addr = NodeAddress::RuleSet { rule_set: next_idx };
        let rs_id = self.ids.insert(rs_addr);
        let mut changed = vec![rs_id];
        let new_rs = &self.config.service.rule_sets[next_idx];
        for rule_idx in 0..new_rs.rules.len() {
            let r_id = self.ids.insert(NodeAddress::Rule {
                rule_set: next_idx,
                rule: rule_idx,
            });
            let resp_id = self.ids.insert(NodeAddress::Respond {
                rule_set: next_idx,
                rule: rule_idx,
            });
            changed.push(r_id);
            changed.push(resp_id);
        }
        // Sanity: new_len is purely informational here, but makes
        // the invariant explicit to anyone reading the code.
        debug_assert_eq!(new_len, self.config.service.rule_sets.len());

        Ok(changed)
    }

    pub(super) fn cmd_remove_rule_set(&mut self, id: NodeId) -> Result<Vec<NodeId>, ApplyError> {
        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let NodeAddress::RuleSet { rule_set: idx } = addr else {
            return Err(ApplyError::WrongNodeKind {
                id,
                reason: "expected a rule set id".to_owned(),
            });
        };

        let len = self.config.service.rule_sets.len();
        if idx >= len {
            return Err(ApplyError::InvalidPayload {
                reason: format!("rule set index {} out of range (len={})", idx, len),
            });
        }

        // Collect IDs that will change: the removed one plus every rule
        // set (+ rules + responds) whose index shifts down by one.
        let mut changed: Vec<NodeId> = Vec::new();
        // the rule-set itself and its internal nodes (removed)
        changed.push(id);
        if let Some(removed_rs) = self.config.service.rule_sets.get(idx) {
            for rule_idx in 0..removed_rs.rules.len() {
                if let Some(r_id) = self.ids.id_for(NodeAddress::Rule {
                    rule_set: idx,
                    rule: rule_idx,
                }) {
                    changed.push(r_id);
                }
                if let Some(resp_id) = self.ids.id_for(NodeAddress::Respond {
                    rule_set: idx,
                    rule: rule_idx,
                }) {
                    changed.push(resp_id);
                }
            }
        }

        // Actually remove.
        self.config.service.rule_sets.remove(idx);
        if let Some(paths) = self.config.service.rule_sets_file_paths.as_mut()
            && idx < paths.len()
        {
            paths.remove(idx);
        }

        // Migrate IDs: everything at `idx` onwards in the *old* layout
        // needs its address remapped. The clean approach: gather the
        // old (address → id) pairs we care about, clear the entries
        // affected by the shift, re-insert with new addresses.
        self.shift_rule_sets_down(idx);

        // Every shifted rule set's ID remains valid but its address
        // has changed; surface those IDs too so the GUI refreshes
        // their position indicators.
        for shifted_idx in idx..self.config.service.rule_sets.len() {
            if let Some(shifted_id) = self.ids.id_for(NodeAddress::RuleSet {
                rule_set: shifted_idx,
            }) && !changed.contains(&shifted_id)
            {
                changed.push(shifted_id);
            }
        }

        Ok(changed)
    }

    // ── RFC 025: per-rule-set strategy ───────────────────────────────────

    pub(super) fn cmd_update_rule_set_strategy(
        &mut self,
        id: crate::view::NodeId,
        strategy_name: Option<String>,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::strategy::{PriorityTiebreaker, Strategy};

        let rs_idx = match self.ids.lookup(id) {
            Some(NodeAddress::RuleSet { rule_set }) => rule_set,
            _ => return Err(ApplyError::UnknownNode { id }),
        };

        let strategy = match strategy_name.as_deref() {
            None | Some("") => None,
            Some(s) => {
                let parsed = match s {
                    "first_match" => Strategy::FirstMatch,
                    "uniform_random" => Strategy::UniformRandom { seed: None },
                    "weighted_random" => Strategy::WeightedRandom { seed: None },
                    "priority" => Strategy::Priority {
                        tiebreaker: PriorityTiebreaker::FirstMatch,
                    },
                    "round_robin" => Strategy::RoundRobin,
                    _ => {
                        return Err(ApplyError::InvalidPayload {
                            reason: format!(
                                "unknown strategy {:?}; expected one of: \
                             first_match, uniform_random, weighted_random, \
                             priority, round_robin",
                                s
                            ),
                        });
                    }
                };
                Some(parsed)
            }
        };

        self.config.service.rule_sets[rs_idx].strategy = strategy;

        let rs_id = self
            .ids
            .id_for(NodeAddress::RuleSet { rule_set: rs_idx })
            .unwrap_or(id);
        Ok(vec![rs_id])
    }
}
