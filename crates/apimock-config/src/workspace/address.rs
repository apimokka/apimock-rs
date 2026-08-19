//! Natural-key addressing for editable nodes (RFC 057).
//!
//! # Why this exists
//!
//! `NodeId` is a fresh UUID per `load()` — stable within one `Workspace`
//! instance, meaningless across a process boundary. `apimock set` is a
//! new process per invocation, so it cannot address anything by
//! `NodeId`: an ID printed by one invocation is unusable by the next.
//!
//! `NodeAddress` (`id_index.rs`) already models a natural-key-shaped
//! position — `rule_set: usize` / `rule: usize` — but it is
//! `pub(crate)`, on purpose (see that module's doc and RFC 057's
//! handoff § 2 Unresolved 3): it is positional rather than natural (an
//! index into `service.rule_sets`, not a path), and it carries variants
//! `set` doesn't expose (`BodyCondition`, `Middleware`,
//! `FallbackRespondDir`). Publishing it would freeze an internal shape
//! onto a contract that must stay stable.
//!
//! So this module is deliberately small: a **resolution function** —
//! turn an already-path-resolved rule-set index plus a 0-based rule
//! index into the `NodeId` `apply()` needs — and an **address
//! renderer** — turn a `NodeId` back into a natural-key string for a
//! preview or diff summary, so neither ever has to serialise a
//! `NodeId`. The path half of `set`'s (path, index) address is resolved
//! by the caller against `Workspace::config().service.rule_sets`,
//! which is already public; nothing here duplicates that.

use crate::view::{DiffItem, NodeId};

use super::Workspace;
use super::id_index::NodeAddress;

impl Workspace {
    /// The `NodeId` of the rule set at this index, if one exists.
    pub fn rule_set_id_at(&self, rule_set: usize) -> Option<NodeId> {
        self.ids.id_for(NodeAddress::RuleSet { rule_set })
    }

    /// The `NodeId` of the rule at `(rule_set, rule)`, if one exists.
    /// Both indices are 0-based, matching `get`'s JSON `matched` block
    /// (RFC 057's handoff § 1.3 — `set` takes the same base as the
    /// machine-readable contract, not the 1-based text display).
    pub fn rule_id_at(&self, rule_set: usize, rule: usize) -> Option<NodeId> {
        self.ids.id_for(NodeAddress::Rule { rule_set, rule })
    }

    /// The `NodeId` of the `respond` block at `(rule_set, rule)`, if one
    /// exists. Distinct from `rule_id_at`'s `NodeId` — `respond` is its
    /// own addressable node (`EditCommand::UpdateRespond` targets it,
    /// not the rule).
    pub fn respond_id_at(&self, rule_set: usize, rule: usize) -> Option<NodeId> {
        self.ids.id_for(NodeAddress::Respond { rule_set, rule })
    }

    /// Render a `NodeId` back to a human-readable natural-key
    /// description — never the UUID itself. Used to label `--dry-run`
    /// previews and diff-summary rows without the single highest-risk
    /// mistake this RFC's handoff calls out: serialising `DiffItem`
    /// (whose `target: NodeId` is `#[serde(transparent)]` over a
    /// `Uuid`) directly into `set`'s JSON output.
    ///
    /// A plain `String` rather than a typed enum deliberately: RFC 057
    /// § 2 Unresolved 3 asks for a renderer, not a second address type
    /// to keep stable — a string has no shape to freeze. Returns `None`
    /// only for a `NodeId` this workspace's index has never seen (a
    /// stale ID from a different `load()`), which `set`'s one-load,
    /// one-invocation lifecycle should never actually produce.
    pub fn describe(&self, id: NodeId) -> Option<String> {
        let rule_set_label = |rule_set: usize| -> String {
            self.config
                .service
                .rule_sets
                .get(rule_set)
                .map(|rs| rs.file_path.clone())
                .unwrap_or_else(|| format!("rule set #{rule_set}"))
        };
        let addr = self.ids.lookup(id)?;
        Some(match addr {
            NodeAddress::Root => "root config".to_owned(),
            NodeAddress::RuleSet { rule_set } => {
                format!("rule set `{}`", rule_set_label(rule_set))
            }
            NodeAddress::Rule { rule_set, rule } => {
                format!("rule set `{}`, rule #{rule}", rule_set_label(rule_set))
            }
            NodeAddress::Respond { rule_set, rule } => {
                format!(
                    "rule set `{}`, rule #{rule} respond",
                    rule_set_label(rule_set)
                )
            }
            NodeAddress::Middleware { middleware } => format!("middleware #{middleware}"),
            NodeAddress::FallbackRespondDir => "fallback_respond_dir".to_owned(),
            NodeAddress::HeaderCondition {
                rule_set,
                rule,
                header_name,
            } => format!(
                "rule set `{}`, rule #{rule}, header `{header_name}`",
                rule_set_label(rule_set)
            ),
            NodeAddress::BodyCondition {
                rule_set,
                rule,
                path,
            } => format!(
                "rule set `{}`, rule #{rule}, body `{path}`",
                rule_set_label(rule_set)
            ),
        })
    }

