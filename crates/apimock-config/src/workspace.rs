//! The editable workspace: loaded TOML + stable node IDs + edit API.
//!
//! # Role in the 5.1 design
//!
//! A GUI never touches `Config` or `RuleSet` directly. It holds a
//! `Workspace` value, calls `snapshot()` to get a read-only view for
//! rendering, and `apply(EditCommand)` to mutate. Later, `save()`
//! writes changes back to disk.
//!
//! # Stage breakdown
//!
//! 5.1.0 implements Steps 1–3 of the spec's §12 plan:
//!
//! - **Step 1** (this file) — `load` + `snapshot`.
//! - **Step 2** — `apply` with the full eight-command set.
//! - **Step 3** — `validate` producing a `ValidationReport`.
//!
//! Steps 4 (`save` + diff) and 5 (richer routing snapshot) are planned
//! for 5.2+.
//!
//! # IDs
//!
//! Every editable node gets a v4 UUID at load time. IDs are stable
//! across `apply()` calls within one `Workspace` instance, so GUI
//! selection survives edits that reorder or rename surrounding nodes.
//! IDs are *not* stable across fresh `load()` calls — a reload
//! regenerates the table, which matches the spec §10 "Workspace は
//! メモリ上に独立インスタンスを持つ" stance.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use apimock_routing::{RoutingError, RuleSet};

use crate::{
    Config,
    error::{ApplyError, ConfigError, SaveError, WorkspaceError},
    view::{
        ApplyResult, ConfigFileKind, ConfigFileView, ConfigNodeView, Diagnostic, EditCommand,
        EditValue, NodeId, NodeKind, NodeValidation, SaveResult, Severity, ValidationIssue,
        ValidationReport, WorkspaceSnapshot,
    },
};

/// Editable view of an apimock workspace.
///
/// # Internal layout
///
/// The `Workspace` holds the loaded TOML model (as a `Config`) plus
/// two index maps:
///
/// - `id_to_address`: NodeId → where the node lives in `config`.
/// - `address_to_id`: reverse — used when rebuilding snapshots.
///
/// On every `apply()` that could move nodes around (Add / Remove /
/// Move), these tables are partially rebuilt. Reloading the config
/// discards them and re-seeds with fresh IDs.
pub struct Workspace {
    /// Path this workspace was loaded from.
    root_path: PathBuf,
    /// Loaded TOML model. Authoritative source of truth for
    /// persistence; edits happen through the editable helpers on
    /// `Workspace` which keep `config` + id tables in sync.
    config: Config,
    /// ID index — see struct doc.
    ids: IdIndex,
    /// Workspace-scope diagnostics (e.g. load-time warnings). Per-node
    /// diagnostics live inside each node's `NodeValidation`.
    diagnostics: Vec<Diagnostic>,
}

/// Internal index mapping NodeId to an editable node's logical
/// address.
///
/// # Why a separate enum and not a path string
///
/// The `apply` layer needs to mutate the underlying config, which is
/// only safe if the address is a closed, exhaustively-matchable set.
/// A free-form `"rule_sets[0].rules[2]"` string would force the apply
/// code to parse at every edit and silently accept nonsense paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum NodeAddress {
    /// The root config (there is exactly one).
    Root,
    /// A whole rule set, identified by its index in `service.rule_sets`.
    RuleSet { rule_set: usize },
    /// A single rule inside a rule set.
    Rule { rule_set: usize, rule: usize },
    /// The `respond` block of a rule.
    Respond { rule_set: usize, rule: usize },
    /// A middleware file reference, by its index in
    /// `service.middlewares_file_paths`.
    Middleware { middleware: usize },
    /// The fallback respond dir. Singleton — there is one per workspace.
    FallbackRespondDir,
}

#[derive(Default)]
struct IdIndex {
    id_to_address: HashMap<NodeId, NodeAddress>,
    address_to_id: HashMap<NodeAddress, NodeId>,
}

impl IdIndex {
    fn insert(&mut self, address: NodeAddress) -> NodeId {
        if let Some(&id) = self.address_to_id.get(&address) {
            return id;
        }
        let id = NodeId::new();
        self.id_to_address.insert(id, address);
        self.address_to_id.insert(address, id);
        id
    }

    /// Lookup a NodeAddress by id. Used by the apply layer in Step 2.
    #[allow(dead_code)]
    fn lookup(&self, id: NodeId) -> Option<NodeAddress> {
        self.id_to_address.get(&id).copied()
    }

