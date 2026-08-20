//! RFC 016's per-condition commands — add / update / remove a header
//! or body match condition on an existing rule.
//!
//! Split out of `edit.rs` (RFC 043) — a pure move, along the seam its
//! own section comments already marked. See `edit.rs`'s module doc for
//! why the split follows this shape.

use crate::error::ApplyError;

use super::super::Workspace;
use super::super::id_index::NodeAddress;
use super::payload;

impl Workspace {
    pub(super) fn cmd_add_header_condition(
        &mut self,
        rule_id: crate::view::NodeId,
        payload: crate::view::HeaderConditionPayload,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::headers::HeaderConditionStatement;

        let (rs_idx, rule_idx) = self.find_rule_indices(rule_id)?;
        let op = payload::header_op_to_routing_pub(payload.op);
        let value = payload.value.unwrap_or_default();
        let stmt = HeaderConditionStatement {
            op: Some(op),
            value,
        };
        let name = payload.name.to_lowercase();

        // Ensure headers map exists.
        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        let headers = rule.when.request.headers.get_or_insert_with(|| {
            apimock_routing::rule_set::rule::when::request::headers::Headers(
                indexmap::IndexMap::new(),
            )
        });
        headers.0.insert(name.clone(), stmt);

        let cond_id = self.ids.insert(NodeAddress::HeaderCondition {
            rule_set: rs_idx,
            rule: rule_idx,
            header_name: name,
        });
        let rule_id_out = self
            .ids
            .id_for(NodeAddress::Rule {
                rule_set: rs_idx,
                rule: rule_idx,
            })
            .unwrap_or(rule_id);
        Ok(vec![rule_id_out, cond_id])
    }

    pub(super) fn cmd_update_header_condition(
        &mut self,
        id: crate::view::NodeId,
        payload: crate::view::HeaderConditionPayload,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::headers::HeaderConditionStatement;

        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let (rs_idx, rule_idx, old_name) = match addr {
            NodeAddress::HeaderCondition {
                rule_set,
                rule,
                header_name,
            } => (rule_set, rule, header_name),
            _ => {
                return Err(ApplyError::InvalidPayload {
                    reason: "id does not refer to a header condition".to_owned(),
                });
            }
        };

        let op = payload::header_op_to_routing_pub(payload.op);
        let value = payload.value.unwrap_or_default();
        let new_name = payload.name.to_lowercase();
        let stmt = HeaderConditionStatement {
            op: Some(op),
            value,
        };

        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        let headers = rule.when.request.headers.get_or_insert_with(|| {
            apimock_routing::rule_set::rule::when::request::headers::Headers(
                indexmap::IndexMap::new(),
            )
        });

        // Remove old key, insert under new name (supports rename).
        headers.0.shift_remove(&old_name);
        headers.0.insert(new_name.clone(), stmt);

        // Re-register the condition under the new name.
        let new_id = self.ids.insert(NodeAddress::HeaderCondition {
            rule_set: rs_idx,
            rule: rule_idx,
            header_name: new_name,
        });
        Ok(vec![new_id])
    }

    pub(super) fn cmd_remove_header_condition(
        &mut self,
        id: crate::view::NodeId,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let (rs_idx, rule_idx, name) = match addr {
            NodeAddress::HeaderCondition {
                rule_set,
                rule,
                header_name,
            } => (rule_set, rule, header_name),
            _ => {
                return Err(ApplyError::InvalidPayload {
                    reason: "id does not refer to a header condition".to_owned(),
                });
            }
        };

        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        if let Some(headers) = rule.when.request.headers.as_mut() {
            headers.0.shift_remove(&name);
            if headers.0.is_empty() {
                rule.when.request.headers = None;
            }
        }

        let rule_id = self
            .ids
            .id_for(NodeAddress::Rule {
                rule_set: rs_idx,
                rule: rule_idx,
            })
            .unwrap_or(id);
        Ok(vec![rule_id])
    }

