# RFC 018 — ConditionalFallback strategy: audit and recommendation

**Status.** Withdrawn — audit confirmed that `ConditionalFallback`'s
intended semantics are already provided by the default multi-rule-set
fall-through dispatch. No code change needed. See addendum in
`done/007-rule-evaluation-strategy-variants.md`.
**Tracks.** RFC 007 follow-up — auditing the `ConditionalFallback`
strategy variant that RFC 007 advertised in its guide-level
explanation but never landed in the routing crate, and deciding
whether to implement it now or formally withdraw it.
**Touches.** `apimock-routing` (potentially `strategy.rs`,
`rule_set.rs`, and the dispatch path in `apimock-server::server`),
`rfcs/done/007-rule-evaluation-strategy-variants.md`
(addendum or explicit gap note), documentation.

## Summary

RFC 007 was marked **Implemented (v5.8.0)** but only four of its
five proposed strategies actually shipped: `UniformRandom`,
`WeightedRandom`, `Priority`, and (in v5.9.0 via RFC 011)
`RoundRobin`. The fifth — `ConditionalFallback { primary,
secondary }` — was never coded.

A v5.11 audit of the dispatch code reveals an important fact: the
behaviour `ConditionalFallback` was meant to provide **already
exists** in apimock today, just not as a `Strategy` variant.
`apimock-server::server::rule_set_response` already iterates over
`config.service.rule_sets` and falls through to the next set when
the current set produces no match:

```rust
for (idx, rule_set) in config.service.rule_sets.iter().enumerate() {
    if let Some(respond) = rule_set.find_matched(...) {
        return Some(respond);  // first set that matches wins
    }
}
None  // no set matched → caller tries fallback dir
```

This RFC presents the audit, proposes withdrawing the original
`ConditionalFallback { primary, secondary }` design, and recommends
a small documentation update to make the existing behaviour
discoverable.

## Motivation

There are three reasonable resolutions for an unimplemented
sub-proposal inside a "done" RFC, and we should pick one explicitly
rather than leave the gap silent:

1. **Implement it** — write the missing variant.
2. **Withdraw it** — document why it was dropped, point at the
   functionality that replaces it.
3. **Defer it** — leave the gap and revisit later.

(3) is what's happened by default since 5.8.0, and it's the worst
outcome: the done/ RFC still claims the feature exists, the
codebase silently lacks it, and any user who reads the TOML example
`strategy = { kind = "conditional_fallback", primary_rule_set = "primary.toml", secondary_rule_set = "secondary.toml" }`
will be confused when the parser rejects it.

This RFC resolves the gap by picking (2) — withdrawal with
clarification — for the reasons enumerated below.

## Audit findings

### Existing fall-through behaviour

The current dispatcher iterates `service.rule_sets` in declaration
order. Per rule-set the configured `Strategy` (FirstMatch,
UniformRandom, etc.) decides the match within that set. If a set
produces no match, the next set is tried. If no set matches,
control returns to the caller, which then tries the file-based
fallback respond directory.

In RFC 007's original framing, "primary" and "secondary" rule-set
behaviour was the point of `ConditionalFallback`. The audit shows
this is the default behaviour today — there is no opt-in needed.
The original `Strategy::ConditionalFallback` variant would be
redundant with what's already happening.

### What `ConditionalFallback` would have added beyond today

Re-reading the RFC 007 design, two specific behaviours would have
been new:

1. **Explicit rule-set selection.** `primary_rule_set =
   "primary.toml"` would let a `Strategy` reference a specific
   rule-set file by path, decoupling fall-through order from
   `service.rule_sets` declaration order.
2. **Strategy-level scope.** `Strategy` would gain a non-local
   variant — every other `Strategy` operates on the rules within a
   single set, but `ConditionalFallback` would cross sets.

Both add complexity: rule-set references need validation,
loop-detection, and a new dispatch path. The benefit over editing
`service.rule_sets` order in the TOML is small for the common case,
and the GUI editing surface for "reorder rule sets" already works
(per RFC 016 work).

### Conclusion of audit

The original `ConditionalFallback` variant was a solution searching
for a problem. The dispatch behaviour it would have produced is
already the default, and the only genuinely novel aspect (explicit
cross-set referencing) doesn't fit well into the `Strategy` shape
without introducing a non-local variant that's awkward in every
other way.

## Guide-level explanation

`Strategy::ConditionalFallback` is withdrawn from the proposed
strategy set. RFC 007 stands as Implemented for its other four
variants. The done/ RFC text is updated with a "5.11 addendum"
section pointing to this RFC.

For users who wanted "fall through to a secondary rule set", the
existing behaviour is now documented: declare multiple rule-sets
in `service.rule_sets`, order them most-specific to least-specific,
and rely on the existing fall-through. No new TOML syntax.

Documentation updates:

- `docs/src/technical-reference/architecture.md` (or the
  appropriate dispatch section): a short paragraph explaining that
  multi-rule-set dispatch is fall-through-on-no-match in
  declaration order.
- `docs/src/advanced-topics/`: an example showing two rule-sets,
  one specific and one catch-all, demonstrating the pattern.
- `rfcs/done/007-rule-evaluation-strategy-variants.md`: addendum
  noting that `ConditionalFallback` was withdrawn per RFC 018.

## Reference-level explanation

### Code changes

None. The dispatch code already does the right thing.

