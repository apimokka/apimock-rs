# RFC 016 — Per-condition NodeId addressability

**Status.** Implemented (v5.10.0)
**Tracks.** RFC 004 follow-up — giving individual header and body
conditions stable `NodeId`s so a GUI can issue granular edit commands
(`AddHeaderCondition`, `UpdateHeaderCondition`, `RemoveBodyCondition`)
without replacing the entire condition list via `UpdateRule`.
**Touches.** `apimock-routing` (`view.rs` — `HeaderConditionView`,
`BodyConditionView`), `apimock-config` (`view.rs` — new `EditCommand`
variants, `workspace/edit.rs`, `workspace/id_index.rs`).

## Summary

RFC 002 added `headers: Option<Vec<HeaderConditionPayload>>` and
`body: Option<Vec<BodyConditionPayload>>` to `RulePayload`. A GUI
that wants to add one header condition to an existing rule must
replace the **entire** `headers` list — it cannot say "add this
one condition". This is a latency and conflict-risk problem for live
form editing: the GUI must read the full condition list, append one
entry, and write all of them back.

This RFC assigns a `NodeId` to each condition view so the GUI can
target individual conditions with granular commands.

## Motivation

### Stage-2 "live form editing" UX

The GUI brief §stage-3 mentions "live form editing" where changes
are applied per-keystroke. For header conditions, this means:

- User clicks `+ Add header condition` → `AddHeaderCondition`
- User types in the name field → `UpdateHeaderConditionName { id, name }`
- User deletes a condition row → `RemoveHeaderCondition { id }`

None of these are expressible today without a full `UpdateRule`
round-trip that replaces all conditions.

### Conflict risk in multi-tab editing

If two GUI tabs have the same rule open and Tab A adds a header
condition while Tab B also adds one, a full-list `UpdateRule`
will silently drop Tab A's change when Tab B saves. Per-condition
`NodeId`s allow the conflict-detection layer to spot the race at
the condition granularity.

## Guide-level explanation

`HeaderConditionView` and `BodyConditionView` gain a `NodeId`:

```rust
pub struct HeaderConditionView {
    pub id: NodeId,    // NEW — stable within a Workspace instance
    pub name: String,
    pub op: String,
    pub value: Option<String>,
}

pub struct BodyConditionView {
    pub id: NodeId,    // NEW
    pub kind: String,
    pub path: String,
    pub op: String,
    pub value: serde_json::Value,
}
```

New `EditCommand` variants:

```rust
pub enum EditCommand {
    // … existing variants …

    /// Add a single header condition to an existing rule.
    AddHeaderCondition {
        rule_id: NodeId,
        condition: HeaderConditionPayload,
    },
    /// Update one field of an existing header condition by its NodeId.
    UpdateHeaderCondition {
        id: NodeId,
        condition: HeaderConditionPayload,
    },
    /// Remove a header condition by its NodeId.
    RemoveHeaderCondition {
        id: NodeId,
    },
    /// Add a single body condition.
    AddBodyCondition {
        rule_id: NodeId,
        condition: BodyConditionPayload,
    },
    /// Update one body condition.
    UpdateBodyCondition {
        id: NodeId,
        condition: BodyConditionPayload,
    },
    /// Remove a body condition.
    RemoveBodyCondition {
        id: NodeId,
    },
}
```

## Reference-level explanation

### NodeId assignment

Condition `NodeId`s are assigned at snapshot time, not at load
time. The routing crate does not know about `NodeId`s — they live
in the config crate's `id_index`.

The `id_index` tracks addresses of the form:

```
NodeAddress::HeaderCondition { rule_set_idx, rule_idx, header_name }
NodeAddress::BodyCondition   { rule_set_idx, rule_idx, path }
```

