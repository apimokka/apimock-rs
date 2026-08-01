# RFC 027 — Rule priority field surfaced in view, payload, and TOML writer

**Status.** Implemented (v5.14.0)
**Tracks.** Priority strategy gap — `Rule.priority: Option<i32>` exists
in the routing crate and is read by the `Priority` strategy, but it
is not exposed in `RuleView`, `RulePayload`, or `toml_writer`. GUI
cannot set priorities and freshly saved files lose them.
**Touches.** `apimock-routing` (`view.rs`, `view/build.rs`),
`apimock-config` (`view.rs` `RulePayload`, `workspace/edit/payload.rs`,
`toml_writer.rs`), documentation.

## Summary

Adds `priority: Option<i32>` to `RuleView` and `RulePayload`, wires
it through `build_rule_view` and `build_rule_from_payload`, and
teaches `toml_writer` to emit it. After this RFC a GUI can read and
set per-rule priorities, and the round-trip through save → load
preserves them.

## Reference-level explanation

### `RuleView` (routing crate)

```rust
pub struct RuleView {
    pub index: usize,
    pub priority: Option<i32>,   // NEW
    pub when: WhenView,
    pub respond: RespondView,
}
```

`build_rule_view` populates it from `rule.priority`.

### `RulePayload` (config crate)

```rust
pub struct RulePayload {
    pub url_path: Option<String>,
    pub url_path_op: Option<UrlPathOp>,
    pub method: Option<String>,
    pub priority: Option<i32>,   // NEW
    pub headers: Option<Vec<HeaderConditionPayload>>,
    pub body: Option<Vec<BodyConditionPayload>>,
    pub respond: RespondPayload,
}
```

`build_rule_from_payload` sets `rule.priority = payload.priority`.

### `toml_writer`

Emits `priority = <n>` in the rule table when `rule.priority.is_some()`.

## Drawbacks

`i32` is unbounded. Validation (cap to a sane range like `−1000..=1000`)
is out of scope here; callers supply what they store.

## Unresolved questions

None.
