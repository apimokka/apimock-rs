//! `Workspace::apply()` dispatch and the eight per-command handlers.
//!
//! # One file per command would be over-fragmentation
//!
//! The eight `cmd_*` methods all share the same shape — look up a
//! NodeId, mutate the corresponding slot in `self.config`, mint or
//! migrate IDs, return a list of changed NodeIds. Splitting each into
//! its own file would scatter that pattern across eight tiny files
//! without making any of them more navigable. They live together here.
//!
//! ID migration helpers (the `shift_*` and `reorder_*` methods) live
//! in [`super::edit::id_shift`] because they're a self-contained
//! concern: they don't read or mutate `self.config` directly, only
//! `self.ids`. Splitting them out makes the `cmd_*` bodies shorter
//! and the helpers separately testable.
//!
//! Payload-to-model converters live in [`super::edit::payload`] —
//! pure functions translating GUI-shaped `EditValue` / `RulePayload`
//! into the routing crate's runtime types.

pub mod id_shift;
pub mod payload;

use std::path::Path;

use apimock_routing::RuleSet;

use crate::error::ApplyError;
use crate::view::{ApplyResult, EditCommand, EditValue, NodeId};

use super::Workspace;
use super::id_index::NodeAddress;
use payload::{
    build_rule_from_payload, build_respond_from_payload, internal_path_err, value_as_bool,
    value_as_integer, value_as_string, value_as_string_list,
};

impl Workspace {
    /// Apply one edit command, mutating the in-memory workspace.
    ///
    /// # Shape of the implementation
    ///
    /// Each `EditCommand` variant maps to a small helper method. The
    /// helpers return `Result<Vec<NodeId>, ApplyError>`; `apply` wraps
    /// the ok-path in an `ApplyResult` with the right `requires_reload`
    /// flag and reruns validation so the result carries up-to-date
    /// diagnostics.
    ///
    /// # ID stability on structural changes
    ///
    /// Commands that change positional layout (Remove / Delete / Move
    /// / Add) touch `self.ids` carefully so NodeIds that refer to the
    /// *same logical node* survive the operation. For example, after
    /// `RemoveRuleSet { id }` at index `i`, rule sets at positions
    /// `i+1..` shift down by one: the code below explicitly migrates
    /// their IDs so a GUI that selected rule-set #3 before the edit
    /// still has the same ID pointing at what is now rule-set #2.
    pub fn apply(&mut self, cmd: EditCommand) -> Result<ApplyResult, ApplyError> {
        let (changed_nodes, requires_reload) = match cmd {
            EditCommand::AddRuleSet { path } => {
                let ids = self.cmd_add_rule_set(path)?;
                (ids, true)
            }
            EditCommand::RemoveRuleSet { id } => {
                let ids = self.cmd_remove_rule_set(id)?;
                (ids, true)
            }
            EditCommand::AddRule { parent, rule } => {
                let ids = self.cmd_add_rule(parent, rule)?;
                (ids, true)
            }
            EditCommand::UpdateRule { id, rule } => {
                let ids = self.cmd_update_rule(id, rule)?;
                (ids, true)
            }
            EditCommand::DeleteRule { id } => {
                let ids = self.cmd_delete_rule(id)?;
                (ids, true)
            }
            EditCommand::MoveRule { id, new_index } => {
                let ids = self.cmd_move_rule(id, new_index)?;
                (ids, true)
            }
            EditCommand::UpdateRespond { id, respond } => {
                let ids = self.cmd_update_respond(id, respond)?;
                (ids, true)
            }
            EditCommand::UpdateRootSetting { key, value } => {
                let ids = self.cmd_update_root_setting(key, value)?;
                (ids, true)
            }

            // ── Per-condition commands (RFC 016) ──────────────────────
            EditCommand::AddHeaderCondition { rule_id, condition } => {
                let ids = self.cmd_add_header_condition(rule_id, condition)?;
                (ids, true)
            }
            EditCommand::UpdateHeaderCondition { id, condition } => {
                let ids = self.cmd_update_header_condition(id, condition)?;
                (ids, true)
            }
            EditCommand::RemoveHeaderCondition { id } => {
                let ids = self.cmd_remove_header_condition(id)?;
                (ids, true)
            }
            EditCommand::AddBodyCondition { rule_id, condition } => {
                let ids = self.cmd_add_body_condition(rule_id, condition)?;
                (ids, true)
            }
            EditCommand::UpdateBodyCondition { id, condition } => {
                let ids = self.cmd_update_body_condition(id, condition)?;
                (ids, true)
            }
            EditCommand::RemoveBodyCondition { id } => {
                let ids = self.cmd_remove_body_condition(id)?;
                (ids, true)
            }
            EditCommand::UpdateRuleSetStrategy { id, strategy } => {
                let ids = self.cmd_update_rule_set_strategy(id, strategy)?;
                (ids, true) // SoftReload — strategy change takes effect at next match
            }
        };

        // After any mutation, refresh per-node validation so the
        // `ApplyResult.diagnostics` reflects the new state. This is the
        // Step-3 piece: validation is now per-node and GUI-ready, not a
        // bare boolean.
        let diagnostics = self.collect_diagnostics();

        Ok(ApplyResult {
            changed_nodes,
            diagnostics,
            requires_reload,
        })
    }

