//! `Workspace::snapshot()` and the per-file view builders it composes.
//!
//! # Why this is a separate module
//!
//! Snapshot is read-only; it owns no mutating logic. Keeping it apart
//! from the edit / save modules makes the read path easy to reason
//! about — there's no chance a snapshot helper accidentally mutates
//! the model because the `&self` receiver here can't.
//!
//! # Per-node validation runs here too
//!
//! `rule_set_file_view` calls into [`crate::workspace::validate`]
//! to attach `NodeValidation` to each respond node. That means
//! snapshot rendering and `validate()` walk the same checks; a node
//! marked invalid in the snapshot will also appear in
//! `ValidationReport::diagnostics`.

use std::path::PathBuf;

use apimock_routing::RuleSet;

use crate::view::{
    ConfigFileKind, ConfigFileView, ConfigNodeView, NodeKind, NodeValidation, WorkspaceSnapshot,
};

use super::Workspace;
use super::id_index::NodeAddress;
use super::path_helpers::file_basename;
use super::validate::respond_node_validation;

impl Workspace {
    /// Build a snapshot for GUI rendering.
    ///
    /// # Allocation cost
    ///
    /// A snapshot fully owns its data (no borrows into the workspace)
    /// so the GUI can serialise / send / store it without lifetime
    /// gymnastics. This is O(total editable nodes) allocation per
    /// call; the GUI should call it once per edit, not once per
    /// render frame.
    pub fn snapshot(&self) -> WorkspaceSnapshot {
        let mut files: Vec<ConfigFileView> = Vec::new();

        // Root file.
        if let Some(root_nodes) = self.root_file_nodes() {
            files.push(root_nodes);
        }

        // Rule-set files.
        for (rs_idx, rule_set) in self.config.service.rule_sets.iter().enumerate() {
            files.push(self.rule_set_file_view(rs_idx, rule_set));
        }

        // Middleware files. We don't introspect them beyond their path
        // existence; the Rhai AST is a server-side concern.
        if let Some(paths) = self.config.service.middlewares_file_paths.as_ref() {
            for (mw_idx, mw_path) in paths.iter().enumerate() {
                let abs = self.resolve_relative(mw_path);
                let id = self
                    .ids
                    .id_for(NodeAddress::Middleware { middleware: mw_idx })
                    .expect("middleware id seeded at load");
                let node = ConfigNodeView {
                    id,
                    source_file: abs.clone(),
                    toml_path: format!("service.middlewares[{}]", mw_idx),
                    display_name: mw_path.clone(),
                    kind: NodeKind::Script,
                    validation: NodeValidation::ok(),
                };
                files.push(ConfigFileView {
                    path: abs.clone(),
                    display_name: file_basename(&abs),
                    kind: ConfigFileKind::Middleware,
                    nodes: vec![node],
                });
            }
        }

        // Route catalog — assemble from rule sets, fallback dir,
        // file tree (depth-1 eager), and middleware script routes.
        // Builders live in `apimock_routing::view::build`; the config
        // crate just feeds them the data they need.
        let fallback_dir = self.config.service.fallback_respond_dir.as_str();
        let fallback_abs = self.resolve_relative(fallback_dir);
        let filter = self
            .config
            .file_tree_view
            .as_ref()
            .map(|c| c.to_filter())
            .unwrap_or_default();
        let file_tree =
            apimock_routing::view::build::build_file_tree_with(&fallback_abs, &filter);

        let script_routes: Vec<apimock_routing::view::ScriptRouteView> = self
            .config
            .service
            .middlewares_file_paths
            .as_ref()
            .map(|paths| {
                paths
                    .iter()
                    .enumerate()
                    .map(|(idx, p)| apimock_routing::view::build::build_script_route_view(idx, p))
                    .collect()
            })
            .unwrap_or_default();

        let routes = apimock_routing::view::build::build_route_catalog(
            &self.config.service.rule_sets,
            Some(fallback_dir),
            file_tree,
            script_routes,
        );

        WorkspaceSnapshot {
            files,
            routes,
            diagnostics: self.diagnostics.clone(),
        }
    }
    /// Root config file as a `ConfigFileView`, if it can be rendered.
    fn root_file_nodes(&self) -> Option<ConfigFileView> {
        let mut nodes = Vec::new();

        if let Some(root_id) = self.ids.id_for(NodeAddress::Root) {
            nodes.push(ConfigNodeView {
                id: root_id,
                source_file: self.root_path.clone(),
                toml_path: String::new(),
                display_name: "apimock.toml".to_owned(),
                kind: NodeKind::RootSetting,
                validation: NodeValidation::ok(),
            });
        }

        if let Some(fb_id) = self.ids.id_for(NodeAddress::FallbackRespondDir) {
            nodes.push(ConfigNodeView {
                id: fb_id,
                source_file: self.root_path.clone(),
                toml_path: "service.fallback_respond_dir".to_owned(),
                display_name: self.config.service.fallback_respond_dir.clone(),
                kind: NodeKind::FileNode,
                validation: NodeValidation::ok(),
            });
        }

        Some(ConfigFileView {
            path: self.root_path.clone(),
            display_name: file_basename(&self.root_path),
            kind: ConfigFileKind::Root,
            nodes,
        })
    }

