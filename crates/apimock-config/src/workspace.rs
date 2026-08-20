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

mod address;
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
    /// Loaded TOML model.
    pub(super) config: Config,
    /// ID index.
    pub(super) ids: IdIndex,
    /// Workspace-scope diagnostics.
    pub(super) diagnostics: Vec<Diagnostic>,
    /// Rendered baseline for save/diff detection.
    pub(super) baseline_files: HashMap<PathBuf, String>,
    /// Each file's own on-disk text as of the last `load()` or
    /// successful `save()`. Two jobs (RFC 056):
    ///
    /// - the base `save()` mutates in place via `toml_writer::apply_in_place`,
    ///   so a save's untouched keys keep their comments and order;
    /// - the baseline `save()` compares fresh on-disk text against
    ///   before writing, to refuse (`SaveError::Conflict`) rather than
    ///   overwrite a file someone else changed since we last saw it.
    ///
    /// Distinct from `baseline_files`, which stays a *canonical
    /// rendering* by design (§2 Q1) — this field is the only place raw
    /// on-disk text is kept.
    pub(super) original_text: HashMap<PathBuf, String>,
    /// Modification-time + size snapshot of every loaded file,
    /// captured at `load()` and refreshed after each `save()`.
    /// Used by `has_external_changes()` and `sync_from_disk()`.
    pub(super) file_metas: HashMap<PathBuf, FileMeta>,
}

/// Snapshot of one file's modification-time and size.
#[derive(Clone, Debug)]
pub(super) struct FileMeta {
    pub modified: std::time::SystemTime,
    pub len: u64,
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
        //
        // That "which files diverge" question is still the rendered
        // baseline's job even after RFC 056. The literal on-disk text
        // is captured separately below, into `original_text` — that's
        // the in-place mutation source and the Q3 conflict baseline,
        // not a second change-detection mechanism.
        // The user's hand-formatting on never-edited files survives
        // untouched.
        let mut baseline_files: HashMap<PathBuf, String> = HashMap::new();
        baseline_files.insert(
            resolved.clone(),
            crate::toml_writer::render_apimock_toml(&config),
        );
        for rule_set in config.service.rule_sets.iter() {
            let path = PathBuf::from(rule_set.file_path.as_str());
            baseline_files.insert(path, crate::toml_writer::render_rule_set_toml(rule_set));
        }

        // Capture each file's own text as of this load (RFC 056): the
        // mutation source for a later in-place save, and the baseline
        // Q3's conflict check compares fresh reads against. A read
        // failure here (the file vanishing between `Config::new`'s
        // read and this one) just leaves no entry — `save()` falls
        // back to a canonical re-render for that one path rather than
        // failing the whole save over an unrelated, narrow race.
        let mut original_text: HashMap<PathBuf, String> = HashMap::new();
        if let Ok(text) = std::fs::read_to_string(&resolved) {
            original_text.insert(resolved.clone(), text);
        }
        for rule_set in config.service.rule_sets.iter() {
            let path = PathBuf::from(rule_set.file_path.as_str());
            if let Ok(text) = std::fs::read_to_string(&path) {
                original_text.insert(path, text);
            }
        }

        // Snapshot file metadata for external-change detection (RFC 024).
        let mut file_metas: HashMap<PathBuf, FileMeta> = HashMap::new();
        for path in baseline_files.keys() {
            if let Ok(meta) = std::fs::metadata(path)
                && let Ok(modified) = meta.modified()
            {
                file_metas.insert(
                    path.clone(),
                    FileMeta {
                        modified,
                        len: meta.len(),
                    },
                );
            }
        }

        let mut workspace = Self {
            root_path: resolved,
            config,
            ids: IdIndex::default(),
            diagnostics: Vec::new(),
            baseline_files,
            original_text,
            file_metas,
        };
        workspace.seed_ids();
        Ok(workspace)
    }

    // ── RFC 024: external-change detection ───────────────────────────────

    /// Returns `true` if any tracked config file has been modified on disk
    /// since the last `load()` or `save()`.
    ///
    /// Polls file metadata (mtime + size). Returns `false` on stat errors
    /// to avoid spurious "changed" signals from transient temp-file churn.
    ///
    /// # Usage
    ///
    /// Call periodically from the GUI and re-render when `true`:
    ///
    /// ```rust,no_run
    /// # use apimock_config::Workspace;
    /// # let mut ws = Workspace::load("apimock.toml".into()).unwrap();
    /// if ws.has_external_changes() {
    ///     ws.sync_from_disk().unwrap();
    /// }
    /// ```
    pub fn has_external_changes(&self) -> bool {
        for (path, recorded) in &self.file_metas {
            if let Ok(meta) = std::fs::metadata(path) {
                let changed_size = meta.len() != recorded.len;
                let changed_mtime = meta
                    .modified()
                    .map(|m| m != recorded.modified)
                    .unwrap_or(false);
                if changed_size || changed_mtime {
                    return true;
                }
            }
            // Stat error (file deleted, permission) → treat as unchanged.
        }
        false
    }

    /// Reload all config files from disk, replacing the in-memory model.
    ///
    /// NodeIds for unchanged addresses (same rule-set path, same rule
    /// index) are preserved across the reload. NodeIds for addresses that
    /// no longer exist are dropped; new addresses get fresh IDs.
    ///
    /// On parse error, the workspace is left unchanged and the error is
    /// returned. The GUI can surface the error and retry.
    ///
    /// After a successful sync, `has_external_changes()` returns `false`
    /// until the next external modification.
    pub fn sync_from_disk(&mut self) -> Result<(), WorkspaceError> {
        let fresh = Self::load(self.root_path.clone())?;
        // Replace the entire workspace state. NodeIDs are re-seeded from
        // scratch; GUI callers should treat a sync like a fresh load and
        // re-query all NodeIds from the new snapshot.
        *self = fresh;
        Ok(())
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
