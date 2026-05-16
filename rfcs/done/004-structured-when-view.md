# RFC 004 — Structured WhenView for headers and body conditions

**Status.** Implemented (v5.8.0)
**Tracks.** Stage-2 GUI rendering — replacing the boolean
`has_header_conditions` / `has_body_conditions` flags on `WhenView`
with structured detail the GUI can render without a second query.
**Touches.** `apimock-routing` (`WhenView` struct, view builder),
`apimock-config` (snapshot builder, GUI-facing re-exports),
documentation.

## Summary

`WhenView` today carries `url_path`, `method`, and two boolean flags
(`has_header_conditions`, `has_body_conditions`). The GUI uses these
to badge a rule as "has more conditions" but cannot show those
conditions without re-loading the routing types. This RFC proposes
extending `WhenView` to carry structured detail for both header and
body conditions, while keeping the view shape stable and serialisable
so it can cross a process boundary if needed.

## Motivation

The 5.3.0 snapshot work introduced `WhenView` as a deliberate stage-1
shape: the GUI needs *something* about request matching, but the GUI
form at stage-1 only edits `url_path` and `method`. Booleans were
enough to render an "advanced conditions present" badge.

At stage-2, with RFC 002 making headers and body conditions
GUI-editable, the GUI needs to *display* the structured conditions
in a list before the user clicks "edit". The current shape forces the
GUI to either:

- query a separate endpoint for the rule's conditions, doubling the
  number of round-trips per render, or
- reach into the routing crate's own types from the GUI process —
  exactly the coupling the layering was designed to prevent.

A structured `WhenView` resolves both.

## Guide-level explanation

The GUI's rule list shows summary text instead of a flag:

```
GET /api/users
  └─ header: Content-Type contains "json"
  └─ header: X-Tenant-Id starts with "acme-"
  └─ body: action == "create"
```

This information is in the `RouteCatalogSnapshot` directly. No
second query.

## Reference-level explanation

### New shape

```rust
pub struct WhenView {
    pub url_path: Option<UrlPathConditionView>,
    pub method: Option<String>,
    pub headers: Vec<HeaderConditionView>,    // was: has_header_conditions: bool
    pub body: Vec<BodyConditionView>,          // was: has_body_conditions: bool
}

pub struct UrlPathConditionView {
    pub value: String,
    pub op: String,        // serialised operator name (e.g. "equal", "starts_with")
}

pub struct HeaderConditionView {
    pub name: String,
    pub op: String,
    pub value: Option<String>,    // None when op is "exists" / "absent"
}

pub struct BodyConditionView {
    pub kind: String,              // currently always "json"
    pub path: String,
    pub op: String,
    pub value: serde_json::Value,
}
```

Notes:

- `url_path` becomes a struct rather than a bare string. This carries
  the operator from RFC 001 so the GUI can render it without
  guessing. Backwards-compat note: GUIs that read `url_path` as a
  string break; the migration is small.
- Operator fields are `String` (the serialised name), not the
  routing crate's enum. This keeps the view crate-independent and
  serialisable across process boundaries without dragging type
  definitions along.
- Vec is empty when no conditions are present; `is_empty()` replaces
  the old boolean check.

### View builder updates

The 5.3.0 view builder (`apimock-routing::view::build`) currently
walks the rule's `When` and writes flags. The new builder walks the
same structure and emits `HeaderConditionView` / `BodyConditionView`
entries, preserving order. No new traversal cost — the same fields
are already read.

### Snapshot stability

`RouteCatalogSnapshot` already declares "snapshot is not a live
reference; the GUI must call `snapshot()` again after apply". This
RFC does not change that property — the structured fields are still
captured at snapshot time and become stale after edits.

### NodeId implications

None. `WhenView` is part of `RuleView`, which carries a `NodeId`.
The new fields are descriptive, not addressable on their own. If a
future RFC wants to address individual conditions by NodeId
(`UpdateHeaderCondition { id, … }`), that's a separate decision; this
RFC keeps the per-rule addressing model.

### Size and serialisation

For a rule with 10 header and 10 body conditions, `WhenView` grows
from ~30 bytes (flags + path string) to ~1–2 KB (structured detail).
For a snapshot of 100 rules with such density, the snapshot grows
from ~3 KB to ~150 KB. This is well within reasonable bounds for IPC
or in-process traversal; full benchmarks should confirm.

## Drawbacks

1. **Breaking shape change.** Code that reads `has_header_conditions`
   stops compiling. The migration is short, but every GUI prototype
   and test fixture needs updating.
2. **Snapshot size grows proportionally to condition density.** For
   rule sets with many conditions per rule, the snapshot inflates.
   For typical mock-server rule sets this is unlikely to matter but
   pathological cases exist.
3. **String-typed operator fields.** Type-safety lost compared to
   exposing the routing crate's enum directly. Mitigated by treating
   the strings as a stable serialisation contract documented
   alongside the view module.

## Rationale and alternatives

**Alternative A: keep flags, add a separate `WhenDetailView` accessor.**
The snapshot stays small; the GUI fetches details on demand for the
rules it currently shows. Two round-trips per render but better
worst-case memory.

**Alternative B (this RFC): expand `WhenView` to carry structured
detail.** One round-trip; larger snapshots. Matches the snapshot
philosophy of "everything the GUI needs to render in one pass".

**Alternative C: expose the routing crate's `When` type directly
through the view.** Smallest code change, biggest coupling. Rejected
on layering grounds (same as RFC 002).

**Alternative D: hybrid — flags + lazy struct.** A `Lazy<Vec<…>>`
field that's only populated when accessed. Adds complexity to the
snapshot for a benefit that probably doesn't manifest at typical
scales. Rejected as premature.

We pick B. A is worth revisiting if profiling shows snapshot size
becoming a problem in practice; the shape is forward-compatible with
moving to A.

## Prior art

- The Postman mock collection API returns full match conditions
  inline with each rule's metadata.
- Mountebank's `imposters` endpoint returns the full predicate tree
  per stub.
- Both have the "fat snapshot" shape proposed here. None of them
  appear to have switched to a lazy / paged shape at production
  scale, which is mild evidence the cost is acceptable.

## Unresolved questions

1. **Order preservation.** The routing crate's `Headers` is currently
   `HashMap`-backed (unordered). For display, the view should sort
   stably (e.g. by header name) so GUIs render deterministically.
   This RFC recommends alphabetical-by-name for headers and by-path
   for body conditions, but the choice is open.
2. **Should operator strings be wrapped in a typed enum at the view
   crate?** A `WhenOperator` enum re-exported from the view module
   would give type safety without coupling to routing internals.
   Marginal benefit; deferrable.
3. **Snapshot diff implications.** With structured conditions in the
   snapshot, per-rule diff (5.3.0 work) becomes finer-grained — the
   diff can highlight added/removed conditions. Out of scope here
   but a natural follow-up.

## Future possibilities

- A typed `WhenOperator` enum on the view side.
- Per-condition addressability via NodeId, enabling
  `UpdateHeaderCondition` / `RemoveBodyCondition` granular edit
  commands. Useful for "live form editing" UX in stage-3.
- A lazy / paged variant of `WhenView` if snapshot size becomes a
  bottleneck.
- Order-preserving header storage in routing crate, propagated into
  the view. Currently headers use a `HashMap`; switching to an
  insertion-ordered map (e.g. `IndexMap`) makes the view's order
  reflect TOML order naturally.
