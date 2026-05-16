# RFC 001 — URL path operator selection in RulePayload

**Status.** Implemented (v5.8.0)
**Tracks.** Stage-2 GUI editing — exposing the URL-path match operator
that the routing crate already supports but the editing payload hides.
**Touches.** `apimock-config` (`RulePayload`, payload→rule construction,
`toml_writer`, validation), `apimock-routing` (no internal changes, but
the operator enum gains a stable serialised name surface).

## Summary

`RulePayload::url_path` is currently a bare `Option<String>` matched
with implicit "equal" semantics. The routing crate supports a richer
set of operators (`equal`, `starts_with`, `contains`, `ends_with`,
`regex`) but they are not addressable through the GUI editing API. This
RFC proposes an additive `url_path_op` field on `RulePayload` so the
GUI can author and edit rules that use the non-`equal` operators
without round-tripping through hand-edited TOML.

## Motivation

The 5.5.0 preservation guarantee already lets `UpdateRule` keep
headers and `body.json` conditions that a hand-edited TOML rule
carries. The same is *not* true for the URL-path operator: as soon as
the GUI form sets `url_path`, the resulting rule's `UrlPath` is
constructed with `Equal`. Operators authored in TOML are lost the
moment a GUI save touches the rule.

For the stage-2 GUI roadmap (richer matcher editing, header/body
condition forms, structured `WhenView`) this is the smallest of the
gaps but the one most likely to surprise users: they author
`starts_with` in TOML, hit "Save" in the GUI after an unrelated
change, and the rule starts matching nothing.

## Guide-level explanation

The GUI rule form gains an operator dropdown next to the path field:

```
URL path:  [ /api/v1/users          ]   match by: [ equal ▼ ]
                                                    starts_with
                                                    contains
                                                    ends_with
                                                    regex
```

When the form submits, the payload carries both fields:

```rust
RulePayload {
    url_path: Some("/api/v1/users".to_string()),
    url_path_op: Some(UrlPathOp::StartsWith),
    method: Some("GET".to_string()),
    respond: RespondPayload { /* ... */ },
}
```

A GUI client that has not been updated for this RFC continues to omit
`url_path_op`. In that case the resulting rule uses `Equal`, matching
today's behaviour exactly.

## Reference-level explanation

### Type additions

In `apimock-config`:

```rust
pub struct RulePayload {
    pub url_path: Option<String>,
    pub url_path_op: Option<UrlPathOp>,   // NEW
    pub method: Option<String>,
    pub respond: RespondPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UrlPathOp {
    Equal,
    StartsWith,
    Contains,
    EndsWith,
    Regex,
}
```

The enum is defined in `apimock-config` (not re-exported from
`apimock-routing`) for the same reason `RulePayload` itself lives in
config: payload types are the GUI-facing contract and should not pull
the entire routing crate's internal type tree into the GUI dependency
closure. Routing's internal `UrlPathOperator` is converted at the
boundary.

### Conversion at the payload boundary

`build_rule_from_payload` (in `apimock-config::workspace::edit`)
converts `url_path_op` to the routing crate's operator type using a
small mapping function in the same module:

```rust
fn to_routing_op(op: UrlPathOp) -> apimock_routing::UrlPathOperator {
    match op { /* one-to-one */ }
}
```

### Interaction with the 5.5.0 preservation guarantee

The guarantee currently states: fields not present in the payload are
preserved from the existing rule. With this RFC the rules become:

- `url_path: None` → URL-path condition (value *and* operator) is
  preserved from existing rule.
- `url_path: Some(_)`, `url_path_op: None` → operator defaults to
  `Equal`. This matches today's behaviour for any client that has
  not been updated.
- `url_path: Some(_)`, `url_path_op: Some(_)` → both replaced.

The "set value without specifying operator" case is the one to
discuss. The alternative — preserve operator if the value is set but
the operator is unset — sounds friendlier but introduces a subtle
"hidden state" bug where the GUI form looks like it owns the field
but actually doesn't. Explicit `Equal` is safer for a typed API.