    fn id_for(&self, address: NodeAddress) -> Option<NodeId> {
        self.address_to_id.get(&address).copied()
    }
}

impl Workspace {
    /// Load a workspace rooted at the given `apimock.toml`-like path.
    ///
    /// Accepts either a direct path to the config file or the
    /// directory containing one; a missing file-path is searched for
    /// as `apimock.toml` inside `root`. Mirrors the CLI's existing
    /// resolution rules.
    pub fn load(root: PathBuf) -> Result<Self, WorkspaceError> {
        let resolved = resolve_root(&root)?;

        // Re-use `Config::new` so rule-set loading + validation go
        // through the same path as the running server. This is
        // important — the spec's "GUI doesn't break running server
        // behaviour" invariant (§13) is easiest to guarantee if both
        // paths share the same loader.
        let config_path_string = resolved.to_string_lossy().into_owned();
        let config = Config::new(Some(&config_path_string), None).map_err(WorkspaceError::from)?;

        let mut workspace = Self {
            root_path: resolved,
            config,
            ids: IdIndex::default(),
            diagnostics: Vec::new(),
        };
        workspace.seed_ids();
        Ok(workspace)
    }

    /// Assign a fresh NodeId to every editable address in `config`.
    /// Called from `load` and from any `apply()` path that might
    /// change the address of existing nodes.
    ///
    /// # Why we rebuild rather than patch
    ///
    /// `NodeAddress` carries positional indices (`rule_set: usize`).
    /// When a rule is deleted from the middle of a list, every rule
    /// after it gets a new index, so its `NodeAddress` changes. The
    /// GUI's NodeId must *not* change — that's the whole point of
    /// UUIDs — so this function preserves the existing
    /// address_to_id mapping where addresses still exist and only
    /// mints new IDs for genuinely new addresses.
    ///
    /// For Step 1 there's nothing to preserve: load is a from-scratch
    /// operation. Step 2 will call a more careful `reseed_after_edit`.
    fn seed_ids(&mut self) {
        // Root is always present.
        self.ids.insert(NodeAddress::Root);

        // Fallback respond dir is always present — even if the user
        // hasn't set it, it has a default value.
        self.ids.insert(NodeAddress::FallbackRespondDir);

        // Rule sets + their rules + respond blocks.
        for (rs_idx, rule_set) in self.config.service.rule_sets.iter().enumerate() {
            self.ids.insert(NodeAddress::RuleSet { rule_set: rs_idx });
            for (rule_idx, _rule) in rule_set.rules.iter().enumerate() {
                self.ids.insert(NodeAddress::Rule {
                    rule_set: rs_idx,
                    rule: rule_idx,
                });
                self.ids.insert(NodeAddress::Respond {
                    rule_set: rs_idx,
                    rule: rule_idx,
                });
            }
        }

        // Middleware references.
        if let Some(paths) = self.config.service.middlewares_file_paths.as_ref() {
            for mw_idx in 0..paths.len() {
                self.ids
                    .insert(NodeAddress::Middleware { middleware: mw_idx });
            }
        }
    }

    /// Build a snapshot for GUI rendering.
    ///
    /// # Allocation cost
    ///
    /// A snapshot fully owns its data (no borrows into the workspace)
    /// so the GUI can serialise / send / store it without lifetime
    /// gymnastics. This is O(total editable nodes) allocation per
    /// call; the GUI should call it once per edit, not once per
    /// render frame.
    pub fn snapshot(&self) -> WorkspaceSnapshot {
        let mut files: Vec<ConfigFileView> = Vec::new();

        // Root file.
        if let Some(root_nodes) = self.root_file_nodes() {
            files.push(root_nodes);
        }

        // Rule-set files.
        for (rs_idx, rule_set) in self.config.service.rule_sets.iter().enumerate() {
            files.push(self.rule_set_file_view(rs_idx, rule_set));
        }

        // Middleware files. We don't introspect them beyond their path
        // existence; the Rhai AST is a server-side concern.
        if let Some(paths) = self.config.service.middlewares_file_paths.as_ref() {
            for (mw_idx, mw_path) in paths.iter().enumerate() {
                let abs = self.resolve_relative(mw_path);
                let id = self
                    .ids
                    .id_for(NodeAddress::Middleware { middleware: mw_idx })
                    .expect("middleware id seeded at load");
                let node = ConfigNodeView {
                    id,
                    source_file: abs.clone(),
                    toml_path: format!("service.middlewares[{}]", mw_idx),
                    display_name: mw_path.clone(),
                    kind: NodeKind::Script,
                    validation: NodeValidation::ok(),
                };
                files.push(ConfigFileView {
                    path: abs.clone(),
                    display_name: file_basename(&abs),
                    kind: ConfigFileKind::Middleware,
                    nodes: vec![node],
                });
            }
        }

        // Route catalog — placeholder for Step 5. Currently an empty
        // snapshot; stage-2 of routing will populate.
        let routes = apimock_routing::view::RouteCatalogSnapshot::empty();

        WorkspaceSnapshot {
            files,
            routes,
            diagnostics: self.diagnostics.clone(),
        }
    }

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
                // Root settings include listener port / ip, which change
                // how the listener binds. Those need a full restart, not
                // just a reload — the caller reads `reload_hint` from
                // save() for the fine-grained hint; at apply time we
                // conservatively flag `requires_reload = true`.
                (ids, true)
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

