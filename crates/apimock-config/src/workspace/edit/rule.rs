//! Rule-level edit commands: add / update / delete / move a rule, and
//! update a rule's `respond` block.
//!
//! Split out of `edit.rs` (RFC 043) — a pure move, along the seam its
//! own section comments already marked. See `edit.rs`'s module doc for
//! why the split follows this shape.

use crate::error::ApplyError;
use crate::view::NodeId;

use super::super::Workspace;
use super::super::id_index::NodeAddress;
use super::payload::{build_respond_from_payload, build_rule_from_payload};

impl Workspace {
    pub(super) fn cmd_add_rule(
        &mut self,
        parent: NodeId,
        rule_payload: crate::view::RulePayload,
    ) -> Result<Vec<NodeId>, ApplyError> {
        let addr = self
            .ids
            .lookup(parent)
            .ok_or(ApplyError::UnknownNode { id: parent })?;
        let NodeAddress::RuleSet { rule_set: rs_idx } = addr else {
            return Err(ApplyError::WrongNodeKind {
                id: parent,
                reason: "expected a rule set id (parent for AddRule must be a rule set)".to_owned(),
            });
        };

        let rule_set = self
            .config
            .service
            .rule_sets
            .get_mut(rs_idx)
            .ok_or_else(|| ApplyError::InvalidPayload {
                reason: format!("rule set index {} out of range", rs_idx),
            })?;

        let new_rule = build_rule_from_payload(rule_payload, rule_set, rs_idx, None)?;
        let new_rule_idx = rule_set.rules.len();
        rule_set.rules.push(new_rule);

        let r_id = self.ids.insert(NodeAddress::Rule {
            rule_set: rs_idx,
            rule: new_rule_idx,
        });
        let resp_id = self.ids.insert(NodeAddress::Respond {
            rule_set: rs_idx,
            rule: new_rule_idx,
        });
        Ok(vec![parent, r_id, resp_id])
    }

    pub(super) fn cmd_update_rule(
        &mut self,
        id: NodeId,
        rule_payload: crate::view::RulePayload,
    ) -> Result<Vec<NodeId>, ApplyError> {
        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let NodeAddress::Rule {
            rule_set: rs_idx,
            rule: rule_idx,
        } = addr
        else {
            return Err(ApplyError::WrongNodeKind {
                id,
                reason: "expected a rule id".to_owned(),
            });
        };

        let rule_set = self
            .config
            .service
            .rule_sets
            .get_mut(rs_idx)
            .ok_or_else(|| ApplyError::InvalidPayload {
                reason: format!("rule set index {} out of range", rs_idx),
            })?;

        // Preserve headers / body match conditions that the GUI's
        // `RulePayload` doesn't expose — without this, every
        // `UpdateRule` would silently strip those clauses from the
        // existing rule. See `build_rule_from_payload`'s rustdoc.
        let existing = rule_set.rules.get(rule_idx).cloned();
        let new_rule = build_rule_from_payload(rule_payload, rule_set, rs_idx, existing.as_ref())?;
        *rule_set
            .rules
            .get_mut(rule_idx)
            .ok_or_else(|| ApplyError::InvalidPayload {
                reason: format!("rule index {} out of range", rule_idx),
            })? = new_rule;

        let resp_id = self
            .ids
            .id_for(NodeAddress::Respond {
                rule_set: rs_idx,
                rule: rule_idx,
            })
            .unwrap_or_default();
        Ok(vec![id, resp_id])
    }

