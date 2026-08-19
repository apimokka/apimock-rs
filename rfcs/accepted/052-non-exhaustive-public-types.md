# RFC 052 — `#[non_exhaustive]` on the public types that keep growing

**Status.** **Accepted** — **decision approved by the project owner
2026-08-17**; this document records it and scopes the work. Implemented
and merged to `main`; awaiting the 6.0.0 release.

**Tracks.** API stability. Five public structs assert that their field
set is final. It is not, and three RFCs this month have proved it.
**Touches.** `apimock-server` (`TraceConfig`, `RequestSummary`),
`apimock-routing` (`ParsedRequest`), `apimock-config` (`LogConfig`,
`VerboseConfig`), plus constructors where downstream construction is
genuinely needed. **A breaking change, deliberately.**
**Related.** Risk **R-09**. Unblocks
[RFC 050](./050-non-json-body-capture-decision.md) and
[RFC 051](./051-verbose-log-header-redaction.md) from taking the same
break piecemeal.

## Summary

Mark five public structs `#[non_exhaustive]`, in one deliberate break, so
that adding a field stops being a breaking change.

## Motivation

### The evidence is this month's

| RFC | What it added | To |
|---|---|---|
| 040 | three fields | `TraceConfig` |
| 050 | one field each | `ParsedRequest`, `RequestSummary` |
| 051 | possibly one | `VerboseConfig` / `LogConfig` |

None of the five is `#[non_exhaustive]`, so **every one of those
additions is a breaking change to a public API**. RFC 040's is already
committed and unreleased on `main`, which means 5.19.0 cannot honestly be
cut as a minor as things stand.

Nobody noticed. Not the implementer, not two reviewers, not me — until
the owner asked an unrelated question about whether two configuration
surfaces should be integrated. That is the strongest available evidence
that "absorb the breaks as they arrive" does not work here: it depends on
vigilance that has already been demonstrated absent.

### This is drift, not unawareness

`apimock-config`'s `view.rs` types **are** `#[non_exhaustive]`, with a
comment saying why: *"Every type here is `#[non_exhaustive]` so adding
fields later is not"* breaking. Someone thought this through once and
applied it in one place.

So the idiom is known in this codebase. It simply was not carried to the
types that turned out to grow fastest.

### Why the attribute is the right answer on the merits

- **Correct.** A struct without it asserts its field set is complete.
  For all five, that assertion is false.
- **Stable.** Removes a recurring break rather than paying it repeatedly.
- **Robust.** Structural rather than vigilant — the same principle as
  RFC 044's two-file trigger split and RFC 040's redact-at-capture, both
  of which this project adopted for the same reason.
- **Secure.** Downstream construction must go through a constructor,
  so **defaults are always applied**. A struct literal can silently omit
  a newly added safety field; a constructor cannot. Several fields these
  types are gaining *are* safety fields — `header_redaction` is one.
  There is also a second-order effect: if adding a field is expensive,
  there is pressure not to add one, and that pressure falls hardest on
  exactly the small safety controls worth adding.

## Goals

1. The five types are `#[non_exhaustive]`.
2. Anything downstream legitimately needs to construct, it still can —
   through a constructor or builder, not a literal.
3. The break is taken **once**, not five times.
4. What breaks, and for whom, is written down before it ships.

## Non-goals

- Changing any type's behaviour, fields, or defaults.
- Auditing every public type in the workspace. Five named types, and the
  general question is RFC 039's.
- Building RFC 039's additive-only gate. This RFC removes one class of
  break; 039 is what would have *caught* it.

## Proposed design

### The types divide in two, and so does the cost

**Output types — `RequestSummary`, `ParsedRequest`.** Consumers read
these; they do not construct them. The attribute costs essentially
nothing and is pure upside. `ParsedRequest` is re-exported at
`apimock-routing/src/lib.rs:43`, so it is genuinely public surface.

**Configuration types — `TraceConfig`, `LogConfig`, `VerboseConfig`.**
Downstream does construct these, so each needs a way to build one
without a literal. `Default` is implemented for all three, so a
`with_*`-style builder or a constructor taking the required fields is a
small addition — and arguably an improvement on a bare struct literal
regardless.

**Establish which types are actually constructed downstream before
designing the constructors.** The GUI is the one consumer we know of and
cannot see; do not guess at its usage, and do not add a builder nobody
needs.

### Timing

**Ship at the v6 boundary, not in a 5.x minor.** RFC 040's break is
already on `main`, so this RFC does not create the problem — it makes the
break deliberate, complete, and documented instead of accidental and
partial.

RFC 051, the security fix, is deliberately being attempted **without new
public fields** precisely so that it need not wait for this.

## Testing and verification

- The workspace builds and the full suite passes — internal code is
  unaffected, since `#[non_exhaustive]` constrains *other* crates only.
- A construction path exists for each configuration type that downstream
  plausibly builds, and it is exercised by a test.
- **Write the migration note as part of this change**, not after: what
  breaks (struct literals, exhaustive destructuring), and what replaces
  it. It belongs with v6's migration guide (RFC 048 § 7).

## Risks

| Risk | Mitigation |
|---|---|
| A downstream consumer we cannot see relies on literal construction | That is precisely what breaks, and it is the point of doing it deliberately. The migration note is the mitigation |
| Builders are added for types nobody constructs | Establish actual usage first; do not add speculative API |
| It is taken as licence to add fields freely | It removes the *compatibility* cost of adding a field, not the design cost. A field still has to earn its place |

## Unresolved questions

1. **Does the GUI construct any of the five?** The one consumer we know
   about. Ask alongside RFC 040's Q2 and RFC 042's round-trip rather than
   as a separate conversation.
2. **Do the enums want the same treatment?** `ConfigError` and friends
   are not `#[non_exhaustive]` either, which is why RFC 041 is deferred
   to 6.0.0. Same class of problem, larger blast radius, and worth
   deciding in the same window — but not folded into this RFC without
   that being an explicit choice.