### Documentation changes

1. **Addendum to `rfcs/done/007*.md`.** A new section at the end:

   ```markdown
   ## v5.11 addendum — ConditionalFallback withdrawn

   This RFC's original "guide-level" section included a fifth strategy
   variant, `ConditionalFallback { primary, secondary }`. That variant
   was never implemented in v5.8.0 and has been formally withdrawn by
   RFC 018 (v5.11). The behaviour it would have provided is already
   the default for multi-rule-set configs: rule-sets are tried in
   declaration order; the first that matches wins; on no match the
   next is tried.

   See [RFC 018](../proposed/018-conditional-fallback-strategy.md)
   for the audit and rationale.
   ```

2. **Architecture / dispatch docs.** A 5–10 line paragraph
   describing the dispatch loop, suitable for a "How requests are
   matched" section of the technical reference.

3. **Worked example in advanced topics.** A two-rule-set TOML
   example showing the pattern that would have used
   `ConditionalFallback`, now expressed with default fall-through.

### Lifecycle action

On acceptance:

- This RFC moves from `rfcs/proposed/` to `rfcs/archive/` with
  status `Withdrawn — audit found the behaviour already exists;
  no code change needed`.
- The done/007 addendum lands in the same change.

If the audit conclusion is rejected (i.e. reviewers decide
explicit `ConditionalFallback` IS useful), this RFC stays in
proposed/ and is reworked into an implementation proposal. The
text below ("Rationale and alternatives") covers what that
implementation would look like.

## Drawbacks

1. **An RFC that ships as "withdrawn with documentation" can look
   like a non-event.** It isn't: the audit captures a real
   discrepancy and the documentation closes it. But reviewers used
   to RFCs producing code may find the shape unusual.
2. **The done/007 addendum precedent.** Editing a done/ RFC's body
   to add an addendum is a new pattern in this repo. The
   alternative — leaving done/007 silent and relying on readers to
   find this RFC — invites the same "ConditionalFallback was meant
   to ship" confusion in the future. Addendum is the lesser evil.
3. **Loses the explicit rule-set-by-name reference capability.**
   For users who want a strategy that names specific rule-sets, the
   withdrawal removes that direction. Mitigated by the fact that
   nobody has asked for it; if someone does, a fresh RFC can
   propose it without inheriting RFC 007's shape.

## Rationale and alternatives

**Alternative A (this RFC): withdraw the variant, document the
existing behaviour.** Lowest cost. Closes the discrepancy.
Captures the audit reasoning.

**Alternative B: implement `Strategy::ConditionalFallback { primary, secondary }`
as originally drafted.** Would land a new `Strategy` variant whose
dispatch crosses rule-set boundaries. Implementation work:

- New variant in `Strategy`.
- `RuleSetRef` type (string path or rule-set NodeId).
- Validation: primary and secondary must resolve to loaded rule-sets;
  no cycles.
- Modified dispatch loop that consults the strategy at service level
  before iterating rule_sets, with fallback to the existing
  iteration when the strategy is anything other than
  `ConditionalFallback`.
- Round-trip support in `toml_writer`.
- Tests covering: primary matches → primary wins; primary doesn't
  → secondary tried; cycle → load-time error; missing reference →
  validation error.

Total: roughly 200–300 lines of new code, mostly straightforward but
with one tricky bit (rule-set referencing through `service.rule_sets`
when ordering might change). Justified only if users actually need
named cross-set selection.

**Alternative C: implement a smaller construct — a per-rule-set
`terminal: bool` flag.** A rule-set marked `terminal = true` would
stop dispatch on no-match (return 404 immediately instead of
falling through). This inverts the question: instead of "how do I
chain sets?" the user asks "how do I stop chaining sets?". Smaller
implementation than B; orthogonal to `Strategy`. Could be added
later if a use case appears; not part of this RFC.

**Alternative D: defer.** Status quo. Confusing for readers of
done/007 who think the variant exists. Rejected.

We recommend A. C is parked for future consideration.

## Prior art

- WireMock's "mappings" load from multiple JSON files in
  alphabetical order; no explicit cross-file fall-through control.
  Closest analogue to apimock's existing behaviour.
- Mountebank's `imposters` are listed in startup config order; an
  imposter with no matching predicate falls through to the next.
  Same pattern.
- Neither tool offers an explicit "primary/secondary chain"
  construct, which is mild evidence the construct isn't widely
  needed.

## Unresolved questions

1. **Should the done/007 addendum go directly in `done/007*.md`, or
   in a sidecar file?** ✅ **Resolved.** Direct edit with a clear
   `## v5.11 addendum` heading. Keeps the historical record in one
   place; matches the "see source for one-place-truth" reading of
   RFC 000.
2. **Should `Strategy` add an exhaustive-test that fails if a
   variant exists in the enum but is unreachable from the
   dispatcher?** Useful guard against this class of bug recurring,
   but adds complexity. Defer; can land as a follow-up if a similar
   gap is discovered again.

## Future possibilities

- A `terminal: bool` rule-set flag (Alternative C above) if user
  feedback shows people want to stop fall-through explicitly.
- A separate `[[fallback_chain]]` root-config construct if
  cross-set composition needs become real.
- A dispatcher-level lint at server startup that warns when
  rule-set order looks accidental (e.g. a less-specific set
  ordered before a more-specific one). Tooling, not RFC scope.