        let new_rule = build_rule_from_payload(rule_payload, rule_set, rs_idx)?;
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

        let new_rule = build_rule_from_payload(rule_payload, rule_set, rs_idx)?;
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
                // The only recognised strategy value today is
                // "first_match". Anything else is rejected — if future
                // strategies are added, extend this match.
                match s.as_str() {
                    "first_match" => {
                        self.config.service.strategy =
                            Some(apimock_routing::Strategy::FirstMatch);
                    }
                    other => {
                        return Err(ApplyError::InvalidPayload {
                            reason: format!("unknown strategy: {}", other),
                        });
                    }
                }
            }
        }

        // Root is a singleton; its NodeId is always the same.
        let id = self
            .ids
            .id_for(NodeAddress::Root)
            .expect("root id seeded at load");
        Ok(vec![id])
    }

    // --- Shared helpers ----------------------------------------------

    /// After a rule set is removed at `removed_idx`, migrate every ID
    /// whose address referenced a later rule set to its new index.
    fn shift_rule_sets_down(&mut self, removed_idx: usize) {
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
        for (&addr, &id) in self.ids.address_to_id.iter() {
            match addr {
                NodeAddress::RuleSet { rule_set } if rule_set >= removed_idx => {
                    to_migrate.push((id, addr));
                }
                NodeAddress::Rule { rule_set, .. } if rule_set >= removed_idx => {
                    to_migrate.push((id, addr));
                }
                NodeAddress::Respond { rule_set, .. } if rule_set >= removed_idx => {
                    to_migrate.push((id, addr));
                }
                _ => {}
            }
        }

        for (id, addr) in &to_migrate {
            self.ids.address_to_id.remove(addr);
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
            self.ids.id_to_address.insert(id, new_addr);
            self.ids.address_to_id.insert(new_addr, id);
        }
    }

    /// After a rule is deleted from `rule_set_idx` at position
    /// `removed_rule_idx`, shift IDs for later rules in the same set.
    fn shift_rules_down(&mut self, rule_set_idx: usize, removed_rule_idx: usize) {
        let mut to_migrate: Vec<(NodeId, NodeAddress)> = Vec::new();
        for (&addr, &id) in self.ids.address_to_id.iter() {
            match addr {
                NodeAddress::Rule { rule_set, rule }
                    if rule_set == rule_set_idx && rule >= removed_rule_idx =>
                {
                    to_migrate.push((id, addr));
                }
                NodeAddress::Respond { rule_set, rule }
                    if rule_set == rule_set_idx && rule >= removed_rule_idx =>
                {
                    to_migrate.push((id, addr));
                }
                _ => {}
            }
        }

        for (id, addr) in &to_migrate {
            self.ids.address_to_id.remove(addr);
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
            self.ids.id_to_address.insert(id, new_addr);
            self.ids.address_to_id.insert(new_addr, id);
        }
    }

    /// After a rule in `rule_set_idx` moves from `old_idx` to
    /// `new_idx`, shuffle the IDs of every rule between those indices.
    fn reorder_rule_ids(&mut self, rule_set_idx: usize, old_idx: usize, new_idx: usize) {
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
            self.ids.id_to_address.insert(id, addr);
            self.ids.address_to_id.insert(addr, id);
        }
        for (r, id_opt) in resp_ids.into_iter().enumerate() {
            let addr = NodeAddress::Respond {
                rule_set: rule_set_idx,
                rule: r,
            };
            let id = id_opt.unwrap_or_else(NodeId::new);
            self.ids.id_to_address.insert(id, addr);
            self.ids.address_to_id.insert(addr, id);
        }
    }

    fn config_relative_dir(&self) -> Result<String, ConfigError> {
        self.config.current_dir_to_parent_dir_relative_path()
    }

    /// Walk every node, asking it for its validation state, and return
    /// the flat list of issues. Used at apply-time and on demand from
    /// `validate()`.
    fn collect_diagnostics(&self) -> Vec<Diagnostic> {
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

    /// Save the workspace back to disk. **Step 4 will implement this.**
    pub fn save(&mut self) -> Result<SaveResult, SaveError> {
        Err(SaveError::Inconsistent {
            reason: "Workspace::save is a Step-4 feature; not implemented in 5.1.0"
                .to_owned(),
        })
    }

    /// Root config file as a `ConfigFileView`, if it can be rendered.
    fn root_file_nodes(&self) -> Option<ConfigFileView> {
        let mut nodes = Vec::new();

        if let Some(root_id) = self.ids.id_for(NodeAddress::Root) {
            nodes.push(ConfigNodeView {
                id: root_id,
                source_file: self.root_path.clone(),
                toml_path: String::new(),
                display_name: "apimock.toml".to_owned(),
                kind: NodeKind::RootSetting,
                validation: NodeValidation::ok(),
            });
        }

        if let Some(fb_id) = self.ids.id_for(NodeAddress::FallbackRespondDir) {
            nodes.push(ConfigNodeView {
                id: fb_id,
                source_file: self.root_path.clone(),
                toml_path: "service.fallback_respond_dir".to_owned(),
                display_name: self.config.service.fallback_respond_dir.clone(),
                kind: NodeKind::FileNode,
                validation: NodeValidation::ok(),
            });
        }

        Some(ConfigFileView {
            path: self.root_path.clone(),
            display_name: file_basename(&self.root_path),
            kind: ConfigFileKind::Root,
            nodes,
        })
    }

    fn rule_set_file_view(&self, rs_idx: usize, rule_set: &RuleSet) -> ConfigFileView {
        let file_path = PathBuf::from(rule_set.file_path.as_str());
        let mut nodes: Vec<ConfigNodeView> = Vec::new();

        // Rule-set itself.
        if let Some(rs_id) = self
            .ids
            .id_for(NodeAddress::RuleSet { rule_set: rs_idx })
        {
            nodes.push(ConfigNodeView {
                id: rs_id,
                source_file: file_path.clone(),
                toml_path: String::new(),
                display_name: file_basename(&file_path),
                kind: NodeKind::RuleSet,
                validation: NodeValidation::ok(),
            });
        }

        // Rules inside.
        for (rule_idx, rule) in rule_set.rules.iter().enumerate() {
            if let Some(rule_id) = self.ids.id_for(NodeAddress::Rule {
                rule_set: rs_idx,
                rule: rule_idx,
            }) {
                let url_path_label = rule
                    .when
                    .request
                    .url_path
                    .as_ref()
                    .map(|u| u.value.as_str())
                    .unwrap_or_default();
                let display = if url_path_label.is_empty() {
                    format!("Rule #{}", rule_idx + 1)
                } else {
                    url_path_label.to_owned()
                };
                nodes.push(ConfigNodeView {
                    id: rule_id,
                    source_file: file_path.clone(),
                    toml_path: format!("rules[{}]", rule_idx),
                    display_name: display,
                    kind: NodeKind::Rule,
                    validation: NodeValidation::ok(),
                });
            }

            if let Some(resp_id) = self.ids.id_for(NodeAddress::Respond {
                rule_set: rs_idx,
                rule: rule_idx,
            }) {
                nodes.push(ConfigNodeView {
                    id: resp_id,
                    source_file: file_path.clone(),
                    toml_path: format!("rules[{}].respond", rule_idx),
                    display_name: summarise_respond(&rule.respond),
                    kind: NodeKind::Respond,
                    validation: respond_node_validation(&rule.respond, rule_set, rule_idx, rs_idx),
                });
            }
        }

        ConfigFileView {
            path: file_path.clone(),
            display_name: file_basename(&file_path),
            kind: ConfigFileKind::RuleSet,
            nodes,
        }
    }

    fn resolve_relative(&self, rel: &str) -> PathBuf {
        match self.config.current_dir_to_parent_dir_relative_path() {
            Ok(dir) => Path::new(&dir).join(rel),
            Err(_) => PathBuf::from(rel),
        }
    }

    /// Access the underlying `Config`. Intended for embedders that
    /// need to build a running `Server` from the same workspace. Edit
    /// via `apply()` instead of touching `Config` directly — changes
    /// made through this reference are invisible to the ID index.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Access the root path. Primarily for diagnostics.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }
}

