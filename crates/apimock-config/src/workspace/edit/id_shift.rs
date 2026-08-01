//! ID-migration helpers for structural edits.
//!
//! # Why these are split out from the cmd_* methods
//!
//! Every command that *moves* nodes (Remove / Delete / Move) has to
//! do two things: mutate `self.config`, then re-key `self.ids` so the
//! NodeIds the GUI was holding still resolve to the right addresses.
//! The mutation logic varies per command but the ID migration is
//! mechanical: walk the affected `NodeAddress`es, drop their old
//! entries, re-insert under the new positional indices.
//!
//! Keeping that mechanical pass in its own module means each
//! `cmd_*` body reads as "mutate config; call shift / reorder helper;
//! return changed IDs" instead of dragging another 50 lines of map
//! manipulation inline.
//!
//! # ID stability invariant
//!
//! These helpers preserve every NodeId that still has an address in
//! the post-edit layout. A rule whose position shifts from index 3
//! to index 2 keeps its NodeId; only its `NodeAddress` changes.
//! That's what makes a GUI selection survive a deletion in the
//! middle of a list.

use crate::view::NodeId;

use super::super::Workspace;
use super::super::id_index::NodeAddress;

impl Workspace {
    /// After a rule set is removed at `removed_idx`, migrate every ID
    /// whose address referenced a later rule set to its new index.
    pub(super) fn shift_rule_sets_down(&mut self, removed_idx: usize) {
        // Walk current layout (after removal). For each surviving
        // rule_set at new index `new_idx`, the *old* index was
        // `new_idx` if `new_idx < removed_idx` (no shift needed) or
        // `new_idx + 1` if `new_idx >= removed_idx` (it shifted down).
        // We rebuild mappings only for the shifted half.
        let new_rs_count = self.config.service.rule_sets.len();

        // First drop any stale ID entries for the removed index and
        // for everything whose old address will be replaced.
        // Collect stale (old) addresses first, then update `self.ids`.
        let mut stale: Vec<NodeAddress> = Vec::new();
        stale.push(NodeAddress::RuleSet {
            rule_set: removed_idx,
        });
        // The old index range is [removed_idx, new_rs_count+1).
        for old_idx in removed_idx..new_rs_count + 1 {
            stale.push(NodeAddress::RuleSet { rule_set: old_idx });
            // We don't know the old rule counts any more, so we walk
            // the id index for matches.
        }

        // Safer approach: pull all entries whose address's rule_set
        // field is >= removed_idx (both Rule and Respond and RuleSet),
        // and rebuild them.
        let mut to_migrate: Vec<(NodeId, NodeAddress)> = Vec::new();
        for (addr, &id) in self.ids.address_to_id.iter() {
            match addr {
                NodeAddress::RuleSet { rule_set } if *rule_set >= removed_idx => {
                    to_migrate.push((id, addr.clone()));
                }
                NodeAddress::Rule { rule_set, .. } if *rule_set >= removed_idx => {
                    to_migrate.push((id, addr.clone()));
                }
                NodeAddress::Respond { rule_set, .. } if *rule_set >= removed_idx => {
                    to_migrate.push((id, addr.clone()));
                }
                _ => {}
            }
        }

        for (id, addr) in &to_migrate {
            self.ids.address_to_id.remove(addr); // addr already cloned in to_migrate
            self.ids.id_to_address.remove(id);
        }

        // Re-insert with shifted addresses, skipping anything that
        // belonged to the removed rule set.
        for (id, addr) in to_migrate {
            let new_addr = match addr {
                NodeAddress::RuleSet { rule_set } => {
                    if rule_set == removed_idx {
                        continue; // removed outright
                    }
                    NodeAddress::RuleSet {
                        rule_set: rule_set - 1,
                    }
                }
                NodeAddress::Rule { rule_set, rule } => {
                    if rule_set == removed_idx {
                        continue;
                    }
                    NodeAddress::Rule {
                        rule_set: rule_set - 1,
                        rule,
                    }
                }
                NodeAddress::Respond { rule_set, rule } => {
                    if rule_set == removed_idx {
                        continue;
                    }
                    NodeAddress::Respond {
                        rule_set: rule_set - 1,
                        rule,
                    }
                }
                other => other,
            };
            self.ids.id_to_address.insert(id, new_addr.clone());
            self.ids.address_to_id.insert(new_addr, id);
        }
    }

