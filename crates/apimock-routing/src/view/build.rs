//! Builders that turn the in-memory routing model into the view types
//! a GUI consumes.
//!
//! # Why these aren't `From` impls
//!
//! The view shapes need contextual information the source types don't
//! carry — the `index` field on `RuleSetView` and `RuleView` for
//! example. Free functions taking the index alongside the model keep
//! the call sites explicit and type-checked. A `From<&RuleSet>` impl
//! would have to invent the index (probably defaulting to zero) which
//! is just the kind of silent-bug surface we want to avoid.
//!
//! # Why this is a sibling module rather than baked into `view.rs`
//!
//! `view.rs` is the *type* surface — it must stay stable across
//! routing-crate refactors so a GUI's bindings don't churn. Builders
//! depend on the internal `RuleSet` / `Rule` / `When` shapes which
//! *do* churn. Keeping them in their own module makes the dependency
//! direction obvious: `view::build` may import from anywhere in the
//! crate; `view.rs` itself stays leaf.

use std::path::Path;

use serde_json;

use crate::rule_set::rule::respond::Respond;
use crate::rule_set::rule::when::When;
use crate::rule_set::rule::when::request::Request;
use crate::rule_set::rule::when::request::http_method::HttpMethod;
use crate::rule_set::rule::when::request::rule_op::RuleOp;
use crate::rule_set::rule::when::request::url_path::UrlPathConfig;
use crate::rule_set::rule::Rule;
use crate::rule_set::RuleSet;

use crate::view::{
    BodyConditionView, FileNodeKind, FileNodeView, FileTreeView, HeaderConditionView,
    RespondView, RouteCatalogSnapshot, RuleSetView, RuleView, ScriptRouteView, UrlPathView,
    WhenView,
};

/// Compose the top-level `RouteCatalogSnapshot` from already-built
/// components. Caller supplies `file_tree` and `script_routes` because
/// the routing crate doesn't know about middleware-file paths or the
/// fallback dir's location — those live in `apimock-config`.
pub fn build_route_catalog(
    rule_sets: &[RuleSet],
    fallback_respond_dir: Option<&str>,
    file_tree: Option<FileTreeView>,
    script_routes: Vec<ScriptRouteView>,
) -> RouteCatalogSnapshot {
    let rule_set_views = rule_sets
        .iter()
        .enumerate()
        .map(|(idx, rs)| build_rule_set_view(rs, idx))
        .collect();

    RouteCatalogSnapshot {
        rule_sets: rule_set_views,
        fallback_respond_dir: fallback_respond_dir.map(str::to_owned),
        file_tree,
        script_routes,
    }
}

pub fn build_rule_set_view(rule_set: &RuleSet, index: usize) -> RuleSetView {
    let (url_prefix, dir_prefix) = match rule_set.prefix.as_ref() {
        Some(p) => (p.url_path_prefix.clone(), p.respond_dir_prefix.clone()),
        None => (None, None),
    };

    RuleSetView {
        index,
        source_path: rule_set.file_path.clone(),
        url_path_prefix: url_prefix,
        respond_dir_prefix: dir_prefix,
        rules: rule_set
            .rules
            .iter()
            .enumerate()
            .map(|(idx, r)| build_rule_view(r, idx))
            .collect(),
    }
}

pub fn build_rule_view(rule: &Rule, index: usize) -> RuleView {
    RuleView {
        index,
        when: build_when_view(&rule.when),
        respond: build_respond_view(&rule.respond),
    }
}

pub fn build_when_view(when: &When) -> WhenView {
    let req: &Request = &when.request;
    WhenView {
        url_path: build_url_path_view(req.url_path_config.as_ref()),
        method: req.http_method.as_ref().map(http_method_name),
        headers: build_header_condition_views(req.headers.as_ref()),
        body: build_body_condition_views(req.body.as_ref()),
    }
}

fn build_header_condition_views(
    headers: Option<&crate::rule_set::rule::when::request::headers::Headers>,
) -> Vec<HeaderConditionView> {
    let headers = match headers {
        Some(h) => h,
        None => return Vec::new(),
    };
    // IndexMap preserves insertion (TOML authoring) order — no sort needed.
    headers
        .0
        .iter()
        .map(|(name, stmt)| {
            let op_str = op_name(stmt.op.as_ref().unwrap_or(&RuleOp::default()));
            HeaderConditionView {
                name: name.clone(),
                op: op_str,
                value: Some(stmt.value.clone()),
            }
        })
        .collect()
}