These are stable within a Workspace instance as long as the header
name / body path doesn't change. Renaming a header condition is
modelled as remove + add (same as today's `UpdateRule` semantics).

### snapshot.rs update

`build_when_view` (called from snapshot) currently builds
`HeaderConditionView` without a `NodeId`. After this RFC, it
receives a `NodeIdIndex` ref and mints or looks up an ID for each
condition:

```rust
let id = id_index.id_for(NodeAddress::HeaderCondition {
    rule_set_idx,
    rule_idx,
    header_name: name.clone(),
});
```

### Edit command implementation

`cmd_add_header_condition`:
1. Find the rule by `rule_id` using `id_index`.
2. Clone the rule's `headers` (or create empty `Headers`).
3. Insert the new condition with a fresh `NodeId`.
4. Write back to `self.config`.

`cmd_update_header_condition`:
1. Find the condition by `id`.
2. Replace the `ConditionStatement` in-place.

`cmd_remove_header_condition`:
1. Find the condition by `id`.
2. Remove the entry from `headers.0`.

Body variants follow the same pattern on `body.0`.

### Backwards compatibility

The new `NodeId` fields in `HeaderConditionView` / `BodyConditionView`
are additive (new fields on structs marked `#[non_exhaustive]`).
Existing code that destructures these types will get a compile error
— this is the intended signal to add `id` to the pattern. The
struct update syntax (`..` rest) handles it gracefully.

The new `EditCommand` variants are also additive; existing `match`
arms that handle `EditCommand` must add a `_` catch-all or explicit
arms for the new variants.

### Tests

1. **`snapshot_header_condition_views_have_ids`** — Load a rule
   with two header conditions. Assert each `HeaderConditionView`
   carries a non-nil `NodeId` and the two IDs are distinct.
2. **`ids_stable_across_apply`** — Snapshot, apply an unrelated
   edit (e.g. respond text change), snapshot again. Assert
   `HeaderConditionView` IDs are unchanged.
3. **`add_header_condition_command`** — Call
   `AddHeaderCondition { rule_id, condition }`. Assert the
   resulting rule has one more condition.
4. **`remove_header_condition_command`** — Remove a condition by
   `id`. Assert it is gone; other conditions have unchanged IDs.
5. **`update_body_condition_command`** — Update a body condition's
   operator. Assert value changed; sibling conditions unaffected.

## Drawbacks

1. **`NodeAddress` complexity grows.** Condition addresses add
   two new `NodeAddress` variants with composite keys. The
   `id_index` must handle the extra cases.
2. **Snapshot becomes slightly heavier.** Minting/looking up IDs
   for potentially many conditions on every snapshot adds map
   lookups. At typical rule-set sizes (< 100 rules, < 10 conditions
   each) this is negligible.
3. **Header renaming loses continuity.** Changing a header name
   via `UpdateHeaderCondition` gives the condition a new ID because
   the address key changed. The GUI must refresh its selection set
   after a rename.

## Rationale and alternatives

**Alternative A: keep full-list replace; add per-condition IDs only
to views (read-side).** Easier to implement; the edit side stays
simple. But the GUI must still read-modify-write the full list to
make a single-condition change, which is the root problem.

**Alternative B (this RFC): per-condition IDs + granular edit
commands.** Resolves both the UX problem and the conflict-detection
gap.

## Unresolved questions

1. **ID stability across renames.** Should `UpdateHeaderCondition`
   that changes the `name` field keep the same `NodeId` or assign
   a new one? Arguments for keeping the same: GUI selection stays
   valid. Arguments for a new ID: address-based lookup would be
   stale. Recommendation: keep the same ID; update the
   `NodeAddress` mapping to point to the new name.
2. **`EditCommand` variant naming.** `AddHeaderCondition` vs
   `AddConditionToRule { kind: ConditionKind, … }`. Flat naming is
   more explicit; generic is more extensible. Flat preferred for
   stage-3 (avoids premature abstraction).
3. **`#[non_exhaustive]` migration.** Several existing tests
   destructure `HeaderConditionView` without `..`. Adding the `id`
   field will break them. Decide at implementation time whether to
   update all callers or suppress `#[non_exhaustive]` for these
   view types.
