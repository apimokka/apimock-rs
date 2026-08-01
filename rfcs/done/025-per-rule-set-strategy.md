# RFC 025 — Per-rule-set strategy override

**Status.** Implemented (v5.13.0)
**Tracks.** RFC 007 Future possibilities — "per-rule-set strategy
overrides." Currently `Strategy` is a single service-level property;
all rule sets use the same selection algorithm.
**Touches.** `apimock-routing` (`rule_set.rs`, `view.rs`),
`apimock-config` (`EditCommand`, `view.rs`, workspace apply handler,
`toml_writer`), documentation.

## Summary

`RuleSet` gains an optional `strategy` field. When set, it overrides
the service-level strategy for matching within that rule set. When
absent, the service-level strategy is used (existing behaviour).

## Reference-level explanation

### `RuleSet` (routing crate)

```rust
pub struct RuleSet {
    pub strategy: Option<Strategy>,   // NEW — per-rule-set override
    // … existing fields
}
```

`find_matched` already receives `strategy: Option<&Strategy>` from
the server. The call site changes to:

```rust
rule_set.find_matched(
    parsed_request,
    rule_set.strategy.as_ref().or(service_strategy),
    rule_set_idx,
)
```

### `RuleSetView` (routing crate)

```rust
pub struct RuleSetView {
    pub strategy: Option<String>,    // NEW — None = inherited
    // … existing fields
}
```

### TOML (routing crate)

```toml
[prefix]
url_path_prefix = "/api/v1"
strategy = "round_robin"    # NEW — optional override
```

The `strategy` field on `[prefix]` is re-used — it was originally
reserved for this purpose in the routing crate's `Prefix` struct.

### Edit API (config crate)

New `EditCommand`:

```rust
EditCommand::UpdateRuleSetStrategy {
    id: NodeId,                    // rule-set node
    strategy: Option<String>,      // None → inherit; Some("…") → override
}
```

`ReloadHint`: `SoftReload` — strategy change takes effect when rule
matching runs next.

### Validation

`strategy` string must be one of the known strategy names
(`first_match`, `uniform_random`, `weighted_random`, `priority`,
`round_robin`) or `null`/absent (inherit). Unknown strings return
`ApplyError::InvalidPayload`.

## Unresolved questions

None.
