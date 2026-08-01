# RFC 029 — Finer-grained diff: per-condition change items

**Status.** Implemented (v5.14.0)
**Tracks.** Completion of RFC 016 (per-condition NodeIds). Currently
`compute_diff_summary` emits one `DiffItem` per changed rule; with
RFC 016's per-condition NodeIds available, it can emit finer items
for added/removed header and body conditions, letting the GUI
highlight exactly which condition changed.
**Touches.** `apimock-config` (`workspace/diff.rs`), tests.

## Summary

`DiffKind` gains `HeaderConditionAdded`, `HeaderConditionRemoved`,
`BodyConditionAdded`, `BodyConditionRemoved`. `compute_diff_summary`
emits these when a `save()` changes exactly a condition within a
rule, in addition to (not instead of) the parent `RuleUpdated` item.

## Reference-level explanation

### `DiffKind` additions

```rust
pub enum DiffKind {
    // existing …
    HeaderConditionAdded,
    HeaderConditionRemoved,
    BodyConditionAdded,
    BodyConditionRemoved,
}
```

### `DiffItem`

Unchanged struct — `{ kind: DiffKind, target: NodeId, summary: String }`.

For condition items `target` is the condition's `NodeId` (as returned
by `snapshot()` alongside the condition view).

### `compute_diff_summary` logic

For each rule that has a `RuleUpdated` item:

1. Retrieve the old rule shape from `baseline_files` (re-parsed).
2. Compare header keys: new keys → `HeaderConditionAdded`, missing
   keys → `HeaderConditionRemoved`.
3. Compare body paths: new paths → `BodyConditionAdded`, missing
   paths → `BodyConditionRemoved`.
4. Emit one condition-level item per change (in addition to the
   existing `RuleUpdated` item for the parent rule).

The existing `RuleUpdated` item is always emitted when a rule changes,
regardless of whether condition-level items are also emitted. This
preserves backward compatibility for GUI code that only processes
rule-level diffs.

## Drawbacks

`compute_diff_summary` re-parses the baseline TOML to get the old
rule shape. This is a string-parse on every save, but baselines are
small (a few KB at most) and parse time is negligible.

## Unresolved questions

None.