fn build_body_condition_views(
    body: Option<&crate::rule_set::rule::when::request::body::Body>,
) -> Vec<BodyConditionView> {
    use crate::rule_set::rule::when::request::body::body_kind::BodyKind;
    use crate::rule_set::rule::when::request::body::body_operator::BodyOperator;

    let body = match body {
        Some(b) => b,
        None => return Vec::new(),
    };

    let mut views: Vec<BodyConditionView> = Vec::new();
    for (kind, conditions) in &body.0 {
        let kind_str = match kind {
            BodyKind::Json => "json",
        };
        for (path, stmt) in conditions {
            let op_str = format!(
                "{}",
                stmt.op.as_ref().unwrap_or(&BodyOperator::Equal)
            )
            .trim()
            .to_owned();
            // Normalise the op display string to snake_case form matching
            // the serde rename: strip surrounding spaces, lower-case.
            let op_clean = body_op_name(stmt.op.as_ref().unwrap_or(&BodyOperator::Equal));
            // value: try to parse as JSON; fall back to JSON string.
            let value = serde_json::from_str::<serde_json::Value>(&stmt.value)
                .unwrap_or_else(|_| serde_json::Value::String(stmt.value.clone()));
            let _ = op_str; // suppress unused warning
            views.push(BodyConditionView {
                kind: kind_str.to_owned(),
                path: path.clone(),
                op: op_clean,
                value,
            });
        }
    }
    // Stable order: alphabetical by path.
    views.sort_by(|a, b| a.path.cmp(&b.path));
    views
}

/// Public wrapper so `toml_writer` can serialise body operators to
/// TOML `op` strings without importing routing-internal types.
pub fn body_op_name_pub(op: &crate::rule_set::rule::when::request::body::body_operator::BodyOperator) -> String {
    body_op_name(op)
}

fn body_op_name(op: &crate::rule_set::rule::when::request::body::body_operator::BodyOperator) -> String {
    use crate::rule_set::rule::when::request::body::body_operator::BodyOperator;
    match op {
        BodyOperator::Equal => "equal",
        BodyOperator::EqualString => "equal_string",
        BodyOperator::Contains => "contains",
        BodyOperator::StartsWith => "starts_with",
        BodyOperator::EndsWith => "ends_with",
        BodyOperator::Regex => "regex",
        BodyOperator::EqualTyped => "equal_typed",
        BodyOperator::EqualNumber => "equal_number",
        BodyOperator::GreaterThan => "greater_than",
        BodyOperator::LessThan => "less_than",
        BodyOperator::GreaterOrEqual => "greater_or_equal",
        BodyOperator::LessOrEqual => "less_or_equal",
        BodyOperator::Exists => "exists",
        BodyOperator::Absent => "absent",
        BodyOperator::ArrayLengthEqual => "array_length_equal",
        BodyOperator::ArrayLengthAtLeast => "array_length_at_least",
        BodyOperator::ArrayContains => "array_contains",
        BodyOperator::EqualInteger => "equal_integer",
    }
    .to_owned()
}

fn build_url_path_view(cfg: Option<&UrlPathConfig>) -> Option<UrlPathView> {
    let cfg = cfg?;
    let (value, op) = match cfg {
        UrlPathConfig::Simple(s) => (s.clone(), op_name(&RuleOp::default())),
        UrlPathConfig::Detailed(detail) => {
            let op = detail
                .op
                .as_ref()
                .map(op_name)
                .unwrap_or_else(|| op_name(&RuleOp::default()));
            (detail.value.clone(), op)
        }
    };
    Some(UrlPathView { value, op })
}

/// TOML-form name for a `RuleOp`. The `Display` impl on `RuleOp`
/// produces a human-readable form (`" == "`, `" starts with "`),
/// which is good for log output but not for a stable identifier the
/// GUI can match against. We translate to the same `snake_case` form
/// `serde(rename_all = "snake_case")` produces on the way in, so the
/// view round-trips back to the original TOML keyword.
pub fn op_name(op: &RuleOp) -> String {
    match op {
        RuleOp::Equal => "equal",
        RuleOp::NotEqual => "not_equal",
        RuleOp::StartsWith => "starts_with",
        RuleOp::Contains => "contains",
        RuleOp::WildCard => "wild_card",
    }
    .to_owned()
}

fn http_method_name(m: &HttpMethod) -> String {
    m.as_str().to_owned()
}

pub fn build_respond_view(respond: &Respond) -> RespondView {
    if let Some(path) = respond.file_path.as_ref() {
        return RespondView::File {
            path: path.clone(),
            csv_records_key: respond.csv_records_key.clone(),
        };
    }
    if let Some(text) = respond.text.as_ref() {
        return RespondView::Text {
            text: text.clone(),
            status: respond.status,
        };
    }
    if let Some(status) = respond.status {
        return RespondView::Status { code: status };
    }
    // Fallback for an empty respond — not legal per validation, but
    // surface it as an empty text body so the snapshot stays
    // well-formed for GUIs that re-render mid-edit.
    RespondView::Text {
        text: String::new(),
        status: None,
    }
}

// -------------------------------------------------------------------
// File tree (depth-1 eager) — RFC 005 filtering
// -------------------------------------------------------------------

/// Built-in directory names to exclude from `FileTreeView` by default.
/// These are overwhelmingly build outputs / VCS metadata across common
/// ecosystems; projects with unusual layouts can disable via
/// `FileTreeFilter::builtin_excludes = false`.
pub const BUILTIN_EXCLUDES: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    "out",
    "__pycache__",
    ".venv",
    "vendor",
    ".cargo",
    ".gradle",
    ".idea",
    ".vscode",
];

