use serde::Deserialize;

/// Persistent filter preferences for [`FileTreeView`], loaded from the
/// optional `[file_tree_view]` section of `apimock.toml`.
///
/// When the section is absent, [`FileTreeViewConfig::default()`] is used,
/// which mirrors [`apimock_routing::view::build::FileTreeFilter::default()`]:
/// dotfiles hidden, built-in excludes on, no extra filters, gitignore off.
///
/// [`FileTreeView`]: apimock_routing::view::FileTreeView
#[derive(Clone, Debug, Deserialize)]
pub struct FileTreeViewConfig {
    /// Show dotfiles and dot-directories (default: `false`).
    #[serde(default)]
    pub show_hidden: bool,

    /// Apply the built-in exclude list (`target`, `node_modules`, etc.)
    /// (default: `true`).
    #[serde(default = "default_true")]
    pub builtin_excludes: bool,

    /// Glob patterns for additional exclusions (RFC 019).
    ///
    /// Each entry is matched against the entry's `file_name()` only.
    /// Supports standard glob syntax (`*`, `?`, `[…]`). A trailing `/`
    /// restricts the pattern to directories. Pre-5.11 exact-name entries
    /// continue to work because a bare name is a valid glob.
    #[serde(default)]
    pub extra_excludes: Vec<String>,

    /// If non-empty, only files whose name matches at least one of these
    /// glob patterns are shown. Directories always pass the include filter
    /// so the user can drill into them. (default: `[]` — show everything)
    #[serde(default)]
    pub include: Vec<String>,

    /// Parse `.gitignore` files in the tree root and its ancestors,
    /// applying Git-compatible ignore rules (RFC 019). (default: `false`)
    #[serde(default)]
    pub respect_gitignore: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FileTreeViewConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            builtin_excludes: true,
            extra_excludes: Vec::new(),
            include: Vec::new(),
            respect_gitignore: false,
        }
    }
}

impl FileTreeViewConfig {
    /// Convert to the routing crate's [`FileTreeFilter`].
    ///
    /// [`FileTreeFilter`]: apimock_routing::view::build::FileTreeFilter
    pub fn to_filter(&self) -> apimock_routing::view::build::FileTreeFilter {
        let mut filter = apimock_routing::view::build::FileTreeFilter::default();
        filter.show_hidden = self.show_hidden;
        filter.builtin_excludes = self.builtin_excludes;
        filter.extra_excludes = self.extra_excludes.clone();
        filter.include = self.include.clone();
        filter.respect_gitignore = self.respect_gitignore;
        filter
    }
}
