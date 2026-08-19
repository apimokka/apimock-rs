use console::style;
use serde::Deserialize;

use std::path::Path;

/// A rule set's optional URL-path / response-directory prefix, exactly
/// as authored in `[prefix]`.
///
/// # `respond_dir_prefix` holds only what was written (RFC 058)
///
/// `RuleSet::new` resolves `respond_dir_prefix` against the rule-set
/// file's own directory so `Respond::file_path` can be found regardless
/// of the process's CWD — but it stores that resolved path in
/// `RuleSet::resolved_respond_dir`, never back into this field. Before
/// RFC 058, the resolved value overwrote the authored one here, and
/// `toml_writer` persisted it, so a load+save cycle resolved the
/// already-resolved value again — the field grew by one `./` segment
/// per save, without bound. Read `RuleSet::dir_prefix()` for the
/// resolved directory; this field is exactly what a person (or an
/// unedited default) put in the file.
///
/// # `#[non_exhaustive]` (RFC 052's treatment, applied here by RFC 058)
///
/// ```compile_fail
/// use apimock_routing::rule_set::prefix::Prefix;
///
/// let _ = Prefix {
///     url_path_prefix: todo!(),
///     respond_dir_prefix: todo!(),
/// };
/// ```
#[derive(Clone, Default, Deserialize, Debug)]
#[non_exhaustive]
pub struct Prefix {
    #[serde(rename = "url_path")]
    pub url_path_prefix: Option<String>,
    #[serde(rename = "respond_dir")]
    pub respond_dir_prefix: Option<String>,
}

impl Prefix {
    /// Validate that this prefix's response directory exists.
    ///
    /// Takes the *resolved* directory (`RuleSet::resolved_respond_dir`)
    /// rather than reading `self.respond_dir_prefix` directly — since
    /// RFC 058, that field holds the authored, rule-set-relative value,
    /// which does not `Path::exists()`-check correctly against the
    /// process's CWD. Existence is still gated on whether the user
    /// actually wrote a `respond_dir` at all (`respond_dir_prefix.is_some()`),
    /// matching the pre-RFC-058 behaviour exactly — a rule set that
    /// never mentioned `respond_dir` is never held to this check.
    pub fn validate(&self, resolved_respond_dir: &str, rule_set_idx: usize) -> bool {
        if self.respond_dir_prefix.is_some() {
            let exists = Path::new(resolved_respond_dir).exists();
            if !exists {
                log::error!(
                    "{} of prefix (rule set #{}):\n`{}`",
                    style("directory not found").red(),
                    rule_set_idx,
                    resolved_respond_dir
                );
            }
            exists
        } else {
            true
        }
    }
}

impl std::fmt::Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let has_written = self.url_path_prefix.is_some() || self.respond_dir_prefix.is_some();

        if let Some(url_path_prefix) = self.url_path_prefix.as_ref() {
            let _ = writeln!(f, "[url_path_prefix] {}", style(url_path_prefix).magenta());
        }

        if self.respond_dir_prefix.is_some() {
            let _ = writeln!(
                f,
                "[respond_dir_prefix] {}",
                style(self.respond_dir_prefix.clone().unwrap_or_default().as_str()).magenta()
            );
        }

        if has_written {
            let _ = writeln!(f);
        }

        Ok(())
    }
}
