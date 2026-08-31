# RFC 070 — `round_robin` must rotate per match group

**Status.** Accepted — owner approved 2026-09-01.
**Tracks.** Correctness. External audit 2026-09-01, F-01 (the High
finding on the functionality axis) and D-01.
**Touches.** `crates/apimock-routing/src/rule_set.rs`,
`docs/src/guides/vary-the-response-for-one-path.md`.

## Summary

The `round_robin` strategy keeps **one counter per rule set**, not one
per set of rules that match a given request. When a rule set serves more
than one distinct request shape, rotation breaks — and for some shapes
it never rotates at all.

## Motivation

**Verified 2026-09-01.** A rule set with `strategy = "round_robin"`, two
rules matching `/a` and three matching `/b`:

```
# /a alone, four times — correct
a1 a2 a1 a2

# alternating /a and /b — the defect
/a=a1 /b=b3   /a=a1 /b=b2   /a=a1 /b=b1   /a=a1 /b=b3
```

`/a` returns `a1` on every request. It never rotates.

The mechanism is arithmetic: the shared counter advances once per
request regardless of which group matched. `/a` therefore sees only even
counter values, and `0, 2, 4, 6 (mod 2)` is `0, 0, 0, 0`. Any two groups
whose sizes share a factor with the request interleaving will exhibit
some version of this; equal-sized groups alternating will pin every
group to one index.

**Why this ranks as high as it does.** The project's stated core value
is *predictable, inspectable matching* — "the rule that answers a given
request must be the one the author expects". This returns the wrong
response, silently, and the documentation
(`vary-the-response-for-one-path.md`) states the opposite:

> *"Cycles through matches in file order, one per request … Deterministic"*

True for a rule set with one match group. Described as if general. So a
user reading the guide, writing exactly what it shows, and adding a
second path gets behaviour that contradicts the page with no error
anywhere. That combination — wrong answer, confident documentation, no
diagnostic — is the worst case for a tool whose value is predictability.

## Goals

1. `round_robin` rotates independently for each distinct set of matching
   rules.
2. The guide describes the general case, not the single-group case.
3. The behaviour is pinned by a test that would have caught this.

## Non-goals

- Changing the other strategies. `first_match`, `priority`,
  `weighted_random` and `uniform_random` are stateless per request or
  correctly stateful, and the audit found no defect in them.
- Persisting rotation state across restarts. It is in-memory today and
  stays so.

## Design

Key the counter by the identity of the matching group rather than by the
rule set.

The natural key is the set of matched rule indices — two requests that
match the same rules share a counter; requests matching different rules
do not. It needs no configuration and no author-visible concept.

Implementation shape is the implementer's call, but the property is:

> **Two requests that select the same candidate rules must advance the
> same counter, and requests that select different candidates must not
> interfere.**

### The bounded-growth question, which must be answered not assumed

A map keyed by match group grows with the number of *distinct* groups a
rule set actually serves. That is bounded by the rule set's own
structure, not by request volume — a request either matches an existing
combination or one that the rules permit.

**Establish this rather than assume it.** If a construction exists where
distinct match groups grow with traffic, the map is a memory leak on a
long-running server and the design needs a bound. Report what you find
either way.

## Testing and verification

- **The reported scenario exactly**: two groups of size 2 and 3,
  alternating requests, both must rotate through their own rules.
- Single group unchanged — `a1 a2 a1 a2`.
- Three or more groups, interleaved.
- A group whose rules change between requests (config reload is not
  supported today, so this may be untestable — say so if it is).
- Memory does not grow across a long run of interleaved requests
  (§ Design's open question).
- The guide's example, executed as written, produces what the guide says.

## Risks

| Risk | Mitigation |
|---|---|
| Someone depends on the current sequence | They cannot sensibly — the current sequence is wrong and undocumented as such. A release note names it |
| Per-group state grows unboundedly | The design question above, to be answered before implementing rather than after |
| Group identity is ambiguous when rules overlap partially | The key is the exact matched set, so partial overlap yields a distinct key. Slightly more counters, still bounded by rule-set structure |
