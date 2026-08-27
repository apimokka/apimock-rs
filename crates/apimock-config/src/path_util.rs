use std::{
    env, io,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

/// Relative path from the current working directory to the parent
/// directory of the given file.
///
/// # Why a bare filename resolves to `.`, not an error
///
/// `Path::parent()` returns `None` only for root-only paths (`"/"`) —
/// **not** for a bare filename like `"apimock.toml"`, where it returns
/// `Some("")` instead (RFC 064). An empty parent is a real answer ("no
/// directory component was written"), not a missing one, and the
/// correct resolution for it is the current directory — the same
/// place a bare filename already reads from. Passing `""` straight to
/// `fs::canonicalize` (what this function used to do) fails with
/// `ENOENT`, which then surfaces as "the config file doesn't exist"
/// even when it does — this was fixed independently three times at
/// three different call sites before landing here, where it covers
/// all of them at once.
pub fn current_dir_to_file_parent_dir_relative_path(file_path: &str) -> io::Result<PathBuf> {
    let parent = Path::new(file_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    relative_path(env::current_dir()?.as_path(), parent)
}

/// relative path between two paths
pub fn relative_path(from: &Path, to: &Path) -> io::Result<PathBuf> {
    let from_abs = std::fs::canonicalize(from)?;
    let to_abs = std::fs::canonicalize(to)?;

    let mut from_iter = from_abs.components();
    let mut to_iter = to_abs.components();

    let mut from_rest = vec![];
    let mut to_rest = vec![];
    // collect common prefix
    let mut common_prefix = vec![];
    loop {
        match (from_iter.next(), to_iter.next()) {
            (Some(f), Some(t)) if f == t => {
                common_prefix.push(f);
            }
            (Some(f), Some(t)) => {
                from_rest.push(f);
                to_rest.push(t);
                from_rest.extend(from_iter);
                to_rest.extend(to_iter);
                break;
            }
            (Some(f), None) => {
                from_rest.push(f);
                from_rest.extend(from_iter);
                break;
            }
            (None, Some(t)) => {
                to_rest.push(t);
                to_rest.extend(to_iter);
                break;
            }
            (None, None) => break,
        }
    }

    let mut result = PathBuf::new();

    for _ in from_rest {
        result.push("..");
    }

    for t in to_rest {
        result.push(t.as_os_str());
    }

    if result.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(result)
    }
}
