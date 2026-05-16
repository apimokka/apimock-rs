# RFC 002 — Structured headers and body conditions in RulePayload

**Status.** Implemented (v5.8.0)
**Tracks.** Stage-2 GUI editing — surfacing the header and
`body.json` condition spaces that exist in the routing crate but are
hidden from the editing payload.
**Touches.** `apimock-config` (`RulePayload`, payload→rule
construction, `toml_writer`, validation, round-trip tests),
`apimock-routing` (stable serialisable view shape for `Headers` and
`Body`).

## Summary

`RulePayload` currently carries `url_path` / `method` / `respond` and
nothing else. Headers and `body.json` conditions can be authored only
by hand-editing the TOML file. The 5.5.0 preservation guarantee keeps
them intact during `UpdateRule`, but the GUI form cannot create them
or modify them. This RFC proposes adding optional `headers` and
`body` fields to `RulePayload` with a payload-level shape that mirrors
the TOML structure while staying independent of the routing crate's
internal types.

## Motivation

The preservation guarantee was a deliberate compromise: it lets the
GUI form re-save a hand-edited rule without dropping conditions it
doesn't know about. That works for the static read-only path but
becomes painful once users want the GUI to *do* anything with
headers or bodies — for example:

- Add a rule that matches `Authorization: Bearer .*` and returns 401
  if the value doesn't start with `Bearer admin-`.
- Match on `Content-Type: application/json` AND a `body.json` value
  predicate, then return a fixture.
- Edit an existing rule's header condition (currently impossible —
  the GUI sees it but can't touch it).

Without payload-level shapes for these conditions, every such task
requires the user to drop into TOML, defeating the GUI's purpose. The
preservation guarantee buys time but is not a long-term answer.

## Guide-level explanation

The GUI rule form gains two new optional condition sections:

```
Headers:
  + Content-Type   [ contains    ▼ ]  [ json                ]
  + X-Tenant-Id    [ starts_with ▼ ]  [ acme-                ]
  + Authorization  [ exists      ▼ ]  (no value field)

Body conditions (JSON):
  + path: action          [ equal       ▼ ]  [ "create"        ]
  + path: items.0.qty     [ greater_than▼ ]  [ 0               ]
```

A rule may declare any number of header conditions and any number of
body conditions. The semantics match the routing crate's existing
behaviour: header conditions are ANDed; body conditions are ANDed;
the rule matches when all conditions match.

A payload that omits `headers` and `body` continues to work as today
(no conditions). A payload that sets either field replaces the
existing rule's conditions wholesale.

## Reference-level explanation

### Payload type additions

```rust
pub struct RulePayload {
    pub url_path: Option<String>,
    pub url_path_op: Option<UrlPathOp>,     // from RFC 001
    pub method: Option<String>,
    pub headers: Option<Vec<HeaderConditionPayload>>,    // NEW
    pub body: Option<Vec<BodyConditionPayload>>,          // NEW
    pub respond: RespondPayload,
}

pub struct HeaderConditionPayload {
    pub name: String,                  // case-insensitive at match time
    pub op: HeaderOp,
    pub value: Option<String>,         // None when op is Exists / Absent
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeaderOp {
    Equal,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    Exists,
    Absent,
}

pub struct BodyConditionPayload {
    pub kind: BodyConditionKind,       // currently only Json; future: Form, Raw
    pub path: String,                  // dotted path, NOT canonical JSONPath
    pub op: BodyOp,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyConditionKind { Json }

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyOp {
    Equal,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    // numeric comparisons left to RFC 008 (body match language extension)
}
```

### Conversion at the payload boundary

`build_rule_from_payload` gains two helpers:

```rust
fn build_headers(input: &[HeaderConditionPayload])
    -> apimock_routing::Headers { /* … */ }

fn build_body(input: &[BodyConditionPayload])
    -> apimock_routing::Body { /* … */ }
```

These hide the routing crate's internal `HashMap` keying from the
config crate, mirroring the approach RFC 001 takes for url_path.

### Interaction with the 5.5.0 preservation guarantee

The current preservation rule:

> If `RulePayload.headers` / `body` are not present in the payload's
> shape (which they currently are not, full stop), preserve the
> existing rule's headers / body unconditionally.

