//! The editable workspace: loaded TOML + stable node IDs + edit API.
//!
//! # Role in the design
//!
//! A GUI never touches `Config` or `RuleSet` directly. It holds a
//! `Workspace` value, calls `snapshot()` to get a read-only view for
//! rendering, and `apply(EditCommand)` to mutate. Later, `save()`
//! writes changes back to disk.
//!
//! # Module layout
//!
//! `Workspace` is large enough that its impl is split across several
//! sibling modules under `workspace/`:
//!
//! - `id_index` — `NodeAddress` + `IdIndex` machinery
//! - `snapshot` — `Workspace::snapshot()` and per-file view builders
//! - `edit` — `Workspace::apply()` and the eight `EditCommand`
//!   handlers (with the `id_shift` and `payload` submodules)
//! - `validate` — `Workspace::validate()` and the per-node walker
//! - `save` — `Workspace::save()`, `has_unsaved_changes()`, and
//!   the atomic-write helper
//! - `diff` — `compute_diff_summary()` and per-rule comparison
//! - `path_helpers` — small filesystem utilities reused by the
//!   submodules above
//!
//! Each submodule is private — the public surface remains
//! `apimock_config::Workspace` and the methods it exposes.
//!
//! This file holds the `Workspace` struct itself, the `load` /
//! `seed_ids` lifecycle, plus the small accessor methods that don't
//! belong in any of the larger groupings.
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

use crate::{
    Config,
    error::{ConfigError, WorkspaceError},
    view::Diagnostic,
};

mod diff;
mod edit;
mod id_index;
mod path_helpers;
mod save;
mod snapshot;
mod validate;

#[cfg(test)]
mod tests;

use id_index::{IdIndex, NodeAddress};
use path_helpers::resolve_root;

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
    pub(super) root_path: PathBuf,
    /// Loaded TOML model. Authoritative source of truth for
    /// persistence; edits happen through the editable helpers on
    /// `Workspace` which keep `config` + id tables in sync.
    pub(super) config: Config,
    /// ID index — see struct doc.
    pub(super) ids: IdIndex,
    /// Workspace-scope diagnostics (e.g. load-time warnings). Per-node
    /// diagnostics live inside each node's `NodeValidation`.
    pub(super) diagnostics: Vec<Diagnostic>,
    /// Snapshot of every editable file's rendered baseline at load
    /// time (or after the most recent successful save). `save()` uses
    /// this to skip files whose rendered output is byte-identical to
    /// the baseline; `has_unsaved_changes()` uses it as a cheap "is
    /// the model dirty" check.
    pub(super) baseline_files: HashMap<PathBuf, String>,
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

    /// Resolve a relative path against the config file's parent dir.
    /// Used by snapshot rendering and by `cmd_add_rule_set`.
    pub(super) fn config_relative_dir(&self) -> Result<String, ConfigError> {
        self.config.current_dir_to_parent_dir_relative_path()
    }

    /// Joins a relative TOML path string against the config's parent
    /// directory. Used by snapshot rendering when materialising
    /// middleware / fallback dir paths for display.
    pub(super) fn resolve_relative(&self, rel: &str) -> PathBuf {
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
        let filter = self
            .config
            .file_tree_view
            .as_ref()
            .map(|c| c.to_filter())
            .unwrap_or_default();
        apimock_routing::view::build::list_directory_with(path, &filter)
    }
}
