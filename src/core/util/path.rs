use std::{
    env, io,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

/// Relative path from the current working directory to the parent
/// directory of the given file.
///
/// # Why a file whose parent we can't determine is an `io::Error`
///
/// `Path::parent()` returns `None` for root-only paths (`"/"`) and empty
/// paths. Neither is a valid config file location, so treating them as an
/// I/O error keeps the caller's `?` chain clean instead of forcing them
/// to handle an `Option` separately.
pub fn current_dir_to_file_parent_dir_relative_path(file_path: &str) -> io::Result<PathBuf> {
    let parent = Path::new(file_path).parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("failed to get parent dir: {}", file_path),
        )
    })?;
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
