//! Confine a resolved file path to the directory it was resolved
//! against.
//!
//! # Why this lives in one place instead of at each call site
//!
//! Every `FileResponse` construction site ends up here through
//! [`FileResponse::file_content_response`](super::file_response::FileResponse::file_content_response),
//! including the extension/`index.*` resolution that happens inside it.
//! A single enforcement point catches an escape introduced at any of
//! those stages, not only a raw `..` in the original candidate.

use std::path::{Path, PathBuf};

/// Canonicalise a directory once. Callers do this when the directory
/// is first known (server startup, middleware compilation) and reuse
/// the result across requests — [`confine`] only canonicalises the
/// per-request candidate.
///
/// `None` if `dir` doesn't exist or isn't a directory; [`confine`]
/// treats that as "nothing can be served from it" rather than
/// skipping the check.
pub fn canonical_dir(dir: &str) -> Option<PathBuf> {
    std::fs::canonicalize(dir).ok().filter(|p| p.is_dir())
}

/// Resolve `candidate` and confirm it stays inside `base`.
///
/// Returns the canonical form to actually read from. Returns `None`
/// if `candidate` doesn't exist, if `base` is `None` (the directory
/// couldn't be canonicalised at load time), or if the canonical
/// candidate falls outside `base` — a symlink included, since
/// canonicalisation follows them. Every one of those cases becomes an
/// ordinary 404 to the caller; none is distinguishable from another.
pub fn confine(candidate: &str, base: Option<&Path>) -> Option<PathBuf> {
    let base = base?;
    let canonical_candidate = std::fs::canonicalize(candidate).ok()?;
    if canonical_candidate.starts_with(base) {
        Some(canonical_candidate)
    } else {
        log::debug!("refused: {} resolves outside {}", candidate, base.display());
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_dir_none_for_missing_path() {
        assert!(canonical_dir("/does/not/exist/anywhere").is_none());
    }

    #[test]
    fn canonical_dir_none_for_a_file_not_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.txt");
        std::fs::write(&file, "x").unwrap();
        assert!(canonical_dir(file.to_str().unwrap()).is_none());
    }

    #[test]
    fn confine_accepts_a_candidate_inside_base() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.txt");
        std::fs::write(&file, "x").unwrap();
        let base = canonical_dir(dir.path().to_str().unwrap()).unwrap();
        assert!(confine(file.to_str().unwrap(), Some(&base)).is_some());
    }

    #[test]
    fn confine_refuses_a_candidate_outside_base() {
        let outer = tempfile::tempdir().unwrap();
        let inner = outer.path().join("inner");
        std::fs::create_dir(&inner).unwrap();
        let outside = outer.path().join("outside.txt");
        std::fs::write(&outside, "x").unwrap();
        let base = canonical_dir(inner.to_str().unwrap()).unwrap();
        assert!(confine(outside.to_str().unwrap(), Some(&base)).is_none());
    }

    #[test]
    fn confine_refuses_when_base_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.txt");
        std::fs::write(&file, "x").unwrap();
        assert!(confine(file.to_str().unwrap(), None).is_none());
    }

    #[test]
    fn confine_refuses_a_missing_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let base = canonical_dir(dir.path().to_str().unwrap()).unwrap();
        let missing = dir.path().join("nope.txt");
        assert!(confine(missing.to_str().unwrap(), Some(&base)).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn confine_refuses_a_symlink_escaping_base() {
        let outer = tempfile::tempdir().unwrap();
        let inner = outer.path().join("inner");
        std::fs::create_dir(&inner).unwrap();
        let outside = outer.path().join("outside.txt");
        std::fs::write(&outside, "secret").unwrap();
        let link = inner.join("link.txt");
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        let base = canonical_dir(inner.to_str().unwrap()).unwrap();
        assert!(confine(link.to_str().unwrap(), Some(&base)).is_none());
    }
}