### TOML round-trip

`toml_writer` already serialises `UrlPath::Op` correctly when starting
from a hand-edited TOML file. The change here is that GUI-authored
rules now flow through the same path. The writer needs no new code if
the routing crate's operator is already `serde`-renamed to snake_case
(which it is, per the 5.5.0 work). A targeted round-trip test
suffices.

### Validation

The Regex variant requires the path value to be a syntactically valid
regular expression. Validation is best-effort at the config layer:
attempt to compile and return a structured error on failure. The
routing crate already performs this compile at rule load time; this
RFC suggests duplicating the check at validation time so the GUI gets
the error before save rather than after.

## Drawbacks

1. **Adds a fifth field to `RulePayload`.** The payload was
   deliberately kept to three fields in stage-1; this is the first
   field added since. Each addition lowers the bar for the next.
2. **Two-level operator surface.** Now `RulePayload` carries an
   operator alongside the value. If headers/body conditions get their
   own operators (RFC 002) the pattern repeats. A future refactor to
   "condition objects" (`UrlPathCondition { value, op }`) may be
   warranted; that refactor would be a breaking change.
3. **Regex authoring is error-prone.** Surfacing it via the GUI
   without good editor support is risky. See unresolved questions.

## Rationale and alternatives

**Alternative A: replace `url_path: Option<String>` with
`url_path: Option<UrlPathCondition>` where `UrlPathCondition = {
value, op }`.** Cleaner, but a breaking change for every existing
caller (CLI tools, tests, GUI prototypes). The cost of breakage
outweighs the cleanliness gain at stage-2.

**Alternative B (this RFC): additive `url_path_op` field.** Backwards
compatible, smallest diff, can be revisited later if the pattern
proliferates.

**Alternative C: reuse the routing crate's `UrlPathOperator` enum
directly.** Couples the GUI-facing payload type to a routing-internal
type. Violates the deliberate separation: config types are the public
contract, routing types are implementation.

We pick B. C is rejected on layering grounds. A is rejected on
backward-compat grounds for now, but may become attractive in a
hypothetical 6.0.0 if condition objects accumulate.

## Prior art

- Express.js `path-to-regexp` supports prefix and regex matching
  with explicit operator semantics.
- Mock Service Worker (MSW) takes matcher functions that wrap
  the operator choice in code rather than data.
- Mountebank stub predicates name the operator explicitly
  (`equals`, `startsWith`, `contains`, `matches`).

The Mountebank shape (named operator + value) is closest to what this
RFC proposes; we follow that convention.

## Unresolved questions

1. **Should `Regex` be exposed in stage-2?** Authoring regex in a form
   field without editor support produces footguns. Options: (a) hide
   `Regex` from the stage-2 dropdown and surface it in stage-3 with
   a richer regex authoring component; (b) expose it but require an
   explicit "advanced" toggle.
2. **Case sensitivity.** The routing crate treats matching as
   case-sensitive. If a GUI option for case-insensitive matching is
   added, it crosses the boundary: it's a *modifier* on the operator
   rather than a new operator. The cleanest path is probably a
   separate `url_path_case_insensitive: Option<bool>` flag, deferred
   to a follow-up RFC.
3. **Empty / None value with operator set.** What does
   `url_path: None`, `url_path_op: Some(Contains)` mean? Probably an
   error at validation time. To be confirmed.

## Future possibilities

- Negated operators (`NotEqual`, `NotStartsWith`) — useful for
  carve-out rules. Currently not supported by the routing crate; would
  need both routing and payload work.
- Combinator operators (AND/OR over multiple URL-path conditions) —
  not currently in scope; the existing single-condition shape is
  enough for stage-2.
- Replacing `url_path` + `url_path_op` with a `UrlPathCondition`
  struct in a future major version. This RFC's additive shape keeps
  that door open.
