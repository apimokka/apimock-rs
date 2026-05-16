# Workspace

## Workspace layout (5.0.0+)

Starting with 5.0.0, apimock is organised as a Cargo workspace:

| crate | responsibility |
| --- | --- |
| [`apimock-config`](crates/apimock-config) | TOML config model — load, validate, resolve relative paths. Defines the stage-1 edit / snapshot API types a GUI can depend on. |
| [`apimock-routing`](crates/apimock-routing) | Rule-set definitions, request matching, read-only view types for GUI tooling. |
| [`apimock-server`](crates/apimock-server) | HTTP(S) listener, request dispatch, Rhai middleware compilation, response building. |
| `apimock` (workspace root) | CLI binary + thin façade library. Re-exports the three member crates under `apimock::config`, `apimock::routing`, `apimock::server`. |

End users installing via `cargo install apimock` or `npx apimock` see no difference from 4.8.0. Library consumers migrating from 4.x paths can find the mapping in [CHANGELOG.md](https://github.com/apimokka/apimock-rs/blob/main/CHANGELOG.md).

## GUI-facing Workspace API (5.1.0+)

5.1.0 adds `apimock_config::Workspace` for tooling that wants to edit a configuration through structured commands instead of TOML text. A GUI typically holds a single `Workspace` value for one editing session, calls `snapshot()` to produce a render-ready view, and `apply(EditCommand)` to mutate. Every editable node carries a stable `NodeId` (a v4 UUID) that survives reorderings, so a selection set anchored on NodeIds remains valid across edits.

```rust
use apimock_config::{
    Workspace, EditCommand, RespondPayload, RulePayload,
    NodeId, ConfigFileKind, NodeKind,
};

let mut ws = Workspace::load("apimock.toml".into())?;

// Find the rule-set node from the snapshot.
let snap = ws.snapshot();
let rule_set_id: NodeId = snap.files.iter()
    .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
    .and_then(|f| f.nodes.iter().find(|n| matches!(n.kind, NodeKind::RuleSet)))
    .map(|n| n.id)
    .expect("rule set");

// Add a new rule.
let result = ws.apply(EditCommand::AddRule {
    parent: rule_set_id,
    rule: RulePayload {
        url_path: Some("/api/new".into()),
        method: Some("GET".into()),
        respond: RespondPayload {
            text: Some("hello".into()),
            ..Default::default()
        },
    },
})?;

if !result.diagnostics.is_empty() {
    // surface each per-node diagnostic in the GUI
}
```

Saving back to disk (`Workspace::save`) is reserved for 5.2.0; 5.1.0 covers Steps 1–3 of the GUI extension plan (snapshot + apply + validate).
