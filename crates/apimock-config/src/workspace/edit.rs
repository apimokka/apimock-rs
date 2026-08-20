//! `Workspace::apply()` dispatch, plus the per-command handlers split
//! across sibling modules (RFC 043).
//!
//! # Layout
//!
//! `apply()` below is the only public surface: a `match` over
//! `EditCommand` that dispatches to one `cmd_*` method per variant.
//! The `cmd_*` methods themselves live in sibling modules, grouped by
//! the same seams the RFCs that added them already drew:
//!
//! - [`rule_set`] — add/remove a rule set, and the RFC 025 per-rule-set
//!   strategy override.
//! - [`rule`] — add/update/delete/move a rule, and update its `respond`.
//! - [`root_setting`] — `cmd_update_root_setting`, kept as one function
//!   (171 lines, the largest body in the crate) since decomposing it is
//!   a question about that function, not about module layout.
//! - [`condition`] — RFC 016's six header/body condition commands.
//!
//! Every `cmd_*` method is `pub(super)`, visible only within
//! `workspace::edit` and its descendants — none of them is part of
//! this crate's public API, and the split doesn't change that.
//!
//! ID migration helpers (the `shift_*` and `reorder_*` methods) live
//! in [`id_shift`] because they're a self-contained concern: they
//! don't read or mutate `self.config` directly, only `self.ids`.
//! Splitting them out makes the `cmd_*` bodies shorter and the helpers
//! separately testable.
//!
//! Payload-to-model converters live in [`payload`] — pure functions
//! translating GUI-shaped `EditValue` / `RulePayload` into the routing
//! crate's runtime types.

pub mod condition;
pub mod id_shift;
pub mod payload;
pub mod root_setting;
pub mod rule;
pub mod rule_set;

use crate::error::ApplyError;
use crate::view::{ApplyResult, EditCommand};

use super::Workspace;

impl Workspace {
    /// Apply one edit command, mutating the in-memory workspace.
    ///
    /// # Shape of the implementation
    ///
    /// Each `EditCommand` variant maps to a small helper method. The
    /// helpers return `Result<Vec<NodeId>, ApplyError>`; `apply` wraps
    /// the ok-path in an `ApplyResult` with the right `requires_reload`
    /// flag and reruns validation so the result carries up-to-date
    /// diagnostics.
    ///
    /// # ID stability on structural changes
    ///
    /// Commands that change positional layout (Remove / Delete / Move
    /// / Add) touch `self.ids` carefully so NodeIds that refer to the
    /// *same logical node* survive the operation. For example, after
    /// `RemoveRuleSet { id }` at index `i`, rule sets at positions
    /// `i+1..` shift down by one: the code below explicitly migrates
    /// their IDs so a GUI that selected rule-set #3 before the edit
    /// still has the same ID pointing at what is now rule-set #2.
    pub fn apply(&mut self, cmd: EditCommand) -> Result<ApplyResult, ApplyError> {
        let (changed_nodes, requires_reload) = match cmd {
            EditCommand::AddRuleSet { path } => {
                let ids = self.cmd_add_rule_set(path)?;
                (ids, true)
            }
            EditCommand::RemoveRuleSet { id } => {
                let ids = self.cmd_remove_rule_set(id)?;
                (ids, true)
            }
            EditCommand::AddRule { parent, rule } => {
                let ids = self.cmd_add_rule(parent, rule)?;
                (ids, true)
            }
            EditCommand::UpdateRule { id, rule } => {
                let ids = self.cmd_update_rule(id, rule)?;
                (ids, true)
            }
            EditCommand::DeleteRule { id } => {
                let ids = self.cmd_delete_rule(id)?;
                (ids, true)
            }
            EditCommand::MoveRule { id, new_index } => {
                let ids = self.cmd_move_rule(id, new_index)?;
                (ids, true)
            }
            EditCommand::UpdateRespond { id, respond } => {
                let ids = self.cmd_update_respond(id, respond)?;
                (ids, true)
            }
            EditCommand::UpdateRootSetting { key, value } => {
                let ids = self.cmd_update_root_setting(key, value)?;
                (ids, true)
            }

            // ── Per-condition commands (RFC 016) ──────────────────────
            EditCommand::AddHeaderCondition { rule_id, condition } => {
                let ids = self.cmd_add_header_condition(rule_id, condition)?;
                (ids, true)
            }
            EditCommand::UpdateHeaderCondition { id, condition } => {
                let ids = self.cmd_update_header_condition(id, condition)?;
                (ids, true)
            }
            EditCommand::RemoveHeaderCondition { id } => {
                let ids = self.cmd_remove_header_condition(id)?;
                (ids, true)
            }
            EditCommand::AddBodyCondition { rule_id, condition } => {
                let ids = self.cmd_add_body_condition(rule_id, condition)?;
                (ids, true)
            }
            EditCommand::UpdateBodyCondition { id, condition } => {
                let ids = self.cmd_update_body_condition(id, condition)?;
                (ids, true)
            }
            EditCommand::RemoveBodyCondition { id } => {
                let ids = self.cmd_remove_body_condition(id)?;
                (ids, true)
            }
            EditCommand::UpdateRuleSetStrategy { id, strategy } => {
                let ids = self.cmd_update_rule_set_strategy(id, strategy)?;
                (ids, true) // SoftReload — strategy change takes effect at next match
            }
        };

        // After any mutation, refresh per-node validation so the
        // `ApplyResult.diagnostics` reflects the new state. This is the
        // Step-3 piece: validation is now per-node and GUI-ready, not a
        // bare boolean.
        let diagnostics = self.collect_diagnostics();

        Ok(ApplyResult {
            changed_nodes,
            diagnostics,
            requires_reload,
        })
    }
}
