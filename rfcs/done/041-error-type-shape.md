# RFC 041 — Public API shape: error boxing, `kind()`, and `#[non_exhaustive]` across the surface

**Status.** Implemented (v6.0.0). Accepted — approved by the project owner 2026-08-20.
**Not yet implemented.** Breaking; 6.0.0.
**Amended 2026-08-20, before implementation started** — scope broadened
from the six error enums to the whole re-exported public surface, on the
owner's decision. See § The decision that broadened this RFC.
[Handed off](../handoffs/041-error-type-shape/implementation-handoff.md) 2026-08-20,
with both open questions decided.
**Tracks.** v6 API quality. The error half of what
[RFC 052](../done/052-non-exhaustive-public-types.md) did for the
trace and request types.
**Touches.** The three `error.rs` files and the 15 call sites that
suppress a lint because of them; and, since the 2026-08-20 amendment,
every re-exported public type in the workspace — roughly 43 of them.
**Depends on.** Nothing. **Blocked until** a major version, which 6.0.0
now is — see § Motivation.

## Summary

Box the two error variants that carry a parser error by value, add a
`kind()` accessor to each public error enum, and mark **every
re-exported public type** `#[non_exhaustive]`. One breaking change,
taken once, at the boundary that already exists for it — after which
adding a field or a variant stops being breaking at all.

## Motivation

### The codebase is arguing with its own lint, 15 times

`clippy::result_large_err` is warn-by-default and CI runs
`-D warnings`. The lint currently fires nowhere, because it is
suppressed at **15 sites across 8 files**:

```
crates/apimock-config/src/config.rs                  ×5
crates/apimock-config/src/workspace.rs               ×3
crates/apimock-config/src/workspace/path_helpers.rs  ×1
crates/apimock-routing/src/rule_set.rs               ×1
crates/apimock-server/src/middleware.rs              ×1
crates/apimock-server/src/middleware/middleware_handler.rs ×1
crates/apimock-server/src/server.rs                  ×1
crates/apimock-server/src/tls.rs                     ×2
```

Overriding the suppressions (`--force-warn`) reports **136 bytes** at
seven sites and **144 bytes** at eight.

**This is not a lint we disagree with.** The suppressions carry a
comment saying so:

> clippy: `WorkspaceError` is a public error type (RFC 030 §6 escalation
> trigger); boxing its large variant would change that type's shape.
> See ESCALATION-002 in the RFC 030 review-request package.

The team escalated rather than silenced, the answer was "not without a
major version", and the suppression was the correct interim move. **6.0.0
is that major version.** This RFC exists to spend it.

### The cause is one type, and it is not the obvious one

Measured against the pinned `toml` 1.1.4:

| Type | Size |
|---|---|
| `toml::de::Error` | **88 bytes** |
| `PathBuf` | 24 |
| `Option<PathBuf>` | 24 |
| `std::io::Error` | **8** |
| `Box<toml::de::Error>` | 8 |

`RoutingError::RuleSetParse { path, canonical, source }` is
24 + 24 + 88 = **136** — exactly what clippy reports. The 144-byte
figure is the same payload behind one more wrapper.

**`std::io::Error` is 8 bytes**, because it is already boxed internally.
So the `ConfigRead` / `RuleSetRead` / `PathResolve` variants are
innocent, and the fix is narrower than "box the error types": box
`toml::de::Error` in the **two parse variants**, and all 15 suppressions
go away.

Boxing takes the payload to 24 + 24 + 8 = 56 bytes, plus discriminant —
under half of today's.

### Errors were left out of RFC 052

RFC 052 marked `TraceConfig`, `RequestSummary`, `ParsedRequest`,
`LogConfig` and `VerboseConfig` `#[non_exhaustive]`, closing risk R-09
for the trace and request path. **None of the six public error enums was
included**, and none is `#[non_exhaustive]` today:

`ConfigError`, `WorkspaceError`, `ApplyError`, `SaveError` (all
`apimock-config`), `RoutingError` (`apimock-routing`), `ServerError`
(`apimock-server`).

Error enums are where variants get added most often — every new failure
mode is one. Leaving exactly those types exhaustively matchable is the
same trap R-09 named, in the place most likely to spring it.