    pub(super) fn cmd_add_body_condition(
        &mut self,
        rule_id: crate::view::NodeId,
        payload: crate::view::BodyConditionPayload,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::body::{
            Body, BodyConditionStatement, body_kind::BodyKind,
        };

        let (rs_idx, rule_idx) = self.find_rule_indices(rule_id)?;
        let op = payload::body_op_to_routing_pub(payload.op);
        let value = payload::json_value_to_string_pub(&payload.value);
        let stmt = BodyConditionStatement {
            op: Some(op),
            value,
        };
        let path = payload.path.clone();

        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        if rule.when.request.body.is_none() {
            rule.when.request.body = Some(Body(std::collections::HashMap::new()));
        }
        let body_map = rule.when.request.body.as_mut().unwrap();
        body_map
            .0
            .entry(BodyKind::Json)
            .or_default()
            .insert(path.clone(), stmt);

        let cond_id = self.ids.insert(NodeAddress::BodyCondition {
            rule_set: rs_idx,
            rule: rule_idx,
            path,
        });
        let rule_id_out = self
            .ids
            .id_for(NodeAddress::Rule {
                rule_set: rs_idx,
                rule: rule_idx,
            })
            .unwrap_or(rule_id);
        Ok(vec![rule_id_out, cond_id])
    }

    pub(super) fn cmd_update_body_condition(
        &mut self,
        id: crate::view::NodeId,
        payload: crate::view::BodyConditionPayload,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::body::BodyConditionStatement;

        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let (rs_idx, rule_idx, old_path) = match addr {
            NodeAddress::BodyCondition {
                rule_set,
                rule,
                path,
            } => (rule_set, rule, path),
            _ => {
                return Err(ApplyError::InvalidPayload {
                    reason: "id does not refer to a body condition".to_owned(),
                });
            }
        };

        let op = payload::body_op_to_routing_pub(payload.op);
        let value = payload::json_value_to_string_pub(&payload.value);
        let new_path = payload.path.clone();
        let stmt = BodyConditionStatement {
            op: Some(op),
            value,
        };

        use apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind;
        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        if let Some(body) = rule.when.request.body.as_mut()
            && let Some(json_map) = body.0.get_mut(&BodyKind::Json)
        {
            json_map.shift_remove(&old_path);
            json_map.insert(new_path.clone(), stmt);
        }

        let new_id = self.ids.insert(NodeAddress::BodyCondition {
            rule_set: rs_idx,
            rule: rule_idx,
            path: new_path,
        });
        Ok(vec![new_id])
    }

    pub(super) fn cmd_remove_body_condition(
        &mut self,
        id: crate::view::NodeId,
    ) -> Result<Vec<crate::view::NodeId>, ApplyError> {
        use apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind;

        let addr = self.ids.lookup(id).ok_or(ApplyError::UnknownNode { id })?;
        let (rs_idx, rule_idx, path) = match addr {
            NodeAddress::BodyCondition {
                rule_set,
                rule,
                path,
            } => (rule_set, rule, path),
            _ => {
                return Err(ApplyError::InvalidPayload {
                    reason: "id does not refer to a body condition".to_owned(),
                });
            }
        };

        let rule = &mut self.config.service.rule_sets[rs_idx].rules[rule_idx];
        if let Some(body) = rule.when.request.body.as_mut()
            && let Some(json_map) = body.0.get_mut(&BodyKind::Json)
        {
            json_map.shift_remove(&path);
        }

        let rule_id = self
            .ids
            .id_for(NodeAddress::Rule {
                rule_set: rs_idx,
                rule: rule_idx,
            })
            .unwrap_or(id);
        Ok(vec![rule_id])
    }

    /// Resolve a rule's `(rule_set_idx, rule_idx)` pair from its `NodeId`.
    fn find_rule_indices(
        &self,
        rule_id: crate::view::NodeId,
    ) -> Result<(usize, usize), ApplyError> {
        match self.ids.lookup(rule_id) {
            Some(NodeAddress::Rule { rule_set, rule }) => Ok((rule_set, rule)),
            _ => Err(ApplyError::UnknownNode { id: rule_id }),
        }
    }
}