/// Collapse a `Respond` into a one-line display label.
fn summarise_respond(respond: &apimock_routing::Respond) -> String {
    if let Some(p) = respond.file_path.as_ref() {
        return format!("file: {}", p);
    }
    if let Some(t) = respond.text.as_ref() {
        const LIMIT: usize = 40;
        if t.chars().count() > LIMIT {
            let truncated: String = t.chars().take(LIMIT).collect();
            return format!("text: {}…", truncated);
        }
        return format!("text: {}", t);
    }
    if let Some(s) = respond.status.as_ref() {
        return format!("status: {}", s);
    }
    "(empty)".to_owned()
}

fn respond_node_validation(
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

fn file_basename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

// --- Payload → model helpers used by the apply layer --------------

fn build_rule_from_payload(
    payload: crate::view::RulePayload,
    rule_set: &apimock_routing::RuleSet,
    rs_idx: usize,
) -> Result<apimock_routing::Rule, ApplyError> {
    use apimock_routing::rule_set::rule::Rule;
    use apimock_routing::rule_set::rule::when::When;
    use apimock_routing::rule_set::rule::when::request::{
        Request, http_method::HttpMethod, url_path::UrlPathConfig,
    };

    // Build the Request shape from the simple payload. We use the
    // simple UrlPath variant (Simple(String)) because the payload's
    // url_path is a plain string; the richer variants (op, etc.) are
    // out of scope for 5.1 — a GUI can round-trip them once Step-5
    // exposes richer form controls.
    let url_path_config = payload.url_path.as_ref().map(|s| UrlPathConfig::Simple(s.clone()));

    let http_method = match payload.method.as_deref() {
        Some("GET") | Some("get") => Some(HttpMethod::Get),
        Some("POST") | Some("post") => Some(HttpMethod::Post),
        Some("PUT") | Some("put") => Some(HttpMethod::Put),
        Some("DELETE") | Some("delete") => Some(HttpMethod::Delete),
        Some(other) => {
            return Err(ApplyError::InvalidPayload {
                reason: format!(
                    "unsupported HTTP method `{}` — supported: GET, POST, PUT, DELETE",
                    other
                ),
            });
        }
        None => None,
    };

    let request = Request {
        url_path_config,
        url_path: None, // derived below
        http_method,
        headers: None,
        body: None,
    };

    let rule = Rule {
        when: When { request },
        respond: build_respond_from_payload(payload.respond),
    };

    // compute_derived_fields normalises the URL path with the rule
    // set's prefix and validates the status code. Running it here means
    // the freshly-created rule is ready for matching without a second
    // pass.
    //
    // `rule_idx` at this point is whatever position the rule will
    // occupy after being pushed — use `rule_set.rules.len()` because
    // the push happens immediately after.
    Ok(rule.compute_derived_fields(rule_set, rule_set.rules.len(), rs_idx))
}

fn build_respond_from_payload(payload: crate::view::RespondPayload) -> apimock_routing::Respond {
    apimock_routing::Respond {
        file_path: payload.file_path,
        csv_records_key: None,
        text: payload.text,
        status: payload.status,
        status_code: None, // derived later
        headers: None,
        delay_response_milliseconds: payload.delay_milliseconds,
    }
}

fn value_as_string(value: &EditValue) -> Result<String, ApplyError> {
    match value {
        EditValue::String(s) => Ok(s.clone()),
        EditValue::Enum(s) => Ok(s.clone()),
        other => Err(ApplyError::InvalidPayload {
            reason: format!("expected a string, got {:?}", other),
        }),
    }
}

fn value_as_integer(value: &EditValue) -> Result<i64, ApplyError> {
    match value {
        EditValue::Integer(n) => Ok(*n),
        other => Err(ApplyError::InvalidPayload {
            reason: format!("expected an integer, got {:?}", other),
        }),
    }
}

/// Wrap a ConfigError produced inside an apply command as an
/// `ApplyError::InvalidPayload`. Apply uses anyhow-ish flattening
/// because the caller doesn't care whether the root cause was a
/// read-fail or a parse-fail — they all surface as "edit couldn't
/// be applied" from the GUI's point of view.
fn internal_path_err(err: ConfigError) -> ApplyError {
    ApplyError::InvalidPayload {
        reason: format!("internal path resolution failed: {}", err),
    }
}

fn resolve_root(root: &Path) -> Result<PathBuf, WorkspaceError> {
    if root.is_file() {
        return Ok(root.to_path_buf());
    }
    if root.is_dir() {
        let candidate = root.join("apimock.toml");
        if candidate.is_file() {
            return Ok(candidate);
        }
        return Err(WorkspaceError::InvalidRoot {
            path: root.to_path_buf(),
            reason: "directory does not contain apimock.toml".to_owned(),
        });
    }
    Err(WorkspaceError::InvalidRoot {
        path: root.to_path_buf(),
        reason: "path does not exist".to_owned(),
    })
}

// Convert a raw `RoutingError` sneaked into the load path; normally
// `ConfigError` wraps it, but the explicit conversion keeps the
// apply-layer clean when it needs to materialise one.
#[allow(dead_code)]
fn routing_to_config(err: RoutingError) -> ConfigError {
    ConfigError::from(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a minimal on-disk workspace and return the tempdir guard
    /// + absolute path to the root apimock.toml.
    fn make_workspace() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");

        // Rule-set file with one file-based rule + one text rule.
        let rule_set_toml = concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/api/users\"\n",
            "respond = { text = \"ok\" }\n",
            "\n",
            "[[rules]]\n",
            "when.request.url_path = \"/api/health\"\n",
            "respond = { status = 204 }\n",
        );
        let rs_path = dir.path().join("apimock-rule-set.toml");
        fs::write(&rs_path, rule_set_toml).unwrap();

        // Fallback dir so validation doesn't fail.
        let fallback = dir.path().join("fallback");
        fs::create_dir_all(&fallback).unwrap();

        let root_toml = format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = 3001\n\
             \n\
             [service]\n\
             rule_sets = [\"{}\"]\n\
             fallback_respond_dir = \"{}\"\n",
            rs_path.file_name().unwrap().to_string_lossy(),
            fallback.file_name().unwrap().to_string_lossy(),
        );
        let root_path = dir.path().join("apimock.toml");
        fs::write(&root_path, root_toml).unwrap();

        (dir, root_path)
    }

    #[test]
    fn load_returns_workspace_with_seeded_ids() {
        let (_dir, root) = make_workspace();
        let ws = Workspace::load(root).expect("load");
        // Root + fallback + 1 rule set + 2 rules + 2 responds = 7 nodes
        // registered in the id index.
        let n = ws.ids.id_to_address.len();
        assert!(n >= 7, "expected at least 7 seeded nodes, got {}", n);
    }

    #[test]
    fn snapshot_shapes_match_spec() {
        let (_dir, root) = make_workspace();
        let ws = Workspace::load(root).expect("load");
        let snap = ws.snapshot();

        // Should have a root file view + a rule-set file view.
        assert!(!snap.files.is_empty());
        let root_view = snap
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::Root))
            .expect("root file present");
        assert!(root_view.nodes.iter().any(|n| n.display_name == "apimock.toml"));

        let rs_view = snap
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .expect("rule set present");
        // rule-set + 2 rules + 2 responds
        assert!(rs_view.nodes.len() >= 5);
        assert!(rs_view.nodes.iter().any(|n| matches!(n.kind, NodeKind::RuleSet)));
        assert!(rs_view.nodes.iter().any(|n| matches!(n.kind, NodeKind::Rule)));
        assert!(rs_view.nodes.iter().any(|n| matches!(n.kind, NodeKind::Respond)));
    }

    #[test]
    fn snapshot_nodes_have_unique_ids() {
        let (_dir, root) = make_workspace();
        let ws = Workspace::load(root).expect("load");
        let snap = ws.snapshot();

        let mut seen = std::collections::HashSet::new();
        for file in &snap.files {
            for node in &file.nodes {
                assert!(
                    seen.insert(node.id),
                    "duplicate NodeId in snapshot: {}",
                    node.id
                );
            }
        }
    }

    #[test]
    fn snapshot_respond_node_displays_rule_content() {
        let (_dir, root) = make_workspace();
        let ws = Workspace::load(root).expect("load");
        let snap = ws.snapshot();

        let rs_view = snap
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap();

        // First respond should say "text: ok"; second "status: 204".
        let responds: Vec<_> = rs_view
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Respond))
            .collect();
        assert_eq!(responds.len(), 2);
        assert!(responds[0].display_name.contains("ok"));
        assert!(responds[1].display_name.contains("204"));
    }

    #[test]
    fn apply_add_rule_to_existing_rule_set() {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root).expect("load");

        // Find the rule-set's NodeId.
        let snap = ws.snapshot();
        let rs_node = snap
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap()
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::RuleSet))
            .unwrap();
        let rs_id = rs_node.id;

        let before = ws.config.service.rule_sets[0].rules.len();
        let result = ws
            .apply(EditCommand::AddRule {
                parent: rs_id,
                rule: crate::view::RulePayload {
                    url_path: Some("/new/rule".to_owned()),
                    method: None,
                    respond: crate::view::RespondPayload {
                        text: Some("hi".to_owned()),
                        ..Default::default()
                    },
                },
            })
            .expect("apply AddRule");
        assert!(result.requires_reload);
        assert_eq!(ws.config.service.rule_sets[0].rules.len(), before + 1);
        // changed_nodes should include the parent + new rule + new respond.
        assert!(result.changed_nodes.len() >= 3);
    }

    #[test]
    fn apply_update_respond_changes_content() {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root).expect("load");

        // Grab the first respond node's id.
        let snap = ws.snapshot();
        let resp_id = snap
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap()
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Respond))
            .unwrap()
            .id;

        ws.apply(EditCommand::UpdateRespond {
            id: resp_id,
            respond: crate::view::RespondPayload {
                text: Some("updated-text".to_owned()),
                ..Default::default()
            },
        })
        .expect("apply UpdateRespond");

        // The respond display should now reflect the new text.
        let snap2 = ws.snapshot();
        let new_display = snap2
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == resp_id)
            .unwrap()
            .display_name
            .clone();
        assert!(new_display.contains("updated-text"), "got: {}", new_display);
    }

    #[test]
    fn apply_delete_rule_shifts_successors_and_preserves_ids() {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root).expect("load");

        // Grab the two rule NodeIds before edit.
        let snap = ws.snapshot();
        let rs_view = snap
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap();
        let rules: Vec<NodeId> = rs_view
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Rule))
            .map(|n| n.id)
            .collect();
        assert_eq!(rules.len(), 2);

        let first_id = rules[0];
        let second_id = rules[1];

        ws.apply(EditCommand::DeleteRule { id: first_id })
            .expect("delete");

        // After delete: only one rule remains, and its id should still
        // be `second_id` (ID stability across shift).
        let snap2 = ws.snapshot();
        let rs_view2 = snap2
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap();
        let rules_after: Vec<NodeId> = rs_view2
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Rule))
            .map(|n| n.id)
            .collect();
        assert_eq!(rules_after.len(), 1);
        assert_eq!(
            rules_after[0], second_id,
            "second rule's NodeId should survive deletion of the first"
        );
    }

    #[test]
    fn apply_move_rule_preserves_ids() {
        // Build a workspace with 3 rules so there's room to move.
        let dir = tempfile::tempdir().unwrap();
        let fallback = dir.path().join("fallback");
        std::fs::create_dir_all(&fallback).unwrap();
        let rs_toml = concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/one\"\n",
            "respond = { text = \"a\" }\n",
            "\n",
            "[[rules]]\n",
            "when.request.url_path = \"/two\"\n",
            "respond = { text = \"b\" }\n",
            "\n",
            "[[rules]]\n",
            "when.request.url_path = \"/three\"\n",
            "respond = { text = \"c\" }\n",
        );
        std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
        std::fs::write(
            dir.path().join("apimock.toml"),
            format!(
                "[listener]\nip_address = \"127.0.0.1\"\nport = 3001\n[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \"{}\"\n",
                fallback.file_name().unwrap().to_string_lossy()
            ),
        )
        .unwrap();

        let mut ws = Workspace::load(dir.path().join("apimock.toml")).unwrap();

        let snap = ws.snapshot();
        let rs_view = snap
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap();
        let rule_ids: Vec<NodeId> = rs_view
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Rule))
            .map(|n| n.id)
            .collect();
        assert_eq!(rule_ids.len(), 3);
        let [id_a, id_b, id_c] = [rule_ids[0], rule_ids[1], rule_ids[2]];

        // Move first to last.
        ws.apply(EditCommand::MoveRule {
            id: id_a,
            new_index: 2,
        })
        .expect("move");

        // Verify order by display_name (url_path): should be b, c, a.
        let snap2 = ws.snapshot();
        let rs_view2 = snap2
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap();
        let new_rules: Vec<&ConfigNodeView> = rs_view2
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Rule))
            .collect();
        assert_eq!(new_rules.len(), 3);
        assert_eq!(new_rules[0].id, id_b);
        assert_eq!(new_rules[1].id, id_c);
        assert_eq!(new_rules[2].id, id_a);
    }

    #[test]
    fn apply_update_root_setting_port() {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root).expect("load");
        ws.apply(EditCommand::UpdateRootSetting {
            key: crate::view::RootSettingKey::ListenerPort,
            value: EditValue::Integer(9999),
        })
        .expect("update port");
        assert_eq!(ws.config.listener.as_ref().unwrap().port, 9999);
    }

    #[test]
    fn apply_update_root_setting_bad_port_is_invalid_payload() {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root).expect("load");
        let result = ws.apply(EditCommand::UpdateRootSetting {
            key: crate::view::RootSettingKey::ListenerPort,
            value: EditValue::Integer(70_000),
        });
        assert!(matches!(result, Err(ApplyError::InvalidPayload { .. })));
    }

    #[test]
    fn apply_unknown_node_id() {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root).expect("load");
        let result = ws.apply(EditCommand::DeleteRule { id: NodeId::new() });
        assert!(matches!(result, Err(ApplyError::UnknownNode { .. })));
    }

    #[test]
    fn apply_wrong_kind_error() {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root).expect("load");
        // Grab a respond id and try to delete it as a rule.
        let snap = ws.snapshot();
        let resp_id = snap
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap()
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Respond))
            .unwrap()
            .id;
        let result = ws.apply(EditCommand::DeleteRule { id: resp_id });
        assert!(matches!(result, Err(ApplyError::WrongNodeKind { .. })));
    }

    #[test]
    fn validate_surfaces_per_node_diagnostics() {
        // Build a workspace with an invalid rule (file_path + text).
        let dir = tempfile::tempdir().unwrap();
        let fallback = dir.path().join("fallback");
        std::fs::create_dir_all(&fallback).unwrap();
        let rs_toml = concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/bad\"\n",
            "respond = { file_path = \"nonexistent.json\", text = \"x\" }\n",
        );
        std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
        std::fs::write(
            dir.path().join("apimock.toml"),
            format!(
                "[listener]\nip_address = \"127.0.0.1\"\nport = 3001\n[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \"{}\"\n",
                fallback.file_name().unwrap().to_string_lossy()
            ),
        )
        .unwrap();

        // Construct Workspace manually, bypassing Config::new's validation
        // (which would reject the invalid respond at load time).
        // We do this by first writing a valid rule-set, loading, then
        // mutating through apply.
        std::fs::write(
            dir.path().join("apimock-rule-set.toml"),
            concat!(
                "[[rules]]\n",
                "when.request.url_path = \"/bad\"\n",
                "respond = { text = \"ok\" }\n",
            ),
        )
        .unwrap();
        let mut ws = Workspace::load(dir.path().join("apimock.toml")).unwrap();
        let resp_id = ws
            .snapshot()
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap()
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Respond))
            .unwrap()
            .id;

        // Now update respond to an invalid combination (file_path + text).
        let result = ws
            .apply(EditCommand::UpdateRespond {
                id: resp_id,
                respond: crate::view::RespondPayload {
                    file_path: Some("nope.json".to_owned()),
                    text: Some("also-text".to_owned()),
                    ..Default::default()
                },
            })
            .expect("apply should succeed even with bad payload");

        // diagnostics should include at least two errors: exclusivity
        // violation + file-not-found.
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .collect();
        assert!(
            errors.len() >= 2,
            "expected at least 2 errors; got {}: {:?}",
            errors.len(),
            result.diagnostics
        );

        // validate() should agree.
        let report = ws.validate();
        assert!(!report.is_valid);
    }
}