    pub(super) fn cmd_delete_rule(&mut self, id: NodeId) -> Result<Vec<NodeId>, ApplyError> {
        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let NodeAddress::Rule {
            rule_set: rs_idx,
            rule: rule_idx,
        } = addr
        else {
            return Err(ApplyError::WrongNodeKind {
                id,
                reason: "expected a rule id".to_owned(),
            });
        };

        let rule_set = self
            .config
            .service
            .rule_sets
            .get_mut(rs_idx)
            .ok_or_else(|| ApplyError::InvalidPayload {
                reason: format!("rule set index {} out of range", rs_idx),
            })?;

        if rule_idx >= rule_set.rules.len() {
            return Err(ApplyError::InvalidPayload {
                reason: format!("rule index {} out of range", rule_idx),
            });
        }

        // Gather IDs that will change.
        let mut changed: Vec<NodeId> = Vec::new();
        changed.push(id);
        if let Some(resp_id) = self.ids.id_for(NodeAddress::Respond {
            rule_set: rs_idx,
            rule: rule_idx,
        }) {
            changed.push(resp_id);
        }

        rule_set.rules.remove(rule_idx);
        self.shift_rules_down(rs_idx, rule_idx);

        // Shifted rules' ids change their address but not their identity.
        let new_rule_count = self.config.service.rule_sets[rs_idx].rules.len();
        for shifted_idx in rule_idx..new_rule_count {
            if let Some(r_id) = self.ids.id_for(NodeAddress::Rule {
                rule_set: rs_idx,
                rule: shifted_idx,
            }) && !changed.contains(&r_id)
            {
                changed.push(r_id);
            }
            if let Some(resp_id) = self.ids.id_for(NodeAddress::Respond {
                rule_set: rs_idx,
                rule: shifted_idx,
            }) && !changed.contains(&resp_id)
            {
                changed.push(resp_id);
            }
        }

        Ok(changed)
    }

    pub(super) fn cmd_move_rule(
        &mut self,
        id: NodeId,
        new_index: usize,
    ) -> Result<Vec<NodeId>, ApplyError> {
        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let NodeAddress::Rule {
            rule_set: rs_idx,
            rule: old_idx,
        } = addr
        else {
            return Err(ApplyError::WrongNodeKind {
                id,
                reason: "expected a rule id".to_owned(),
            });
        };

        let rule_set = self
            .config
            .service
            .rule_sets
            .get_mut(rs_idx)
            .ok_or_else(|| ApplyError::InvalidPayload {
                reason: format!("rule set index {} out of range", rs_idx),
            })?;

        if old_idx >= rule_set.rules.len() || new_index >= rule_set.rules.len() {
            return Err(ApplyError::InvalidPayload {
                reason: format!(
                    "move out of bounds: old_idx={}, new_index={}, len={}",
                    old_idx,
                    new_index,
                    rule_set.rules.len()
                ),
            });
        }
        if old_idx == new_index {
            return Ok(vec![id]);
        }

        // Do the move in `config`.
        let rule = rule_set.rules.remove(old_idx);
        rule_set.rules.insert(new_index, rule);

        // Reshuffle IDs for all rules in this rule set: the simplest
        // correct approach is to pull out all rule+respond IDs for
        // this rule-set, reorder them to match the new slice order,
        // and re-insert.
        self.reorder_rule_ids(rs_idx, old_idx, new_index);

        // Every rule in [min(old, new) .. max(old, new)] changed address;
        // report their IDs so the GUI repaints.
        let lo = old_idx.min(new_index);
        let hi = old_idx.max(new_index);
        let mut changed: Vec<NodeId> = Vec::new();
        for idx in lo..=hi {
            if let Some(r_id) = self.ids.id_for(NodeAddress::Rule {
                rule_set: rs_idx,
                rule: idx,
            }) {
                changed.push(r_id);
            }
            if let Some(resp_id) = self.ids.id_for(NodeAddress::Respond {
                rule_set: rs_idx,
                rule: idx,
            }) {
                changed.push(resp_id);
            }
        }
        Ok(changed)
    }

    pub(super) fn cmd_update_respond(
        &mut self,
        id: NodeId,
        respond: crate::view::RespondPayload,
    ) -> Result<Vec<NodeId>, ApplyError> {
        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let NodeAddress::Respond {
            rule_set: rs_idx,
            rule: rule_idx,
        } = addr
        else {
            return Err(ApplyError::WrongNodeKind {
                id,
                reason: "expected a respond id".to_owned(),
            });
        };

        let rule = self
            .config
            .service
            .rule_sets
            .get_mut(rs_idx)
            .and_then(|rs| rs.rules.get_mut(rule_idx))
            .ok_or_else(|| ApplyError::InvalidPayload {
                reason: format!("rule at rule_set={}, rule={} not found", rs_idx, rule_idx),
            })?;

        rule.respond = build_respond_from_payload(respond);

        // Re-run status-code derivation so the updated `status` field
        // has its matching `StatusCode` stored.
        let rule_set = &self.config.service.rule_sets[rs_idx];
        let derived = rule_set.rules[rule_idx].compute_derived_fields(rule_set, rule_idx, rs_idx);
        self.config.service.rule_sets[rs_idx].rules[rule_idx] = derived;

        Ok(vec![id])
    }
}