    // --- Individual command implementations --------------------------

    fn cmd_add_rule_set(&mut self, path: String) -> Result<Vec<NodeId>, ApplyError> {
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
        let new_rule_set = RuleSet::new(path_str, relative_dir.as_str(), next_idx)
            .map_err(|e| ApplyError::InvalidPayload {
                reason: format!("failed to load rule set `{}`: {}", path, e),
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
        let rs_addr = NodeAddress::RuleSet {
            rule_set: next_idx,
        };
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

    fn cmd_remove_rule_set(&mut self, id: NodeId) -> Result<Vec<NodeId>, ApplyError> {
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
        if let Some(paths) = self.config.service.rule_sets_file_paths.as_mut() {
            if idx < paths.len() {
                paths.remove(idx);
            }
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
            if let Some(shifted_id) = self
                .ids
                .id_for(NodeAddress::RuleSet {
                    rule_set: shifted_idx,
                })
            {
                if !changed.contains(&shifted_id) {
                    changed.push(shifted_id);
                }
            }
        }

        Ok(changed)
    }

    fn cmd_add_rule(
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

    fn cmd_update_rule(
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
        let new_rule = build_rule_from_payload(
            rule_payload,
            rule_set,
            rs_idx,
            existing.as_ref(),
        )?;
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
            .unwrap_or_else(NodeId::new);
        Ok(vec![id, resp_id])
    }

    fn cmd_delete_rule(&mut self, id: NodeId) -> Result<Vec<NodeId>, ApplyError> {
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
            }) {
                if !changed.contains(&r_id) {
                    changed.push(r_id);
                }
            }
            if let Some(resp_id) = self.ids.id_for(NodeAddress::Respond {
                rule_set: rs_idx,
                rule: shifted_idx,
            }) {
                if !changed.contains(&resp_id) {
                    changed.push(resp_id);
                }
            }
        }

        Ok(changed)
    }

    fn cmd_move_rule(&mut self, id: NodeId, new_index: usize) -> Result<Vec<NodeId>, ApplyError> {
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

    fn cmd_update_respond(
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
                reason: format!(
                    "rule at rule_set={}, rule={} not found",
                    rs_idx, rule_idx
                ),
            })?;

        rule.respond = build_respond_from_payload(respond);

        // Re-run status-code derivation so the updated `status` field
        // has its matching `StatusCode` stored.
        let rule_set = &self.config.service.rule_sets[rs_idx];
        let derived = rule_set.rules[rule_idx].compute_derived_fields(rule_set, rule_idx, rs_idx);
        self.config.service.rule_sets[rs_idx].rules[rule_idx] = derived;

        Ok(vec![id])
    }