/// Filter options controlling which entries appear in [`FileTreeView`].
///
/// # Defaults
///
/// - `show_hidden = false` — hide dotfiles / dot-directories.
/// - `builtin_excludes = true` — hide known build-output directories.
/// - `extra_excludes = []` — no additional exclusions.
/// - `include = []` — include everything (no inclusion filter).
///
/// The defaults are intentionally conservative: they hide the noise
/// without requiring any configuration for the common case.
#[derive(Clone, Debug)]
pub struct FileTreeFilter {
    /// When `false`, entries whose name starts with `.` are excluded.
    pub show_hidden: bool,
    /// When `true`, entries whose name appears in [`BUILTIN_EXCLUDES`]
    /// are excluded.
    pub builtin_excludes: bool,
    /// Additional entry names to exclude (exact-match on the entry's
    /// `file_name()`, not a glob).
    pub extra_excludes: Vec<String>,
    /// If non-empty, only files whose name matches at least one entry
    /// (simple suffix match) are included. Directories are always kept
    /// so the user can drill into them.
    pub include: Vec<String>,
}

impl Default for FileTreeFilter {
    fn default() -> Self {
        Self {
            show_hidden: false,
            builtin_excludes: true,
            extra_excludes: Vec::new(),
            include: Vec::new(),
        }
    }
}

impl FileTreeFilter {
    /// Return `true` iff `name` should be kept (not filtered out).
    fn keep(&self, name: &str, is_dir: bool) -> bool {
        // Dotfile filter
        if !self.show_hidden && name.starts_with('.') {
            return false;
        }
        // Built-in exclude list
        if self.builtin_excludes && BUILTIN_EXCLUDES.contains(&name) {
            return false;
        }
        // Extra excludes
        if self.extra_excludes.iter().any(|e| e == name) {
            return false;
        }
        // Include filter applies only to files; directories always pass.
        if !is_dir && !self.include.is_empty() {
            if !self.include.iter().any(|pat| name.ends_with(pat.as_str())) {
                return false;
            }
        }
        true
    }
}

/// Build a depth-1 file-tree view rooted at `root`.
///
/// Applies [`FileTreeFilter::default()`] to exclude hidden entries and
/// known build-output directories. Use [`build_file_tree_with`] to
/// supply custom filter options.
pub fn build_file_tree(root: &Path) -> Option<FileTreeView> {
    build_file_tree_with(root, &FileTreeFilter::default())
}

/// Build a depth-1 file-tree view with an explicit [`FileTreeFilter`].
///
/// Returns `None` if the directory doesn't exist or can't be read.
/// Subdirectories carry `children = Some(Vec::new())` to flag them as
/// expandable-but-not-yet-expanded.
pub fn build_file_tree_with(root: &Path, filter: &FileTreeFilter) -> Option<FileTreeView> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut nodes: Vec<FileNodeView> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = metadata.is_dir();
        let kind = if is_dir {
            FileNodeKind::Directory
        } else {
            FileNodeKind::File
        };

        // Apply filter
        if !filter.keep(&name, is_dir) {
            continue;
        }

        let route_hint = if matches!(kind, FileNodeKind::File) {
            path.file_stem()
                .map(|s| format!("/{}", s.to_string_lossy()))
        } else {
            None
        };

        let children = match kind {
            FileNodeKind::Directory => Some(Vec::new()),
            FileNodeKind::File => None,
        };

        nodes.push(FileNodeView {
            name,
            path: path.to_string_lossy().into_owned(),
            kind,
            route_hint,
            children,
        });
    }

    // Stable rendering: directories first, then files; alphabetical within each group.
    nodes.sort_by(|a, b| match (&a.kind, &b.kind) {
        (FileNodeKind::Directory, FileNodeKind::File) => std::cmp::Ordering::Less,
        (FileNodeKind::File, FileNodeKind::Directory) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Some(FileTreeView {
        root_path: root.to_string_lossy().into_owned(),
        entries: nodes,
    })
}

/// Same shape as `build_file_tree`, but for ad-hoc subdirectory
/// expansion. Applies [`FileTreeFilter::default()`].
pub fn list_directory(path: &Path) -> Vec<FileNodeView> {
    list_directory_with(path, &FileTreeFilter::default())
}

/// Subdirectory expansion with an explicit filter.
pub fn list_directory_with(path: &Path, filter: &FileTreeFilter) -> Vec<FileNodeView> {
    build_file_tree_with(path, filter)
        .map(|t| t.entries)
        .unwrap_or_default()
}

// -------------------------------------------------------------------
// Script routes
// -------------------------------------------------------------------

/// Build a `ScriptRouteView` from a middleware path and its index in
/// `service.middlewares_file_paths`.
pub fn build_script_route_view(index: usize, source_file: &str) -> ScriptRouteView {
    let display_name = Path::new(source_file)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| source_file.to_owned());

    ScriptRouteView {
        index,
        source_file: source_file.to_owned(),
        display_name,
    }
}
