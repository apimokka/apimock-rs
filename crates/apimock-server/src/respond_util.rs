use std::path::Path;

use console::style;

/// Join `dir_prefix` and `path`, returning the joined path if it exists.
///
/// # This resolves a path; it does not confine one
///
/// No canonicalisation and no containment check happen here. A `path`
/// containing `..`, or an absolute `path`, yields a result **outside**
/// `dir_prefix` whenever that file exists.
///
/// Confinement is deliberately enforced in one place instead —
/// [`crate::response::confine::confine`], applied by `FileResponse`'s
/// read path to every file it serves (see that module's docs for why
/// it is centralised). Inside apimock, this function's result is
/// always read through that path, and its `path` argument is a rule's
/// static, operator-authored `respond.file_path`, never request input.
///
/// **If you call this directly from a library consumer and then read
/// the result, you are bypassing that confinement** and must apply your
/// own containment check before opening the file.
pub fn full_file_path(path: &str, dir_prefix: &str) -> Option<String> {
    let p = if !dir_prefix.is_empty() {
        Path::new(dir_prefix).join(path)
    } else {
        Path::new(path).to_path_buf()
    };

    if !p.exists() {
        return None;
    }

    match p.to_str() {
        Some(x) => Some(x.to_owned()),
        None => {
            log::error!(
                "{} to get str from canonicalized url path:\n{} (prefix = {})",
                style("failed").red(),
                path,
                dir_prefix
            );
            None
        }
    }
}