### The decision that broadened this RFC

**Amendment, 2026-08-20.** This RFC originally covered the six public
error enums. The owner decided `#[non_exhaustive]` is the **default for
apimock's public API from 6.0.0**, applied across the re-exported
surface in one pass rather than case by case.

The reasoning is the same one that justified the error enums, and it
does not stop at them. **Adding the attribute is itself breaking, so
6.0.0 is the last free window.** Every public type left bare after it
means adding a field to that type is a breaking change — a new trace
field, a new diagnostic, a new respond option would each force 7.0.0.

R-09 recorded this risk and it has already sprung once: RFC 040 added
three fields to `TraceConfig` in what was meant to be a minor release.
RFC 052 then fixed five types; RFC 058 fixed `Prefix`. Each was a
response to one instance. **This is the policy that stops the pattern
rather than the next instance of it.**

A sweep of the workspace found **84 bare public types**, of which about
**43 are re-exported and therefore reachable by a consumer**. Structs
with no public fields are excluded: they cannot be built by literal from
outside the crate anyway, so the attribute buys nothing.

Reproduce the list from each crate's `lib.rs` re-exports rather than
from a count in this document — the number is approximate, the method is
not.

### It pairs with RFC 039, and neither substitutes for the other

[RFC 039](../accepted/039-public-api-additive-only-gate.md) makes API changes
**visible** in review. `#[non_exhaustive]` makes additive ones
**legal** without a major bump. A project with only the gate learns
about every break after writing it; a project with only the attribute
never learns about the breaks the attribute cannot prevent — a removed
method, a narrowed return type.

## Goals

1. Remove all 15 `#[allow(clippy::result_large_err)]` suppressions by
   fixing the cause.
2. `#[non_exhaustive]` on **every re-exported public type** — the six
   error enums among them.
2b. **Every type that gains the attribute stays constructible** by any
   consumer that could construct it before. See § Construction.
3. A `kind()` accessor on each, so callers can branch on the failure
   class without matching variants they are now forbidden to match
   exhaustively.
4. Preserve every existing `Display` message, byte for byte.

## Non-goals

- Redesigning the error taxonomy. The variants are right; their shape
  and their openness are what change.
- Touching `apimock/src/cmd/envelope.rs`'s `ErrorKind`. That is RFC 053's
  CLI-facing taxonomy, deliberately separate from the library's — see
  § Design.
- Types that are `pub` but **not** re-exported, and structs with no
  public fields. Neither is reachable-and-constructible from outside, so
  neither benefits.
- Adding `kind()` to anything that is not an error enum.
- Boxing `io::Error` variants. Measured at 8 bytes; nothing to win.

## Design

### Boxing

```rust
RuleSetParse {
    path: PathBuf,
    canonical: Option<PathBuf>,
    #[source]
    source: Box<toml::de::Error>,
},
```

Same for `ConfigError::ConfigParse`. `#[source]` still works through the
box, so `Display` and `Error::source()` are unchanged — this is a
representation change, not a behavioural one.

Construction sites need `Box::new`. `#[from]` conversions that build
these variants need a manual `From` impl, since `#[from]` cannot box on
its own.

### `kind()`

`#[non_exhaustive]` obliges every downstream `match` to carry a wildcard
arm. Without a stable way to ask "what class of failure is this?", that
wildcard becomes a dead end and callers reach for string-matching on
`Display` — worse than what they had.

So each enum gains:

```rust
#[non_exhaustive]
pub enum ConfigErrorKind { Read, Parse, PathResolve, Validation, RuleSet }

impl ConfigError {
    pub fn kind(&self) -> ConfigErrorKind { … }
}
```

The kind enums are themselves `#[non_exhaustive]`, for the same reason
the errors are.

**These are two separate taxonomies and that is deliberate.** RFC 053's
`ErrorKind` is the CLI's contract, with a schema version and a stability
promise to agents. The library's `kind()` describes library failures.
Fusing them would tie a public CLI contract to internal error
refactoring — precisely the coupling RFC 053 § 2 avoided.

### Construction — the only part of the blanket change with real cost

