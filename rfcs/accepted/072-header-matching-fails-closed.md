# RFC 072 — Header matching must fail closed

**Status.** Accepted — owner approved 2026-09-01.
**Tracks.** Correctness / security-adjacent. External audit 2026-09-01,
S-04.
**Touches.** `crates/apimock-routing/.../headers.rs`,
`crates/apimock/src/cmd/match_test.rs` (or `rule_check.rs`), tests.

## Summary

When a request header's value is not valid UTF-8, the header condition
returns **`true`** — it matches. A condition intended to gate a rule can
be bypassed by sending obs-text bytes.

Worse, `match-test` handles the same case **oppositely**, so the tool
whose purpose is predicting the server contradicts it.

## Motivation

`headers.rs:81-93` returns `true` on a `to_str()` error. The reasoning
was presumably "we cannot compare, so do not block" — but a header
condition is a *gate*, and the safe answer when a gate cannot evaluate
its input is to not open.

Concretely: a rule that says "match only when `x-token: expected`" will
match a request sending `x-token: <invalid utf-8>`.

**The second half is what makes this more than a one-line fix.**
`rule_check.rs:132` — the path `match-test` uses — treats the same input
as a non-match. So:

- The server matches the rule.
- `apimock match-test`, asked to predict that, says it does not.

RFC 055 and RFC 059 built `get` and `match-test` on the promise that
they answer *identically to the running server*. Here they do not, and
the divergence is invisible because no test compares them on this input.

## Goals

1. A header value that cannot be read as UTF-8 **does not satisfy** a
   header condition.
2. The server and `match-test` agree on that input.
3. A test asserts the agreement, so the two cannot drift again.

## Non-goals

- Supporting non-UTF-8 header *matching* (e.g. byte-wise comparison).
  Out of scope; if someone needs it, that is a feature request with a
  design, not a bug fix.
- Changing behaviour for valid UTF-8 values.

## Design

`return false` on the `to_str()` error, and align the `match-test` path
to the same answer.

**The agreement test is the deliverable, not the flip.** Changing
`true` to `false` is one line; what prevents recurrence is a test that
runs the same request through both paths and asserts the same verdict —
the same shape as `respond_validator_agreement.rs`, which exists because
two validators diverged in exactly this way.

## Testing and verification

- A header condition with a non-UTF-8 value: server **does not match**.
- `match-test` on the same input agrees.
- **An agreement test** over a corpus including: valid UTF-8, invalid
  UTF-8, empty value, absent header, and each operator kind
  (`exists`/`absent` included — an `absent` condition on a non-UTF-8
  header is the interesting case, since "cannot read" and "not present"
  are different things and should not be conflated).
- `get --why` explains the non-match rather than saying nothing.

## Risks

| Risk | Mitigation |
|---|---|
| Someone relies on the fail-open | Extremely unlikely, and it is a bypass. Named in the release note as a behaviour change |
| `absent` semantics get conflated | Called out explicitly in the test list above; decide it deliberately and document which way |

## Unresolved questions

1. **What should `absent` mean for an unreadable header?** The header
   *is* present, so `absent` should presumably be false — but the value
   cannot be read, which is what made the positive case fail. Establish
   and document; do not let it fall out of the implementation.
