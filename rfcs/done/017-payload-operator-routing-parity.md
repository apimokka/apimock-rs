# RFC 017 — Payload operator routing parity

**Status.** Implemented (v5.11.0)
**Tracks.** Closing the silent operator-collapse in `RulePayload`'s
header / url_path operator surface: five payload operators that the
type system claims to support do not work as advertised at routing
time because the payload→routing converter silently maps them to
unrelated routing operators.
**Touches.** `apimock-routing` (new `HeaderOperator` enum, additions
to `RuleOp`, updated `Headers::is_match`), `apimock-config`
(`workspace/edit/payload.rs` converter, `view.rs` doc-comment fix,
test coverage for round-tripped operators).

## Summary

Five payload-layer operators currently collapse to unrelated
routing operators at the apply boundary:

| Payload variant         | Today's mapping (broken) | Effective semantics            |
|-------------------------|--------------------------|--------------------------------|
| `UrlPathOp::EndsWith`   | `RuleOp::Contains`       | matches anywhere, not suffix   |
| `HeaderOp::EndsWith`    | `RuleOp::Contains`       | same                           |
| `HeaderOp::Regex`       | `RuleOp::Equal`          | regex never applied            |
| `HeaderOp::Exists`      | `RuleOp::Equal` (empty)  | matches only header with `""` value |
| `HeaderOp::Absent`      | `RuleOp::Equal` (empty)  | matches only header with `""` value, not absent |

The collapse is silent — `cargo test` passes; `cargo check` passes;
the GUI form accepts the operator; the rule serialises to TOML;
the round-trip preserves the operator name. Only at request-match
time does the rule behave wrongly, and the wrong behaviour is
plausible enough (a substring rule may still match many of the
requests an `ends_with` rule would have matched) that the gap can
go undetected for a long time.

This RFC adds the missing routing operators so each payload variant
gets honest routing semantics.

A small adjacent cleanup is included: `UrlPathOp::NotEqual` carries
a `/// Regular expression match.` doc comment (a leftover from a
pre-5.8.0 refactor where this slot was originally `Regex`). The
comment is fixed to match the variant name.

