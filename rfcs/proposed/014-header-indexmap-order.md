# RFC 014 — Header order preservation via IndexMap

**Status.** Proposed
**Tracks.** RFC 004 follow-up — replacing the `HashMap` backing of
`Headers` with `IndexMap` so header conditions are stored and
displayed in TOML-insertion order, making `WhenView.headers`
deterministic without a sort pass.
**Touches.** `apimock-routing` (`Cargo.toml`, `headers.rs`,
`view/build.rs`), `apimock-config` (`toml_writer.rs` — minor).

## Summary

`Headers(HashMap<String, ConditionStatement>)` is unordered. The
view builder works around this by sorting `HeaderConditionView`
entries alphabetically before returning them from `build_when_view`.
Alphabetical order is a reasonable deterministic fallback, but it
differs from the order the rule author wrote in TOML — which is
the order they expect to see in the GUI.

Switching to `indexmap::IndexMap` preserves insertion order
(TOML-parse order) at negligible runtime cost and eliminates the
sort workaround.

## Motivation

Consider this rule file:

```toml
[when.request.headers]
"Authorization"   = { op = "starts_with", value = "Bearer " }
"X-Tenant-Id"     = { op = "equal", value = "acme" }
"Content-Type"    = { op = "contains", value = "json" }
```

The author placed `Authorization` first because it is the most
important condition. With `HashMap`, the GUI may display them as:

```
Authorization  starts_with  Bearer
Content-Type   contains     json
X-Tenant-Id    equal        acme
```

(alphabetical — not what was written). With `IndexMap`, the GUI
displays them in the authored order:

```
Authorization  starts_with  Bearer
X-Tenant-Id    equal        acme
Content-Type   contains     json
```

This is a quality-of-life improvement, not a correctness fix —
`AND` semantics are unchanged regardless of order — but it makes
the GUI feel responsive to the user's intent.

## Guide-level explanation

No user-visible TOML change. The improvement is visible only in
GUI rendering: `WhenView.headers` now arrives in the order the
conditions were written in the rule file.

## Reference-level explanation

### Dependency

Add `indexmap` to `apimock-routing/Cargo.toml`:

```toml
[dependencies]
indexmap = { version = "2", features = ["serde"] }
```

`indexmap` 2.x is the current stable version and requires no
additional workspace-level pinning unless other crates already
pin it to an incompatible version.

### Headers type change

```rust
// Before
use std::collections::HashMap;
pub struct Headers(pub HashMap<String, ConditionStatement>);

// After
use indexmap::IndexMap;
pub struct Headers(pub IndexMap<String, ConditionStatement>);
```

Because `IndexMap` implements `serde::Deserialize` with the
`serde` feature enabled (which we set above), the TOML
deserialisation path is unchanged — `#[serde(transparent)]`
continues to work.

### view/build.rs: remove sort workaround

The `build_header_condition_views` function currently sorts by
header name:

```rust
// Remove this after this RFC:
views.sort_by(|a, b| a.name.cmp(&b.name));
```

After switching to `IndexMap`, insertion order is preserved
through iteration, so the sort is no longer needed. The
`HeaderConditionView` list reflects TOML order naturally.

### toml_writer.rs: no change needed

`toml_writer` iterates `headers.0` to build the TOML table. With
`HashMap`, the iteration order was random; with `IndexMap`, it
matches insertion order. This is a strict improvement — the
round-tripped TOML now preserves the original key order.

### Tests

1. **`headers_preserves_toml_insertion_order`** — Parse a `Headers`
   from TOML with keys `Z`, `A`, `M`. Assert the deserialized map
   iterates in `Z`, `A`, `M` order (not alphabetical).
2. **`when_view_headers_in_insertion_order`** — Build a rule with
   headers in a specific order. Call `build_when_view`. Assert
   `when.headers` slice is in the original order.
3. **`toml_round_trip_preserves_header_order`** — Write a rule
   with three headers, serialize to TOML, parse back. Assert key
   order survived the round trip.

## Drawbacks

1. **New dependency.** `indexmap` adds ~50 KB to the compiled
   routing crate. It is a mature, widely used crate in the Rust
   ecosystem; the risk is low.
2. **Minor API surface change.** Any code that calls
   `headers.0.iter()` and relies on HashMap iteration semantics
   (e.g. non-deterministic ordering in tests) must be updated to
   expect a stable order. In practice this means fixing any tests
   that sorted the output before asserting.
3. **Case-normalisation interplay.** Header keys are lowercased
   at rule-construction time (`payload.rs` does `to_lowercase()`).
   With `IndexMap`, the stored key is the lowercased form, which
   differs from what the author wrote. The view builder must emit
   the stored (lowercased) key. This is unchanged from today's
   `HashMap` behaviour.

## Rationale and alternatives

**Alternative A: sort by header name (current workaround).** Works
but overrides the author's intent. Rejected as the permanent
solution.

**Alternative B: `Vec<(String, ConditionStatement)>` instead of a
map.** Preserves order and allows duplicate keys. But `is_match`
relies on `HashMap::get` for O(1) lookup by name. `Vec` would be
O(n) per key lookup — fine for 5 headers, slow for 50+.

**Alternative C (this RFC): `IndexMap`.** O(1) lookup + O(n)
ordered iteration. The standard choice when both properties are
needed.

## Unresolved questions

1. **`indexmap` version in workspace `[workspace.dependencies]`.**
   If another crate in the workspace already depends on `indexmap`
   (directly or transitively), the version should be coordinated.
   Check `cargo tree` at implementation time.
2. **Case-normalisation in `build_rule_from_payload`.** The config
   crate lowercases header names before inserting into `Headers`.
   The TOML-sourced path (direct deserialization) does not
   lowercase — it stores keys as written. This inconsistency
   predates this RFC and should be audited separately.
