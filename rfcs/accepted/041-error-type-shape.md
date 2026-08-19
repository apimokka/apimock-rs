# RFC 041 — Error type shape: boxing, `kind()`, and `#[non_exhaustive]`

**Status.** **Accepted** — approved by the project owner 2026-08-20.
**Not yet implemented.** Breaking; 6.0.0.
[Handed off](../handoffs/041-error-type-shape/implementation-handoff.md) 2026-08-20,
with both open questions decided.
**Tracks.** v6 API quality. The error half of what
[RFC 052](../accepted/052-non-exhaustive-public-types.md) did for the
trace and request types.
**Touches.** `crates/apimock-config/src/error.rs`,
`crates/apimock-routing/src/error.rs`,
`crates/apimock-server/src/error.rs`, and the 15 call sites that
currently suppress a lint because of them.
**Depends on.** Nothing. **Blocked until** a major version, which 6.0.0
now is — see § Motivation.

## Summary

Box the two error variants that carry a parser error by value, add a
`kind()` accessor to each public error enum, and mark those enums
`#[non_exhaustive]`. One breaking change, taken once, at the boundary
that already exists for it.

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

## Goals

1. Remove all 15 `#[allow(clippy::result_large_err)]` suppressions by
   fixing the cause.
2. `#[non_exhaustive]` on all six public error enums.
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

### Where the break lands

Boxing is breaking for anyone constructing or destructuring the two
parse variants. `#[non_exhaustive]` is breaking for anyone matching any
of the six exhaustively. Both are 6.0.0 changes, and both belong in the
migration guide RFC 054 shipped — see § Testing.

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
- A `compile_fail` doctest proving `#[non_exhaustive]` is load-bearing
  on at least one error enum, in the manner RFC 052 established.
- The migration guide gains an entry per break.

## Risks

| Risk | Mitigation |
|---|---|
| `kind()` becomes a second taxonomy to maintain | It is derived mechanically from the variant; no independent state |
| Confusion with RFC 053's `ErrorKind` | Different crates, different names, and § Design says why they are separate |
| `#[from]` conversions silently dropped while adding boxes | The suppression count and the full suite both catch it; `From` impls are exercised by existing tests |
| Boxing changes `Display` by accident | Asserted explicitly, not assumed |

## Unresolved questions

1. **Do the kind enums live beside their errors, or in one shared
   module?** Beside, most likely — a shared module would couple three
   crates that currently only share a dependency direction.
2. **Does `WorkspaceError` keep wrapping `ConfigError`, or expose its
   own kinds?** It is documented as "a thin wrapper … kept as its own
   type so the `Workspace` API signals intent". If `kind()` just
   delegates, that intent is worth re-examining — but it is a separate
   question from this RFC's, and can be answered after.