    fn rule_set_file_view(&self, rs_idx: usize, rule_set: &RuleSet) -> ConfigFileView {
        let file_path = PathBuf::from(rule_set.file_path.as_str());
        let mut nodes: Vec<ConfigNodeView> = Vec::new();

        // Rule-set itself.
        if let Some(rs_id) = self
            .ids
            .id_for(NodeAddress::RuleSet { rule_set: rs_idx })
        {
            nodes.push(ConfigNodeView {
                id: rs_id,
                source_file: file_path.clone(),
                toml_path: String::new(),
                display_name: file_basename(&file_path),
                kind: NodeKind::RuleSet,
                validation: NodeValidation::ok(),
            });
        }

        // Rules inside.
        for (rule_idx, rule) in rule_set.rules.iter().enumerate() {
            if let Some(rule_id) = self.ids.id_for(NodeAddress::Rule {
                rule_set: rs_idx,
                rule: rule_idx,
            }) {
                let url_path_label = rule
                    .when
                    .request
                    .url_path
                    .as_ref()
                    .map(|u| u.value.as_str())
                    .unwrap_or_default();
                let display = if url_path_label.is_empty() {
                    format!("Rule #{}", rule_idx + 1)
                } else {
                    url_path_label.to_owned()
                };
                nodes.push(ConfigNodeView {
                    id: rule_id,
                    source_file: file_path.clone(),
                    toml_path: format!("rules[{}]", rule_idx),
                    display_name: display,
                    kind: NodeKind::Rule,
                    validation: NodeValidation::ok(),
                });
            }

            if let Some(resp_id) = self.ids.id_for(NodeAddress::Respond {
                rule_set: rs_idx,
                rule: rule_idx,
            }) {
                nodes.push(ConfigNodeView {
                    id: resp_id,
                    source_file: file_path.clone(),
                    toml_path: format!("rules[{}].respond", rule_idx),
                    display_name: summarise_respond(&rule.respond),
                    kind: NodeKind::Respond,
                    validation: respond_node_validation(&rule.respond, rule_set, rule_idx, rs_idx),
                });
            }
        }

        ConfigFileView {
            path: file_path.clone(),
            display_name: file_basename(&file_path),
            kind: ConfigFileKind::RuleSet,
            nodes,
        }
    }
}

fn summarise_respond(respond: &apimock_routing::Respond) -> String {
    if let Some(p) = respond.file_path.as_ref() {
        return format!("file: {}", p);
    }
    if let Some(t) = respond.text.as_ref() {
        const LIMIT: usize = 40;
        if t.chars().count() > LIMIT {
            let truncated: String = t.chars().take(LIMIT).collect();
            return format!("text: {}…", truncated);
        }
        return format!("text: {}", t);
    }
    if let Some(s) = respond.status.as_ref() {
        return format!("status: {}", s);
    }
    "(empty)".to_owned()
}
