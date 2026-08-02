//! Filesystem helpers reused across the workspace's submodules.
//!
//! Pure functions — no Workspace dependency. Kept in their own
//! module so anything that handles paths inside `workspace/` can
//! `use super::path_helpers::*` without dragging in the larger
//! Workspace API.

use std::path::{Path, PathBuf};

use crate::error::WorkspaceError;

/// Last component of `path` as a display string. Falls back to the
/// full path's lossy form when there's no file name (e.g. `/`).
pub(super) fn file_basename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Resolve the `Workspace::load` root argument to an absolute path
/// pointing at the actual `apimock.toml`. Accepts either a direct
/// file path or a directory containing one.
// clippy: WorkspaceError is a public error type (RFC 030 §6 escalation
// trigger); boxing its large variant would change that type's shape. See
// ESCALATION-002 in the RFC 030 review-request package.
#[allow(clippy::result_large_err)]
pub(super) fn resolve_root(root: &Path) -> Result<PathBuf, WorkspaceError> {
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
