use console::style;

use std::{fs, path::Path};

/// Render the fallback respond dir for display, prefixing the
/// canonicalized absolute path when the configured value is relative.
///
/// # Why we show both relative and absolute forms
///
/// The operator configures a relative path (because the config travels
/// between machines), but at server-startup time it's useful to also see
/// the absolute resolution so they can double-check the right directory
/// is being served. We fall back to just the relative form if
/// canonicalization fails — this function is only used to print an
/// informational banner, so it must never fail the startup itself.
pub fn canonicalized_fallback_respond_dir_to_print(fallback_respond_dir: &str) -> String {
    let p = Path::new(fallback_respond_dir);
    if !p.is_relative() {
        return style(fallback_respond_dir).green().to_string();
    }

    let Ok(absolute_path) = fs::canonicalize(fallback_respond_dir) else {
        // Missing directory is caught by validate() elsewhere; here we
        // just want *some* printable text.
        return style(fallback_respond_dir).green().to_string();
    };

    match absolute_path.to_str() {
        Some(abs) => format!("{} ({})", style(fallback_respond_dir).green(), abs),
        None => style(fallback_respond_dir).green().to_string(),
    }
}