    /// What a `save()` right now would write, without writing it.
    /// Thin public wrapper over the existing `pub(super)`
    /// `compute_diff_summary` — RFC 057's `--dry-run` needs exactly
    /// this and nothing `save()`'s other side effects (the atomic
    /// writes, the baseline refresh).
    pub fn preview_changes(&self) -> Vec<DiffItem> {
        self.compute_diff_summary()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        Workspace,
        view::{EditCommand, RespondPayload, RulePayload},
    };

    /// A minimal on-disk workspace, local to this module — `address.rs`
    /// lives outside `workspace/tests/`, so `workspace/tests/common.rs`'s
    /// `pub(super)` fixtures aren't visible here (they're scoped to
    /// `workspace::tests`, not `workspace::address`).
    fn workspace_with_two_rules() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let rs_toml = concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/a\"\n",
            "respond = { text = \"a\" }\n",
            "\n",
            "[[rules]]\n",
            "when.request.url_path = \"/b\"\n",
            "respond = { text = \"b\" }\n",
        );
        let rs_path = dir.path().join("apimock-rule-set.toml");
        std::fs::write(&rs_path, rs_toml).unwrap();
        let root_toml =
            "[service]\nrule_sets = [\"apimock-rule-set.toml\"]\nfallback_respond_dir = \".\"\n";
        let root_path = dir.path().join("apimock.toml");
        std::fs::write(&root_path, root_toml).unwrap();
        (dir, root_path)
    }

    #[test]
    fn rule_set_id_at_resolves_a_freshly_loaded_rule_set() {
        let (_dir, root) = workspace_with_two_rules();
        let ws = Workspace::load(root).expect("load");
        assert!(ws.rule_set_id_at(0).is_some());
        assert!(ws.rule_set_id_at(1).is_none(), "only one rule set exists");
    }

    #[test]
    fn rule_id_at_resolves_a_rule_seeded_at_load_not_just_one_added_this_session() {
        let (_dir, root) = workspace_with_two_rules();
        let ws = Workspace::load(root).expect("load");
        assert!(ws.rule_id_at(0, 0).is_some());
        assert!(ws.rule_id_at(0, 1).is_some());
        assert!(
            ws.rule_id_at(0, 2).is_none(),
            "out-of-range rule index must resolve to None, not panic"
        );
    }

    #[test]
    fn describe_never_contains_a_uuid_shaped_substring() {
        let (_dir, root) = workspace_with_two_rules();
        let mut ws = Workspace::load(root).expect("load");
        let parent = ws.rule_set_id_at(0).expect("rule set 0 exists");
        let result = ws
            .apply(EditCommand::AddRule {
                parent,
                rule: RulePayload {
                    url_path: Some("/new".to_owned()),
                    url_path_op: None,
                    method: None,
                    priority: None,
                    headers: None,
                    body: None,
                    respond: RespondPayload {
                        text: Some("ok".to_owned()),
                        ..Default::default()
                    },
                },
            })
            .expect("apply");

        for id in result.changed_nodes {
            let label = ws.describe(id).expect("every changed node describes");
            assert!(
                !looks_like_a_uuid(&label),
                "describe() must never render a UUID: {label}"
            );
        }
    }

    fn looks_like_a_uuid(s: &str) -> bool {
        // 8-4-4-4-12 hex groups joined by hyphens.
        let groups: Vec<&str> = s.split('-').collect();
        groups.len() >= 5
            && groups.windows(5).any(|w| {
                w.iter()
                    .all(|g| !g.is_empty() && g.chars().all(|c| c.is_ascii_hexdigit()))
            })
    }

    #[test]
    fn preview_changes_matches_what_save_would_report() {
        let (_dir, root) = workspace_with_two_rules();
        let mut ws = Workspace::load(root).expect("load");
        let parent = ws.rule_set_id_at(0).expect("rule set 0 exists");
        ws.apply(EditCommand::AddRule {
            parent,
            rule: RulePayload {
                url_path: Some("/preview".to_owned()),
                url_path_op: None,
                method: None,
                priority: None,
                headers: None,
                body: None,
                respond: RespondPayload {
                    text: Some("ok".to_owned()),
                    ..Default::default()
                },
            },
        })
        .expect("apply");

        let preview = ws.preview_changes();
        assert!(!preview.is_empty(), "an applied add should show in preview");
        let save = ws.save().expect("save");
        assert_eq!(
            preview.len(),
            save.diff_summary.len(),
            "preview_changes must match what save() then actually reports"
        );
    }
}