For a type a consumer only **reads**, `#[non_exhaustive]` costs nothing:
matching gains a wildcard arm, field access is unaffected.

For a type a consumer must **build**, it forbids struct-literal syntax
from outside the defining crate — including functional-update syntax
(`Foo { x, ..Default::default() }`), because that is still a struct
expression. Fields stay public and assignable, so the supported pattern
becomes:

```rust
let mut p = RulePayload::default();
p.url_path = Some(…);
```

The GUI constructs four `EditCommand` payload types. Their current state:

| Type | Derives `Default`? | After the attribute |
|---|---|---|
| `RulePayload` | yes | fine — `default()` then assign |
| `RespondPayload` | yes | fine — `default()` then assign |
| `HeaderConditionPayload` | **no** | **unconstructable** |
| `BodyConditionPayload` | **no** | **unconstructable** |

**The last two must gain construction in the same change** — a `Default`
derive, or a `new()` taking the genuinely required fields. Shipping the
attribute without it would hand the GUI a break with no path through it,
which is the one outcome this RFC must not produce (Goal 2b).

Where `Default` is not meaningful for a type, `new()` is the answer;
where it is, derive it. Judge per type, and say in the submission which
you chose and why for any type where it was not obvious.

### Where the break lands

Boxing is breaking for anyone constructing or destructuring the two
parse variants. `#[non_exhaustive]` is breaking for anyone matching any
of the ~43 types exhaustively, or building one by struct literal. All of
it is 6.0.0, and all of it belongs in the migration guide RFC 054
shipped — see § Testing.

**The migration guide entry matters more than usual here**, because the
break is wide and shallow: many types, each with a mechanical fix. A
reader needs the pattern (`default()` then assign; add a wildcard arm),
not 43 individual entries.

## Testing and verification

- **All 15 suppressions deleted**, and `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` passes. The count is the
  acceptance test; a remaining suppression means the cause was not
  fixed.
- `--force-warn clippy::result_large_err` reports **zero** sites.
- Every error's `Display` output is unchanged — assert on the rendered
  strings, since these appear in user-facing diagnostics and in
  `validate`'s output.
- `Error::source()` still reaches the underlying `toml::de::Error`
  through the box.
- A `compile_fail` doctest proving `#[non_exhaustive]` is load-bearing —
  at least one error enum and at least one payload struct, in the manner
  RFC 052 established.
- **Every re-exported type that could be constructed before can still be
  constructed after.** This is Goal 2b and the acceptance bar for the
  blanket half: enumerate the constructible ones and show a construction
  path for each.
- The list of types that gained the attribute is reported, derived from
  the `lib.rs` re-exports rather than from this RFC's approximate count.
- The migration guide explains the *pattern* for each break class, not
  one entry per type.

## Risks

| Risk | Mitigation |
|---|---|
| `kind()` becomes a second taxonomy to maintain | It is derived mechanically from the variant; no independent state |
| Confusion with RFC 053's `ErrorKind` | Different crates, different names, and § Design says why they are separate |
| `#[from]` conversions silently dropped while adding boxes | The suppression count and the full suite both catch it; `From` impls are exercised by existing tests |
| Boxing changes `Display` by accident | Asserted explicitly, not assumed |
| **A type becomes unconstructable and the GUI cannot build it** | Goal 2b, § Construction, and an explicit acceptance check. This is the failure mode that would make the change worse than not doing it |
| A blanket sweep quietly catches a type that should stay open | The list is derived from re-exports and reported; a type nobody can reach is excluded by construction |
| 43 types is a large mechanical diff to review | It is mechanical *and* uniform, which is what makes it reviewable — the reviewer checks the rule was applied, not 43 judgements |

## Unresolved questions

1. **Do the kind enums live beside their errors, or in one shared
   module?** Beside, most likely — a shared module would couple three
   crates that currently only share a dependency direction.
2. **Does `WorkspaceError` keep wrapping `ConfigError`, or expose its
   own kinds?** It is documented as "a thin wrapper … kept as its own
   type so the `Workspace` API signals intent". If `kind()` just
   delegates, that intent is worth re-examining — but it is a separate
   question from this RFC's, and can be answered after.
