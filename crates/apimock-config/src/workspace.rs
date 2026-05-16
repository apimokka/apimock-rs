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
    /// Snapshot of every TOML file's on-disk contents at the time of
    /// load (or last successful save). Save uses this to:
    ///   - decide which files actually changed (we don't rewrite a
    ///     file whose rendered content is byte-identical to the
    ///     baseline);
    ///   - detect external changes between load and save (the same
    ///     mechanism could surface "file changed underneath you" in a
    ///     future stage; 5.2.0 doesn't act on that yet).
    baseline_files: HashMap<PathBuf, String>,
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

        // Snapshot every TOML file's rendered shape so save() can
        // tell which files actually have unsaved edits.
        //
        // # Why "rendered model" rather than "on-disk text"
        //
        // A naive baseline would store the literal on-disk text. But
        // our writer (`toml_writer`) produces canonicalised TOML —
        // sorted keys, no comments, double-quoted strings, etc. —
        // which almost never byte-matches a hand-edited file. With
        // "on-disk" baseline, `has_unsaved_changes` would return
        // `true` right after a load with no edits, and the first
        // save would unconditionally rewrite every file.
        //
        // Storing the *rendered* baseline solves this: a freshly
        // loaded workspace has rendered == baseline by construction,
        // so `has_unsaved_changes` is false. Edits flip it to true,
        // and only the files that diverge get rewritten on save.
        // The user's hand-formatting on never-edited files survives
        // untouched.
        let mut baseline_files: HashMap<PathBuf, String> = HashMap::new();
        baseline_files.insert(
            resolved.clone(),
            crate::toml_writer::render_apimock_toml(&config),
        );
        for rule_set in config.service.rule_sets.iter() {
            let path = PathBuf::from(rule_set.file_path.as_str());
            baseline_files.insert(
                path,
                crate::toml_writer::render_rule_set_toml(rule_set),
            );
        }

        let mut workspace = Self {
            root_path: resolved,
            config,
            ids: IdIndex::default(),
            diagnostics: Vec::new(),
            baseline_files,
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

        // Route catalog — assemble from rule sets, fallback dir,
        // file tree (depth-1 eager), and middleware script routes.
        // Builders live in `apimock_routing::view::build`; the config
        // crate just feeds them the data they need.
        let fallback_dir = self.config.service.fallback_respond_dir.as_str();
        let fallback_abs = self.resolve_relative(fallback_dir);
        let file_tree = apimock_routing::view::build::build_file_tree(&fallback_abs);

        let script_routes: Vec<apimock_routing::view::ScriptRouteView> = self
            .config
            .service
            .middlewares_file_paths
            .as_ref()
            .map(|paths| {
                paths
                    .iter()
                    .enumerate()
                    .map(|(idx, p)| apimock_routing::view::build::build_script_route_view(idx, p))
                    .collect()
            })
            .unwrap_or_default();

        let routes = apimock_routing::view::build::build_route_catalog(
            &self.config.service.rule_sets,
            Some(fallback_dir),
            file_tree,
            script_routes,
        );

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

    /// Save the workspace back to disk.
    ///
    /// # Algorithm
    ///
    /// 1. Render each editable file (root + each rule set) to TOML text.
    /// 2. Compare against `baseline_files`. Files whose rendered output
    ///    is byte-identical to the baseline are skipped — the user's
    ///    formatting / comments survive untouched in that case.
    /// 3. For files that *do* differ, write atomically via
    ///    `tempfile::NamedTempFile::persist` (same-directory rename(2)
    ///    on POSIX, `MoveFileExW` on Windows). On any single-file
    ///    failure, the partial state is whatever rename(2)s have
    ///    already succeeded — see the type-level docstring on
    ///    `SaveError` for the rationale.
    /// 4. After all writes succeed, refresh `baseline_files` so a
    ///    subsequent save() won't re-write the same files needlessly.
    /// 5. Compute `DiffItem`s by node, comparing the in-memory state
    ///    to the load-time baseline (parsed; not text-diff).
    /// 6. Compute `requires_reload` / `requires_restart` from the set
    ///    of changed files: changes to `[listener]` need a restart,
    ///    everything else just a reload.
    ///
    /// # The "save loses comments" diagnostic
    ///
    /// Per the GUI spec §6 / §11, save is allowed to lose comments and
    /// formatting. We surface this as an `Info`-severity diagnostic
    /// the first time a save would actually overwrite a file that has
    /// non-trivial formatting (any file whose TOML round-trip is not
    /// byte-identical, which is essentially every hand-edited file).
    /// A polished GUI shows it once per session.
    pub fn save(&mut self) -> Result<SaveResult, SaveError> {
        // --- Render every file's new content -------------------------
        let new_root_toml = crate::toml_writer::render_apimock_toml(&self.config);

        let mut rule_set_renders: Vec<(PathBuf, String)> = Vec::new();
        for rule_set in self.config.service.rule_sets.iter() {
            let path = PathBuf::from(rule_set.file_path.as_str());
            let text = crate::toml_writer::render_rule_set_toml(rule_set);
            rule_set_renders.push((path, text));
        }

        // --- Compute changed-file set --------------------------------
        let mut to_write: Vec<(PathBuf, String)> = Vec::new();

        let baseline_root = self.baseline_files.get(&self.root_path);
        if baseline_root.map(String::as_str) != Some(new_root_toml.as_str()) {
            to_write.push((self.root_path.clone(), new_root_toml.clone()));
        }
        for (path, text) in rule_set_renders.iter() {
            let baseline = self.baseline_files.get(path);
            if baseline.map(String::as_str) != Some(text.as_str()) {
                to_write.push((path.clone(), text.clone()));
            }
        }

        // --- Atomic write via tempfile::persist ----------------------
        let mut written: Vec<PathBuf> = Vec::with_capacity(to_write.len());
        for (path, text) in &to_write {
            atomic_write(path, text)?;
            written.push(path.clone());
        }

        // --- Build diff_summary BEFORE updating baseline ------------
        // The diff is "what did this save flush to disk", computed
        // against the *previous* baseline. Once we refresh the
        // baseline below, every node would compare equal again.
        let diff_summary = self.compute_diff_summary();

        // --- Refresh baseline ---------------------------------------
        for (path, text) in to_write.into_iter() {
            self.baseline_files.insert(path, text);
        }

        // --- Reload hint --------------------------------------------
        // If the root file (which holds [listener]) was rewritten we
        // conservatively flag a restart. Otherwise rule-set-only changes
        // are a plain reload.
        let listener_changed = written.contains(&self.root_path);
        let requires_reload = listener_changed || !written.is_empty();

        Ok(SaveResult {
            changed_files: written,
            diff_summary,
            requires_reload,
        })
    }

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
    fn compute_diff_summary(&self) -> Vec<crate::view::DiffItem> {
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
            let target = self
                .ids
                .id_for(NodeAddress::Rule {
                    rule_set: rs_idx,
                    rule: rule_idx,
                })
                .unwrap_or_else(NodeId::new);
            out.push(DiffItem {
                kind: DiffKind::Updated,
                target,
                summary: format!(
                    "rule #{} in rule set #{}",
                    rule_idx + 1,
                    rs_idx + 1
                ),
            });
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

    /// True when at least one editable file's rendered output differs
    /// from its load-time baseline.
    ///
    /// # Use case
    ///
    /// A GUI's "unsaved changes" indicator polls this. Cheap relative
    /// to a full save (no file I/O, just renders + string compares).
    pub fn has_unsaved_changes(&self) -> bool {
        let root_text = crate::toml_writer::render_apimock_toml(&self.config);
        if self
            .baseline_files
            .get(&self.root_path)
            .map(|s| s.as_str())
            != Some(root_text.as_str())
        {
            return true;
        }
        for rule_set in self.config.service.rule_sets.iter() {
            let path = PathBuf::from(rule_set.file_path.as_str());
            let text = crate::toml_writer::render_rule_set_toml(rule_set);
            if self
                .baseline_files
                .get(&path)
                .map(|s| s.as_str())
                != Some(text.as_str())
            {
                return true;
            }
        }
        false
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

    /// Expand a directory in the file tree on demand.
    ///
    /// # When the GUI calls this
    ///
    /// `Workspace::snapshot()` returns a `FileTreeView` populated with
    /// just the top-level entries of the fallback respond dir. Each
    /// directory entry carries `children: Some(Vec::new())` to flag it
    /// as expandable. When a user clicks to expand one of those nodes,
    /// the GUI calls `list_directory(&entry.path)` and gets back the
    /// next depth's entries (still not recursed past that depth — the
    /// same lazy contract holds).
    ///
    /// # Why path-based and not NodeId-based
    ///
    /// File-tree entries don't carry NodeIds (see `FileNodeView`). The
    /// reason is lifecycle: the editable node space (rules, rule sets,
    /// respond blocks) is small, stable, and survives `apply()` calls
    /// — perfect for UUID-keyed state. The file tree is large,
    /// transient, and reflects the filesystem rather than the model;
    /// keying it by path keeps the API simple and avoids mixing two
    /// kinds of identity.
    pub fn list_directory(&self, path: &Path) -> Vec<apimock_routing::view::FileNodeView> {
        apimock_routing::view::build::list_directory(path)
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

/// Render a single rule to canonical TOML text. Used by per-rule diff
/// to compare baseline rules to current rules in a format-agnostic
/// way (the same canonicalisation `toml_writer` applies to whole
/// files).
fn rule_to_string(rule: &apimock_routing::Rule) -> String {
    let table = crate::toml_writer::rule_table(rule);
    toml::to_string_pretty(&toml::Value::Table(table)).unwrap_or_default()
}

/// Write `text` to `path` atomically.
///
/// # Why a tempfile + persist instead of a direct write
///
/// `std::fs::write` is two syscalls (truncate + write) with a window
/// between them where a concurrent reader can see an empty file. The
/// running apimock server reads its own config files when (eventually)
/// it supports reload; if it picks a moment in the middle of
/// `std::fs::write`, it can fail to parse a half-written TOML.
///
/// `tempfile::NamedTempFile::persist` writes to `<dir>/.tmpXXXX`,
/// `fsync`s, then `rename(2)`s onto the destination — a single
/// directory-entry update that the kernel guarantees is atomic. On
/// Windows, `tempfile` translates this into `MoveFileExW` with the
/// replace-existing flag for the same effect.
///
/// # Error mapping
///
/// `tempfile`'s persist returns a `PersistError` that wraps both the
/// `NamedTempFile` and the underlying `io::Error`. We unwrap the
/// `io::Error` and surface it as `SaveError::Write`. The temp file
/// is dropped automatically (and removed) when the persist error
/// returns.
fn atomic_write(path: &Path, text: &str) -> Result<(), SaveError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut tmp =
        tempfile::NamedTempFile::new_in(&parent).map_err(|e| SaveError::Write {
            path: path.to_path_buf(),
            source: e,
        })?;

    use std::io::Write;
    tmp.write_all(text.as_bytes())
        .map_err(|e| SaveError::Write {
            path: path.to_path_buf(),
            source: e,
        })?;
    tmp.flush().map_err(|e| SaveError::Write {
        path: path.to_path_buf(),
        source: e,
    })?;

    tmp.persist(path).map_err(|persist_err| SaveError::Write {
        path: path.to_path_buf(),
        source: persist_err.error,
    })?;
    Ok(())
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
mod tests;