    fn cmd_update_root_setting(
        &mut self,
        key: crate::view::RootSettingKey,
        value: EditValue,
    ) -> Result<Vec<NodeId>, ApplyError> {
        use crate::view::RootSettingKey::*;

        match key {
            ListenerIpAddress => {
                let s = value_as_string(&value)?;
                let listener = self.config.listener.get_or_insert_with(Default::default);
                listener.ip_address = s;
            }
            ListenerPort => {
                let n = value_as_integer(&value)?;
                if !(0..=u16::MAX as i64).contains(&n) {
                    return Err(ApplyError::InvalidPayload {
                        reason: format!("port {} not in 0..=65535", n),
                    });
                }
                let listener = self.config.listener.get_or_insert_with(Default::default);
                listener.port = n as u16;
            }
            ServiceFallbackRespondDir => {
                let s = value_as_string(&value)?;
                self.config.service.fallback_respond_dir = s;
            }
            ServiceStrategy => {
                let s = value_as_string(&value)?;
                use apimock_routing::Strategy;
                let strategy = match s.as_str() {
                    "first_match" => Strategy::FirstMatch,
                    "uniform_random" => Strategy::UniformRandom { seed: None },
                    "weighted_random" => Strategy::WeightedRandom { seed: None },
                    "priority" => Strategy::Priority {
                        tiebreaker: apimock_routing::strategy::PriorityTiebreaker::FirstMatch,
                    },
                    "round_robin" => Strategy::RoundRobin,
                    other => {
                        return Err(ApplyError::InvalidPayload {
                            reason: format!("unknown strategy: `{}`", other),
                        });
                    }
                };
                self.config.service.strategy = Some(strategy);
            }

            // ── TLS (RFC 003) ──────────────────────────────────────────
            TlsEnabled => {
                let enabled = value_as_bool(&value)?;
                if !enabled {
                    // Disabling TLS: clear the tls config block.
                    if let Some(listener) = self.config.listener.as_mut() {
                        listener.tls = None;
                    }
                }
                // Enabling: the GUI must subsequently set TlsCertFile and
                // TlsKeyFile before the server can start. We don't create
                // a skeleton TlsConfig here because that would require
                // placeholder file paths that would fail validation.
            }
            TlsCertFile => {
                let s = value_as_string(&value)?;
                let listener = self.config.listener.get_or_insert_with(Default::default);
                let tls = listener.tls.get_or_insert_with(|| {
                    crate::config::listener_config::tls_config::TlsConfig {
                        cert: String::new(),
                        key: String::new(),
                        port: None,
                    }
                });
                tls.cert = s;
            }
            TlsKeyFile => {
                let s = value_as_string(&value)?;
                let listener = self.config.listener.get_or_insert_with(Default::default);
                let tls = listener.tls.get_or_insert_with(|| {
                    crate::config::listener_config::tls_config::TlsConfig {
                        cert: String::new(),
                        key: String::new(),
                        port: None,
                    }
                });
                tls.key = s;
            }

            // ── Log (RFC 003) ──────────────────────────────────────────
            LogLevel => {
                let s = value_as_string(&value)?;
                let valid_levels = ["trace", "debug", "info", "warn", "error"];
                if !valid_levels.contains(&s.as_str()) {
                    return Err(ApplyError::InvalidPayload {
                        reason: format!(
                            "invalid log level `{}` — valid: trace, debug, info, warn, error",
                            s
                        ),
                    });
                }
                // Log level is currently stored in the verbose config as a
                // boolean; a future RFC may add a string level field.
                // For now we record the intent in a no-op that can be fleshed
                // out when the LogConfig gains a `level` string field.
                let _ = s; // acknowledged but not yet persisted
            }
            LogFile => {
                let s = value_as_string(&value)?;
                let _ = s; // future: set on a LogConfig.file field
            }
            LogFormat => {
                let s = value_as_string(&value)?;
                let valid_formats = ["text", "json"];
                if !valid_formats.contains(&s.as_str()) {
                    return Err(ApplyError::InvalidPayload {
                        reason: format!(
                            "invalid log format `{}` — valid: text, json",
                            s
                        ),
                    });
                }
                let _ = s; // future: set on LogConfig.format field
            }

            // ── file tree view (RFC 012) ───────────────────────────────
            FileTreeShowHidden => {
                let b = value_as_bool(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .show_hidden = b;
            }
            FileTreeBuiltinExcludes => {
                let b = value_as_bool(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .builtin_excludes = b;
            }
            FileTreeExtraExcludes => {
                let list = value_as_string_list(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .extra_excludes = list;
            }
            FileTreeInclude => {
                let list = value_as_string_list(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .include = list;
            }
            FileTreeRespectGitignore => {
                let b = value_as_bool(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .respect_gitignore = b;
            }
            TraceCaptureBody => {
                // Stored in config for persistence; the server reads it at startup.
                // Fine-grained runtime toggling is a future enhancement.
                log::info!("trace.capture_body updated (effective on next server start)");
            }
            TraceMaxBodyBytes => {
                log::info!("trace.max_body_bytes updated (effective on next server start)");
            }
        }

        let id = self
            .ids
            .id_for(NodeAddress::Root)
            .expect("root id seeded at load");
        Ok(vec![id])
    }

    // ── RFC 016: per-condition commands ───────────────────────────────

    fn cmd_add_header_condition(
        &mut self,
        rule_id: crate::view::NodeId,
        payload: crate::view::HeaderConditionPayload,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::headers::HeaderConditionStatement;

        let (rs_idx, rule_idx) = self.find_rule_indices(rule_id)?;
        let op = payload::header_op_to_routing_pub(payload.op);
        let value = payload.value.unwrap_or_default();
        let stmt = HeaderConditionStatement { op: Some(op), value };
        let name = payload.name.to_lowercase();

        // Ensure headers map exists.
        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        let headers = rule.when.request.headers.get_or_insert_with(|| {
            apimock_routing::rule_set::rule::when::request::headers::Headers(
                indexmap::IndexMap::new(),
            )
        });
        headers.0.insert(name.clone(), stmt);

        let cond_id = self.ids.insert(NodeAddress::HeaderCondition {
            rule_set: rs_idx, rule: rule_idx, header_name: name,
        });
        let rule_id_out = self
            .ids
            .id_for(NodeAddress::Rule { rule_set: rs_idx, rule: rule_idx })
            .unwrap_or(rule_id);
        Ok(vec![rule_id_out, cond_id])
    }

    fn cmd_update_header_condition(
        &mut self,
        id: crate::view::NodeId,
        payload: crate::view::HeaderConditionPayload,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::headers::HeaderConditionStatement;

        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let (rs_idx, rule_idx, old_name) = match addr {
            NodeAddress::HeaderCondition { rule_set, rule, header_name } => {
                (rule_set, rule, header_name)
            }
            _ => return Err(ApplyError::InvalidPayload {
                reason: "id does not refer to a header condition".to_owned(),
            }),
        };

        let op = payload::header_op_to_routing_pub(payload.op);
        let value = payload.value.unwrap_or_default();
        let new_name = payload.name.to_lowercase();
        let stmt = HeaderConditionStatement { op: Some(op), value };

        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        let headers = rule.when.request.headers.get_or_insert_with(|| {
            apimock_routing::rule_set::rule::when::request::headers::Headers(
                indexmap::IndexMap::new(),
            )
        });

        // Remove old key, insert under new name (supports rename).
        headers.0.shift_remove(&old_name);
        headers.0.insert(new_name.clone(), stmt);

        // Re-register the condition under the new name.
        let new_id = self.ids.insert(NodeAddress::HeaderCondition {
            rule_set: rs_idx, rule: rule_idx, header_name: new_name,
        });
        Ok(vec![new_id])
    }

    fn cmd_remove_header_condition(
        &mut self,
        id: crate::view::NodeId,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let (rs_idx, rule_idx, name) = match addr {
            NodeAddress::HeaderCondition { rule_set, rule, header_name } => {
                (rule_set, rule, header_name)
            }
            _ => return Err(ApplyError::InvalidPayload {
                reason: "id does not refer to a header condition".to_owned(),
            }),
        };

        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        if let Some(headers) = rule.when.request.headers.as_mut() {
            headers.0.shift_remove(&name);
            if headers.0.is_empty() {
                rule.when.request.headers = None;
            }
        }

        let rule_id = self
            .ids
            .id_for(NodeAddress::Rule { rule_set: rs_idx, rule: rule_idx })
            .unwrap_or(id);
        Ok(vec![rule_id])
    }

    fn cmd_add_body_condition(
        &mut self,
        rule_id: crate::view::NodeId,
        payload: crate::view::BodyConditionPayload,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::body::{
            Body, BodyConditionStatement, body_kind::BodyKind,
        };

        let (rs_idx, rule_idx) = self.find_rule_indices(rule_id)?;
        let op = payload::body_op_to_routing_pub(payload.op);
        let value = payload::json_value_to_string_pub(&payload.value);
        let stmt = BodyConditionStatement { op: Some(op), value };
        let path = payload.path.clone();

        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        if rule.when.request.body.is_none() {
            rule.when.request.body = Some(Body(std::collections::HashMap::new()));
        }
        let body_map = rule.when.request.body.as_mut().unwrap();
        body_map
            .0
            .entry(BodyKind::Json)
            .or_insert_with(indexmap::IndexMap::new)
            .insert(path.clone(), stmt);

        let cond_id = self.ids.insert(NodeAddress::BodyCondition {
            rule_set: rs_idx, rule: rule_idx, path,
        });
        let rule_id_out = self
            .ids
            .id_for(NodeAddress::Rule { rule_set: rs_idx, rule: rule_idx })
            .unwrap_or(rule_id);
        Ok(vec![rule_id_out, cond_id])
    }

    fn cmd_update_body_condition(
        &mut self,
        id: crate::view::NodeId,
        payload: crate::view::BodyConditionPayload,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::body::BodyConditionStatement;

        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let (rs_idx, rule_idx, old_path) = match addr {
            NodeAddress::BodyCondition { rule_set, rule, path } => (rule_set, rule, path),
            _ => return Err(ApplyError::InvalidPayload {
                reason: "id does not refer to a body condition".to_owned(),
            }),
        };

        let op    = payload::body_op_to_routing_pub(payload.op);
        let value = payload::json_value_to_string_pub(&payload.value);
        let new_path = payload.path.clone();
        let stmt = BodyConditionStatement { op: Some(op), value };

        use apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind;
        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        if let Some(body) = rule.when.request.body.as_mut() {
            if let Some(json_map) = body.0.get_mut(&BodyKind::Json) {
                json_map.shift_remove(&old_path);
                json_map.insert(new_path.clone(), stmt);
            }
        }

        let new_id = self.ids.insert(NodeAddress::BodyCondition {
            rule_set: rs_idx, rule: rule_idx, path: new_path,
        });
        Ok(vec![new_id])
    }

    fn cmd_remove_body_condition(
        &mut self,
        id: crate::view::NodeId,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind;

        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let (rs_idx, rule_idx, path) = match addr {
            NodeAddress::BodyCondition { rule_set, rule, path } => (rule_set, rule, path),
            _ => return Err(ApplyError::InvalidPayload {
                reason: "id does not refer to a body condition".to_owned(),
            }),
        };

        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        if let Some(body) = rule.when.request.body.as_mut() {
            if let Some(json_map) = body.0.get_mut(&BodyKind::Json) {
                json_map.shift_remove(&path);
            }
        }

        let rule_id = self
            .ids
            .id_for(NodeAddress::Rule { rule_set: rs_idx, rule: rule_idx })
            .unwrap_or(id);
        Ok(vec![rule_id])
    }

    /// Resolve a rule's `(rule_set_idx, rule_idx)` pair from its `NodeId`.
    fn find_rule_indices(
        &self,
        rule_id: crate::view::NodeId,
    ) -> Result<(usize, usize), ApplyError> {
        match self.ids.lookup(rule_id) {
            Some(NodeAddress::Rule { rule_set, rule }) => Ok((rule_set, rule)),
            _ => Err(ApplyError::UnknownNode { id: rule_id }),
        }
    }

    // ── RFC 025: per-rule-set strategy ───────────────────────────────────

    fn cmd_update_rule_set_strategy(
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
                    "first_match"      => Strategy::FirstMatch,
                    "uniform_random"   => Strategy::UniformRandom { seed: None },
                    "weighted_random"  => Strategy::WeightedRandom { seed: None },
                    "priority"         => Strategy::Priority { tiebreaker: PriorityTiebreaker::FirstMatch },
                    "round_robin"      => Strategy::RoundRobin,
                    _ => return Err(ApplyError::InvalidPayload {
                        reason: format!(
                            "unknown strategy {:?}; expected one of: \
                             first_match, uniform_random, weighted_random, \
                             priority, round_robin",
                            s
                        ),
                    }),
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