    /// After a rule is deleted from `rule_set_idx` at position
    /// `removed_rule_idx`, shift IDs for later rules in the same set.
    pub(super) fn shift_rules_down(&mut self, rule_set_idx: usize, removed_rule_idx: usize) {
        let mut to_migrate: Vec<(NodeId, NodeAddress)> = Vec::new();
        for (addr, &id) in self.ids.address_to_id.iter() {
            match addr {
                NodeAddress::Rule { rule_set, rule }
                    if *rule_set == rule_set_idx && *rule >= removed_rule_idx =>
                {
                    to_migrate.push((id, addr.clone()));
                }
                NodeAddress::Respond { rule_set, rule }
                    if *rule_set == rule_set_idx && *rule >= removed_rule_idx =>
                {
                    to_migrate.push((id, addr.clone()));
                }
                _ => {}
            }
        }

        for (id, addr) in &to_migrate {
            self.ids.address_to_id.remove(addr); // addr already cloned in to_migrate
            self.ids.id_to_address.remove(id);
        }

        for (id, addr) in to_migrate {
            let new_addr = match addr {
                NodeAddress::Rule { rule_set, rule } => {
                    if rule == removed_rule_idx {
                        continue;
                    }
                    NodeAddress::Rule {
                        rule_set,
                        rule: rule - 1,
                    }
                }
                NodeAddress::Respond { rule_set, rule } => {
                    if rule == removed_rule_idx {
                        continue;
                    }
                    NodeAddress::Respond {
                        rule_set,
                        rule: rule - 1,
                    }
                }
                other => other,
            };
            self.ids.id_to_address.insert(id, new_addr.clone());
            self.ids.address_to_id.insert(new_addr, id);
        }
    }

    /// After a rule in `rule_set_idx` moves from `old_idx` to
    /// `new_idx`, shuffle the IDs of every rule between those indices.
    pub(super) fn reorder_rule_ids(&mut self, rule_set_idx: usize, old_idx: usize, new_idx: usize) {
        // Grab current mapping for all rules in this rule set.
        let rule_count = self.config.service.rule_sets[rule_set_idx].rules.len();
        let mut rule_ids: Vec<Option<NodeId>> = (0..rule_count)
            .map(|r| {
                self.ids.id_for(NodeAddress::Rule {
                    rule_set: rule_set_idx,
                    rule: r,
                })
            })
            .collect();
        let mut resp_ids: Vec<Option<NodeId>> = (0..rule_count)
            .map(|r| {
                self.ids.id_for(NodeAddress::Respond {
                    rule_set: rule_set_idx,
                    rule: r,
                })
            })
            .collect();

        // Before the config move, `rule_ids[old_idx]` held the moving
        // rule's old ID. But the config mutation already happened —
        // so the id_for lookups above are pre-migration (the ids
        // didn't change), they simply don't match the new layout yet.
        // We mimic the same move on `rule_ids`:
        let moving_r = rule_ids.remove(old_idx);
        rule_ids.insert(new_idx, moving_r);
        let moving_resp = resp_ids.remove(old_idx);
        resp_ids.insert(new_idx, moving_resp);

        // Clear old mappings for these addresses and repopulate.
        for r in 0..rule_count {
            let rule_addr = NodeAddress::Rule {
                rule_set: rule_set_idx,
                rule: r,
            };
            let resp_addr = NodeAddress::Respond {
                rule_set: rule_set_idx,
                rule: r,
            };
            if let Some(prev_id) = self.ids.address_to_id.remove(&rule_addr) {
                self.ids.id_to_address.remove(&prev_id);
            }
            if let Some(prev_id) = self.ids.address_to_id.remove(&resp_addr) {
                self.ids.id_to_address.remove(&prev_id);
            }
        }
        for (r, id_opt) in rule_ids.into_iter().enumerate() {
            let addr = NodeAddress::Rule {
                rule_set: rule_set_idx,
                rule: r,
            };
            let id = id_opt.unwrap_or_else(NodeId::new);
            self.ids.id_to_address.insert(id, addr.clone());
            self.ids.address_to_id.insert(addr, id);
        }
        for (r, id_opt) in resp_ids.into_iter().enumerate() {
            let addr = NodeAddress::Respond {
                rule_set: rule_set_idx,
                rule: r,
            };
            let id = id_opt.unwrap_or_else(NodeId::new);
            self.ids.id_to_address.insert(id, addr.clone());
            self.ids.address_to_id.insert(addr, id);
        }
    }
}