After this RFC the preservation rule must distinguish *omitted* from
*explicitly empty*:

| payload value | semantics                              |
|---------------|----------------------------------------|
| `None`        | preserve existing (today's behaviour) |
| `Some(vec![])`| clear all conditions (user removed them) |
| `Some(vec![…])`| replace with the given set            |

Because `Option<Vec<_>>` distinguishes `None` from `Some(empty)` at
the type level, this is expressible without an additional flag. The
GUI form is responsible for sending `None` when the user hasn't
touched the section and `Some(vec![…])` (possibly empty) when they
have.

### TOML round-trip

`toml_writer` already serialises `Headers` and `Body` correctly after
the 5.5.0 work. The change here is that GUI-authored rules now flow
through `build_rule_from_payload` and hit the same writer. Tests
need only confirm the new path: payload → rule → TOML → load → rule
preserves both content and order.

### Validation

- `HeaderConditionPayload`: `name` must be a valid HTTP header
  token (RFC 7230 §3.2.6 characters). `Exists` / `Absent` require
  `value` to be `None`; other operators require `Some`.
- `BodyConditionPayload`: `path` must satisfy the routing crate's
  dotted-path mini-syntax (see `apimock_routing::util::json`).
  Regex variants must compile.
- These checks run in the existing `validate()` pipeline (see
  `apimock-config::workspace::validate`).

## Drawbacks

1. **`RulePayload` grows substantially.** From 4 fields (after RFC
   001) to 6, with two of them holding `Vec` of structs. Forms that
   currently bind to the simple payload will need significant work.
2. **Routing-crate types leak by shape if not by import.** The
   payload's `HeaderOp` enum effectively mirrors the routing crate's
   internal operator set. Keeping the two in sync is manual labour;
   drift is possible. A small conformance test (each routing operator
   has a matching payload variant) helps but doesn't fully solve it.
3. **Validation surface area grows.** Body path validation has been
   historically loose; this RFC tightens it at the payload layer,
   which may surface latent issues in the routing crate's own
   validation.

## Rationale and alternatives

**Alternative A: expose routing-crate types directly through the
payload.** Smallest diff, but couples config to routing and breaks
the layering used everywhere else.

**Alternative B (this RFC): mirror the shape in payload-owned types.**
Decouples layers; requires drift-prevention discipline.

**Alternative C: a free-form `serde_json::Value` for headers/body in
the payload.** Postpones the typing decision but pushes validation
into a string-typed runtime path. Rejected: stage-2 wants typed forms.

**Alternative D: a "raw TOML escape hatch" field that bypasses the
typed payload for advanced conditions.** Easier for power users but
contradicts the GUI-first direction.

We pick B. A leaks types; C and D defer rather than resolve.

## Prior art

- Postman's mock server uses typed JSON predicates for header and
  body matching, each predicate carrying an explicit operator.
- WireMock's request matchers use a JSON shape that closely
  resembles `HeaderConditionPayload` here.
- Mountebank's `predicates` array allows operator + value structures
  per condition; our `Vec<HeaderConditionPayload>` follows the same
  pattern.

## Unresolved questions

1. **Header case sensitivity at edit time vs match time.** Matching
   normalises to lowercase; should the payload preserve the
   user-entered case for display? Likely yes — the GUI shows what was
   typed; the matcher does the lowercasing internally.
2. **Multi-valued headers (e.g. `Set-Cookie`, `Accept-Language`).**
   The current routing model treats them as a single concatenated
   string. The payload should mirror that for now; a follow-up RFC
   can address proper multi-value handling.
3. **Numeric / type-aware body operators.** Listed as `BodyOp::*`
   above but only string-style operators are included. Numeric
   comparison is deferred to RFC 008.
4. **Maximum payload size.** A rule with 100 header conditions and
   100 body conditions is technically allowed. Reasonable upper
   bounds at validation time?

## Future possibilities

- Combinator conditions (OR across headers, or "match if any of these
  patterns") — needs routing-crate support first.
- Negated operators at payload level (`NotEqual`, `NotContains`).
- A second `BodyConditionKind` for form-encoded bodies, once the
  routing crate grows that capability.
- "Condition templates" (named, reusable predicate sets) — out of
  scope; addressed at a higher GUI layer.
