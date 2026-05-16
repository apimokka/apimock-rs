//! Stable identity tracking for editable nodes.
//!
//! # Why a closed `NodeAddress` enum and not a path string
//!
//! The `apply` layer needs to mutate the underlying config, which is
//! only safe if the address is a closed, exhaustively-matchable set.
//! A free-form `"rule_sets[0].rules[2]"` string would force the apply
//! code to parse at every edit and silently accept nonsense paths.
//!
//! # Why this is its own module
//!
//! The Workspace's stable-ID machinery is a self-contained concern:
//! `NodeAddress` enumerates every kind of editable node, `IdIndex`
//! holds the bidirectional map, and a few helpers wire them up. Edit
//! operations import these types but don't reach into their internals
//! beyond the published methods on `IdIndex`.

use std::collections::HashMap;

use crate::view::NodeId;

/// Internal index mapping NodeId to an editable node's logical
/// address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeAddress {
    /// The root config (there is exactly one).
    Root,
    /// A whole rule set, identified by its index in `service.rule_sets`.
    RuleSet { rule_set: usize },
    /// A single rule inside a rule set.
    Rule { rule_set: usize, rule: usize },
    /// The `respond` block of a rule.
    Respond { rule_set: usize, rule: usize },
    /// A middleware file reference, by its index in
    /// `service.middlewares_file_paths`.
    Middleware { middleware: usize },
    /// The fallback respond dir. Singleton — there is one per workspace.
    FallbackRespondDir,
}

#[derive(Default)]
pub(crate) struct IdIndex {
    pub(super) id_to_address: HashMap<NodeId, NodeAddress>,
    pub(super) address_to_id: HashMap<NodeAddress, NodeId>,
}

impl IdIndex {
    /// Insert (or look up) a NodeId for a given address.
    pub(super) fn insert(&mut self, address: NodeAddress) -> NodeId {
        if let Some(&id) = self.address_to_id.get(&address) {
            return id;
        }
        let id = NodeId::new();
        self.id_to_address.insert(id, address);
        self.address_to_id.insert(address, id);
        id
    }

    /// Lookup a NodeAddress by id.
    pub(super) fn lookup(&self, id: NodeId) -> Option<NodeAddress> {
        self.id_to_address.get(&id).copied()
    }

    pub(super) fn id_for(&self, address: NodeAddress) -> Option<NodeId> {
        self.address_to_id.get(&address).copied()
    }
}