To close the asymmetry between header and url_path operator
surfaces (headers can match by `Regex`; url_path could not),
`UrlPathOp::Regex` is added to the payload at the same time. This
resolves RFC 001 Unresolved §1 ("Should Regex be exposed in
stage-2?") in the affirmative — the surfaces become symmetric, the
GUI form gets one more variant in the dropdown, and the routing
crate's new `RuleOp::Regex` powers both.

## Motivation

The discovery context: stage-2 GUI work surfaced the question "what
does the user see when they pick `Header → Exists` in the form and
save the rule?" The answer turned out to be "the rule matches no
real header in practice, because Exists is collapsing to
`Equal """`. The form lies about its own contract.

Closing the gap matters more than the gap's narrowness suggests:

1. **The payload is the public contract.** Any operator the
   payload accepts is a promise. Silent collapse breaks that promise
   in a way users can't see without runtime inspection.
2. **GUI work amplifies the cost.** Stage-1 users hand-edited TOML,
   so they had `RuleOp` available and never picked the unrepresentable
   operators. Stage-2 users have only `RulePayload`'s vocabulary, so
   they have no warning the operator they picked has degraded.
3. **Cost is genuinely small.** Two new `RuleOp` variants plus a
   header-specific operator enum. No layout reshuffles.

## Guide-level explanation

Every `UrlPathOp` and `HeaderOp` variant in `RulePayload` now
behaves exactly as its name suggests at request-match time. No
silent fallbacks. A rule authored as
`HeaderOp::Exists` matches any request that carries the header,
regardless of value; `HeaderOp::Absent` matches any request that
does not carry it.

Behaviour for `UrlPathOp::Equal`, `StartsWith`, `Contains`,
`WildCard`, `NotEqual` is unchanged. Behaviour for
`HeaderOp::Equal`, `Contains`, `StartsWith`, `NotEqual`,
`WildCard` is unchanged. The four already-working operators in
each surface continue to work identically.

## Reference-level explanation

### Routing-crate additions

```rust
// apimock_routing::rule_set::rule::when::request::rule_op
pub enum RuleOp {
    Equal,
    NotEqual,
    StartsWith,
    Contains,
    EndsWith,     // NEW (RFC 017)
    Regex,        // NEW (RFC 017)
    WildCard,
}
```

`RuleOp::is_match` gains two arms:

```rust
Self::EndsWith => text.ends_with(checker),
Self::Regex    => regex::Regex::new(checker)
                     .map(|re| re.is_match(text))
                     .unwrap_or(false),
```

The `regex` crate is already a transitive dependency through
`globset`; the addition is no net build cost.

Regex compilation per match call is acceptable for stage-2 scale
(rules are evaluated linearly; rule counts are small).
A compiled-regex cache is a future optimisation, not in scope.

### Header operator surface

`HeaderOp::Exists` and `HeaderOp::Absent` are conceptually distinct
from value-comparison operators: they assert key presence and ignore
any value. Encoding them as `RuleOp` variants would force every
`RuleOp` site (including url_path matching, where presence makes no
sense) to handle the operator. Instead, a header-specific operator
enum is introduced in the routing crate, mirroring the existing
`BodyOperator` pattern:

```rust
// apimock_routing::rule_set::rule::when::request::headers
#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeaderOperator {
    // value-comparison
    Equal,
    NotEqual,
    StartsWith,
    EndsWith,
    Contains,
    WildCard,
    Regex,
    // presence-comparison
    Exists,
    Absent,
}
```

A flat enum (not a `Value(RuleOp) | Exists | Absent` wrapper) is
chosen deliberately for parity with `BodyOperator`'s shape: each
operator surface owns its own enumeration, serialises directly to
TOML via serde's `rename_all = "snake_case"`, and reads cleanly in
match arms without unwrapping. The two operators that *also* exist
in `RuleOp` (Equal, NotEqual, …) are not identical concepts —
header matching is case-insensitive on the key and operates on a
normalised value string, while url_path matching is case-sensitive
and operates on the raw path. The duplication of variant names is
incidental, not architectural.

`Headers` stores a new `HeaderConditionStatement { op: Option<HeaderOperator>, value: String }`
instead of the shared `ConditionStatement` (`Body` already has its
own `BodyConditionStatement` from RFC 008). After this RFC the
`ConditionStatement` type is no longer used by `Headers`; if it has
no remaining users it is deleted as dead code in the same change.
`Headers::is_match` branches:

```rust
match op {
    HeaderOperator::Exists => request_headers.contains_key(name),
    HeaderOperator::Absent => !request_headers.contains_key(name),
    other => match request_headers.get(name) {
        None => false,
        Some(v) => {
            let value_op: RuleOp = other.to_rule_op();   // infallible — all non-presence variants map
            value_op.is_match(v.to_str().ok()?, value)
        }
    },
}
```

The `to_rule_op` helper is a private converter on `HeaderOperator`
that maps the 7 value-comparison variants to `RuleOp`. Presence
variants are filtered out by the match arm above, so the converter
is total within its calling context.

### Payload-side `UrlPathOp::Regex` addition

```rust
// apimock_config::view
pub enum UrlPathOp {
    Equal,
    StartsWith,
    Contains,
    EndsWith,
    WildCard,
    NotEqual,
    Regex,            // NEW (RFC 017) — symmetric with HeaderOp::Regex
}
```

The payload converter and validation pick up the new variant
mechanically. The TOML wire form is `op = "regex"`. Validation
attempts a compile via `regex::Regex::new` and surfaces a
diagnostic on parse failure (same machinery as for
`HeaderOp::Regex`).

### Payload converter cleanup

`apimock_config::workspace::edit::payload`:

```rust
fn url_path_op_to_routing(op: UrlPathOp) -> RuleOp {
    match op {
        UrlPathOp::Equal      => RuleOp::Equal,
        UrlPathOp::StartsWith => RuleOp::StartsWith,
        UrlPathOp::Contains   => RuleOp::Contains,
        UrlPathOp::EndsWith   => RuleOp::EndsWith,   // no fallback
        UrlPathOp::WildCard   => RuleOp::WildCard,
        UrlPathOp::NotEqual   => RuleOp::NotEqual,
        UrlPathOp::Regex      => RuleOp::Regex,       // NEW
    }
}

fn header_op_to_routing(op: HeaderOp) -> HeaderOperator {
    match op {
        HeaderOp::Equal      => HeaderOperator::Equal,
        HeaderOp::NotEqual   => HeaderOperator::NotEqual,
        HeaderOp::StartsWith => HeaderOperator::StartsWith,
        HeaderOp::EndsWith   => HeaderOperator::EndsWith,
        HeaderOp::Contains   => HeaderOperator::Contains,
        HeaderOp::WildCard   => HeaderOperator::WildCard,
        HeaderOp::Regex      => HeaderOperator::Regex,
        HeaderOp::Exists     => HeaderOperator::Exists,
        HeaderOp::Absent     => HeaderOperator::Absent,
    }
}
```

Every payload variant has a non-degenerate routing equivalent. Both
matches are exhaustive in both directions and the conversion is
purely structural — no fallback, no information loss.

### Doc-comment correction

`apimock_config::view::UrlPathOp::NotEqual` currently carries the
comment `/// Regular expression match.` from a pre-5.8.0 draft where
the slot was `Regex`. The comment becomes
`/// Negated equality match.` to match the variant name.

### TOML serialisation

The routing crate's `RuleOp` is serde-renamed as `snake_case`. The
two new variants serialise to `"ends_with"` and `"regex"`. These
strings are already what `HeaderOp::EndsWith` / `Regex` serialise to
in the payload layer, so TOML round-tripping needs no changes
beyond exposing the new operator names in any docs / example TOML
that enumerate the operator vocabulary.

`HeaderOperator` (flat enum, `serde(rename_all = "snake_case")`)
serialises each variant directly to its snake-case name (`"equal"`,
`"exists"`, etc.), matching `BodyOperator`'s wire shape. No
wrapper, no special tagging.

### Validation

A new converter assertion is added: every `HeaderOp` and `UrlPathOp`
variant must map to a non-fallback routing operator. Enforced by
test rather than at compile time (Rust can't statically assert
"every variant must produce a distinct mapping"), but the test
ensures that future variant additions won't silently regress to a
fallback.

### Migration

No user-visible TOML changes. Header conditions previously written
as `op = "ends_with"` were already accepted at parse time but
silently behaved as `contains`. After this RFC, they behave as
ends_with. **This is technically a behaviour change for any rule
that depended on the old wrong behaviour.** The release notes flag
this explicitly; the assumption is no real rule relied on the
miscoded semantics.

## Drawbacks

1. **Behavioural change for `ends_with` / `regex` / `exists` /
   `absent` rules in the wild.** Anyone who wrote one of these
   operators and inadvertently relied on the fallback semantics
   sees a behaviour change. Mitigated by the fact that the
   fallback was almost certainly broken from the user's perspective
   too; this RFC fixes the breakage. A clear CHANGELOG note is
   essential.
2. **Adds two new `RuleOp` variants.** Touches all `RuleOp`
   exhaustive-match sites (display, format_condition, payload
   converter, toml_writer). Mechanical work.
3. **`HeaderOperator` is a new type.** Code reading `Headers` now
   sees two operator types (`RuleOp` for url_path, `HeaderOperator`
   for headers, `BodyOperator` for bodies). Three operator surfaces
   in one crate is borderline; the alternative (a single union enum)
   was rejected in RFC 008's design discussion for `BodyOperator`
   and the same reasoning applies here.

## Rationale and alternatives

**Alternative A: Extend `RuleOp` with `Exists` / `Absent` and have
the matcher special-case them.** Simpler — one enum, three new
variants. Cost: `RuleOp::is_match` would have to accept "no value
available" cases (url_path always has a value; headers don't), and
every site that uses `RuleOp` would need to know not to call it
with Exists/Absent on url_path. Type-level confusion.

**Alternative B (this RFC): Header-specific `HeaderOperator`
wrapping `RuleOp` plus presence variants.** Matches the existing
`BodyOperator` precedent. Each operator surface is right-sized.

**Alternative C: Remove the unreachable variants from
`HeaderOp` / `UrlPathOp`.** Smallest implementation. Loses the
operators that the original RFC 002 intentionally exposed. Goes
backward from what the GUI form already shows.

**Alternative D: Keep the fallback but emit a runtime warning.**
The warning would fire on every match attempt — log spam — and the
operator still behaves wrongly. Cosmetic at best.

We pick B. A loses type clarity; C loses functionality; D doesn't
actually solve anything.

## Prior art

- The `regex` crate is widely used in Rust matching libraries
  (ripgrep, fd) where Regex sits alongside literal-string operators
  in a single enum. The cost-of-Regex-per-match concern is well
  understood; cache when it matters, ship without when it doesn't.
- `BodyOperator` (RFC 008) sets the precedent of a domain-specific
  operator enum that wraps general-purpose operators plus
  domain-specific extensions. `HeaderOperator` here follows the
  same pattern.

## Unresolved questions

1. **`HeaderOperator` TOML wire format.** ✅ **Resolved.** Flat enum
   with `serde(rename_all = "snake_case")` — matches `BodyOperator`'s
   precedent exactly. No wrapper, no tagging.
2. **Regex compilation cache.** Per-match regex compilation is
   ergonomically fine but wasteful. A simple `HashMap<String, Regex>`
   in `RuleOp` would help; not in scope here, but worth flagging as
   a future micro-optimisation.
3. **`Regex` for `UrlPathOp`.** ✅ **Resolved.** Added to the payload
   in this RFC for symmetry with `HeaderOp::Regex`. Closes RFC 001
   Unresolved §1 simultaneously.

## Future possibilities

- A `Regex` compilation cache for hot paths.
- Negated forms of header operators (`NotContains`, `NotStartsWith`)
  if user feedback requests them.
- Header value combinators (e.g. "matches at least one of these
  patterns") — deferred to a separate RFC.
