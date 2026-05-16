use serde::Deserialize;

/// Persistent filter preferences for [`FileTreeView`], loaded from the
/// optional `[file_tree_view]` section of `apimock.toml`.
///
/// When the section is absent, [`FileTreeViewConfig::default()`] is used,
/// which mirrors [`apimock_routing::view::build::FileTreeFilter::default()`]:
/// dotfiles hidden, built-in excludes on, no extra filters.
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

    /// Additional entry names to exclude (exact match on `file_name()`).
    #[serde(default)]
    pub extra_excludes: Vec<String>,

    /// If non-empty, only files whose name ends with one of these suffixes
    /// are shown. Directories always pass the include filter so the user
    /// can drill into them. (default: `[]` — show everything)
    #[serde(default)]
    pub include: Vec<String>,
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
        }
    }
}

impl FileTreeViewConfig {
    /// Convert to the routing crate's [`FileTreeFilter`].
    ///
    /// [`FileTreeFilter`]: apimock_routing::view::build::FileTreeFilter
    pub fn to_filter(&self) -> apimock_routing::view::build::FileTreeFilter {
        apimock_routing::view::build::FileTreeFilter {
            show_hidden: self.show_hidden,
            builtin_excludes: self.builtin_excludes,
            extra_excludes: self.extra_excludes.clone(),
            include: self.include.clone(),
        }
    }
}
