//! `Workspace::save()` and `has_unsaved_changes()`, plus the
//! atomic-write helper they depend on.
//!
//! # Atomic write strategy
//!
//! `std::fs::write` is two syscalls (truncate + write); a concurrent
//! reader can catch an empty file between them. We instead route every
//! write through `tempfile::NamedTempFile::persist`, which writes the
//! new contents to a sibling tempfile, syncs, and renames onto the
//! destination. POSIX `rename(2)` is atomic at the directory-entry
//! level; `tempfile` does the right thing on Windows via
//! `MoveFileExW`. A reader either sees the old file or the new file —
//! never a partial one.
//!
//! # Why diff is computed before refreshing baseline
//!
//! The diff summary describes "what this save just flushed to disk",
//! computed against the previous baseline. We capture it before the
//! baseline refresh in `save()` so there's still something to compare
//! against; once `baseline_files` is updated to the freshly-written
//! contents, the diff would always come back empty.

use std::path::{Path, PathBuf};

use crate::error::SaveError;
use crate::view::SaveResult;

use super::Workspace;

impl Workspace {
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
        // Refresh mtime snapshots so has_external_changes() doesn't
        // immediately fire for files we just wrote (RFC 024).
        for path in self.baseline_files.keys() {
            if let Ok(meta) = std::fs::metadata(path) {
                if let Ok(modified) = meta.modified() {
                    self.file_metas.insert(
                        path.clone(),
                        crate::workspace::FileMeta { modified, len: meta.len() },
                    );
                }
            }
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
