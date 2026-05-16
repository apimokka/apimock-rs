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

use crate::rule_set::rule::respond::Respond;
use crate::rule_set::rule::when::When;
use crate::rule_set::rule::when::request::Request;
use crate::rule_set::rule::when::request::http_method::HttpMethod;
use crate::rule_set::rule::when::request::rule_op::RuleOp;
use crate::rule_set::rule::when::request::url_path::UrlPathConfig;
use crate::rule_set::rule::Rule;
use crate::rule_set::RuleSet;

use crate::view::{
    FileNodeKind, FileNodeView, FileTreeView, RespondView, RouteCatalogSnapshot, RuleSetView,
    RuleView, ScriptRouteView, UrlPathView, WhenView,
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
        has_header_conditions: req.headers.is_some(),
        has_body_conditions: req.body.is_some(),
    }
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
fn op_name(op: &RuleOp) -> String {
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
// File tree (depth-1 eager)
// -------------------------------------------------------------------

/// Build a depth-1 file-tree view rooted at `root`.
///
/// Returns `None` if the directory doesn't exist or can't be read.
/// Subdirectories carry `children = Some(Vec::new())` to flag them as
/// "expandable but not yet expanded"; the embedder later calls a
/// `list_directory` API to populate them on demand.
///
/// # Filtering policy (see ROADMAP.md)
///
/// 5.3.0 enumerates every direct child without filtering. Hidden
/// folders like `.git` show up as expandable nodes but their contents
/// are not loaded until the user clicks. Filtering policy (dotfile
/// prefix vs `.gitignore` vs configurable patterns) is deferred —
/// performance is unaffected and the design space is wide enough that
/// committing now is premature.
pub fn build_file_tree(root: &Path) -> Option<FileTreeView> {
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
        let kind = if metadata.is_dir() {
            FileNodeKind::Directory
        } else {
            FileNodeKind::File
        };

        // route_hint: for files, the URL path that would serve this
        // file under the dyn-route fallback. The convention is the
        // file stem prefixed with `/`. Directories get None.
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

    // Stable rendering: directories first, then files; alphabetical
    // within each group.
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
/// expansion — the embedder calls this when a GUI clicks to expand a
/// previously-collapsed directory node.
pub fn list_directory(path: &Path) -> Vec<FileNodeView> {
    build_file_tree(path)
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
