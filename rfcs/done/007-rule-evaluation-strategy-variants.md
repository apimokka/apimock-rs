# RFC 007 — Rule-evaluation strategy variants beyond `first_match`

**Status.** Implemented (v5.8.0)
**Tracks.** Routing core extension — adding strategies for selecting
among multiple matching rules, beyond the current first-match.
**Touches.** `apimock-routing` (`Strategy` enum, dispatcher),
`apimock-config` (`ServiceStrategy` editing surface, validation),
documentation, examples.

## Summary

The current `Strategy` enum has a single variant, `FirstMatch`,
encoding "evaluate rules in order, return the first that matches".
This RFC proposes additional strategies for common mock-server use
cases: priority-ordered, random pick among matches, weighted
selection, and conditional fallback. Each variant is described with
selection semantics, configuration shape, and interaction with the
existing rule ordering.

## Motivation

`first_match` works for the predominant mock-server use case:
hand-authored rules ordered most-specific to least-specific. But
several recurring patterns are awkward to express:

- **A/B fixtures.** "70% of the time return user-active.json, 30% of
  the time return user-inactive.json." Today: write a Rhai
  middleware. Trivially expressible as a weighted strategy.
- **Random sampling among equivalent rules.** Useful for chaos / load
  testing: "match this URL, then pick uniformly at random from these
  five possible responses." Today: not possible without scripting.
- **Priority with deterministic tiebreaker.** "Rules in this rule
  set have priority weights; among ties, fall through to the next
  matching rule." Today: encoded informally in rule order.
- **Conditional fallback (try-then-fallback).** "Try matching at the
  primary rule set, fall through to the secondary only if no primary
  rule matches." Today: not possible without bespoke composition.

These are routine in commercial mock servers; adding them to apimock
removes a category of "I had to drop into Rhai for what should be a
config decision" friction.

## Guide-level explanation

The `[service]` section gains explicit strategy variants:

```toml
[service]
strategy = "first_match"        # today's default

# or

[service]
strategy = { kind = "weighted_random", seed = 42 }

# or

[service]
strategy = { kind = "priority", tiebreaker = "first_match" }

# or

[service]
strategy = "uniform_random"

# or

[service]
strategy = { kind = "conditional_fallback", primary_rule_set = "primary.toml", secondary_rule_set = "secondary.toml" }
```

For weighted strategies, individual rules carry an optional weight:

```toml
[[rule]]
weight = 70
when.request.url_path = "/api/users/1"
respond.file_path = "user-active.json"

[[rule]]
weight = 30
when.request.url_path = "/api/users/1"
respond.file_path = "user-inactive.json"
```

## Reference-level explanation

### Strategy variants

```rust
pub enum Strategy {
    FirstMatch,
    UniformRandom { seed: Option<u64> },
    WeightedRandom { seed: Option<u64> },
    Priority { tiebreaker: Box<Strategy> },
    ConditionalFallback {
        primary: RuleSetRef,
        secondary: RuleSetRef,
    },
}
```

### Selection semantics

Given a sequence of rules `R1..Rn`, the strategy decides which
matching rule is selected:

- **FirstMatch.** Walk `R1..Rn`, return the first `Ri` that matches.
  (Today's behaviour.)
- **UniformRandom.** Compute the set `M` of all matching rules;
  return a uniformly random pick from `M`. Empty `M` → no match.
- **WeightedRandom.** Compute `M`; pick weighted by each rule's
  `weight` field (default `1`). Empty `M` → no match.
- **Priority.** Group `M` by priority; pick the highest-priority
  group; apply the `tiebreaker` strategy within the group. Empty
  `M` → no match.
- **ConditionalFallback.** Run the primary rule set with its own
  strategy; if no match, run the secondary rule set. Composable.

### Rule fields

Rules gain optional fields:

```rust
pub struct Rule {
    // existing fields …
    pub weight: Option<u32>,           // for WeightedRandom
    pub priority: Option<i32>,          // for Priority
}
```

Both fields are ignored under strategies that don't use them.
Validation: `weight >= 1` if present; `priority` may be any `i32`
(higher = higher priority).

### Determinism

For `UniformRandom` and `WeightedRandom`, the seed determines
reproducibility:

- `seed = Some(n)`: the RNG is seeded once per server startup with
  `n`. Successive requests draw from the same sequence — useful for
  reproducible tests.
- `seed = None`: a non-deterministic seed (OS RNG) is used.

For `Priority { tiebreaker }`, determinism follows the tiebreaker's
rules.

### Configuration

`RootSettingKey::ServiceStrategy` already exists. With this RFC its
`EditValue` becomes richer than a plain string — likely a JSON-encoded
`Strategy` value. The exact shape on the wire is open (see unresolved
questions); the recommended path is a tagged enum encoded as JSON,
mirroring the TOML shape above.

### Interaction with existing rule ordering

`FirstMatch` is the only strategy that depends on rule order. The
other strategies select from the set of matching rules without
regard to order. This means:

- Users moving from `FirstMatch` to `WeightedRandom` may see
  different behaviour even if the rule set is unchanged.
- `MoveRule` (the existing edit command) has no semantic effect
  under non-`FirstMatch` strategies, though it remains valid as a
  cosmetic reordering.

The GUI should surface this — e.g. greying out the move-up / move-down
controls when the active strategy doesn't use ordering.

### Per-rule-set strategies

Strategies live on the `[service]` root section today. A per-rule-set
strategy override is possible but adds a layer of "which strategy
applies?" cognitive load. This RFC keeps strategies at root only for
v1; per-rule-set overrides are listed as a future possibility.

## Drawbacks

1. **Behavioural surprise risk.** Switching strategies changes
   semantics globally. A user who flips from `FirstMatch` to
   `UniformRandom` may not anticipate that rule order stops
   mattering.
2. **Test complexity.** Non-deterministic strategies make rule-set
   testing harder. Mitigation: seeded RNG ships from day one;
   documentation strongly recommends seeding in test environments.
3. **Combinatorial growth in the routing layer.** Each new strategy
   needs its own selection logic, its own test coverage, and its own
   interaction with `WhenView` / per-rule diff. The four variants in
   this RFC are well-scoped, but if many more are added the
   abstraction may need rethinking (e.g. strategies as a trait, not
   an enum).
4. **`Priority` and `WeightedRandom` introduce config fields on
   rules that are silently ignored under other strategies.** Users
   may set `weight` and not understand why it has no effect under
   `FirstMatch`. Documentation must call this out clearly.

## Rationale and alternatives

**Alternative A: keep `FirstMatch` only; expose richer behaviour via
Rhai middleware.** Rhai already exists in the routing crate. Pushing
this work to middleware preserves a small core. The cost is poor
ergonomics for common patterns (weighted random in a one-line config
vs a Rhai script with state).

**Alternative B (this RFC): typed strategies as first-class config.**
Best ergonomics for common patterns; adds core surface area.

**Alternative C: a single `Custom { rhai_script: String }` strategy
that delegates to a user-provided Rhai expression.** Maximally
flexible; same ergonomic gap as A for common patterns.

**Alternative D: strategy as a trait with built-in implementations
plus user-defined plugins.** Most extensible; largest implementation
surface.

We pick B. A and C punt; D is over-engineered for the use cases
listed. B can grow into D later if community-contributed strategies
become a thing.

## Prior art

- WireMock's "scenarios" feature handles stateful sequencing; closer
  to a finite state machine than a strategy enum, but the
  "weighted scenario" pattern is similar to our `WeightedRandom`.
- Mountebank's `predicates` + `responses` array uses a per-stub
  cycle: responses are returned in round-robin order. A fifth
  strategy worth considering for a follow-up.
- nginx's `upstream` directive supports `least_conn`, `ip_hash`,
  weighted variants — different domain (load balancing) but the
  same shape (strategy enum at config root).

## Unresolved questions

1. **TOML encoding of strategy variants.** The example above uses
   inline tables (`{ kind = "weighted_random", seed = 42 }`). An
   alternative is `[service.strategy]` subsections. Probably both
   should work via standard TOML; the recommendation here is to
   pick whichever serde produces by default.
2. **Round-robin (next-in-cycle) strategy.** Common in mock servers
   (Mountebank, WireMock). Stateful (the server tracks which
   response was last returned), which complicates the otherwise
   pure-function dispatcher. Worth adding in a follow-up?
3. **Priority tiebreaker default.** The `Priority` variant requires
   a tiebreaker; what's the default? `FirstMatch` is conservative
   and most intuitive — recommended.
4. **Interaction with the routing snapshot.** The snapshot today
   reports rules in TOML order. Should it indicate the active
   strategy's intended selection (e.g. mark a rule as "highest
   priority among matches for /api/users")? Out of scope for the
   strategy implementation itself but a useful debug surface.

## Future possibilities

- `RoundRobin` / sequence-based strategies (stateful).
- Per-rule-set strategy overrides.
- A `Custom { rhai_script: String }` variant for users who outgrow
  the built-in variants — the trapdoor escape hatch from B back
  toward C.
- Strategy traits + plugin loading (a path to alternative D).
- Strategy-aware diff: when changing strategies via the GUI, the
  diff describes the behavioural shift, not just the field change.

---

## v5.11 addendum — ConditionalFallback withdrawn (RFC 018)

This RFC's original guide-level section included a fifth strategy variant,
`ConditionalFallback { primary_rule_set, secondary_rule_set }`.
That variant was **never implemented** in v5.8.0 and was formally
**withdrawn** by RFC 018 (v5.11.0).

The v5.11 audit found that the behaviour `ConditionalFallback` was
meant to provide — "try primary rule-set; fall through to secondary on
no match" — is already the default behaviour for multi-rule-set
configurations. `apimock-server::rule_set_response` iterates
`config.service.rule_sets` in declaration order; each set that
produces no match causes the loop to continue to the next:

```rust
for rule_set in &config.service.rule_sets {
    if let Some(respond) = rule_set.find_matched(...) {
        return Some(respond);  // first matching set wins
    }
}
None  // no set matched → fallback dir
```

To implement "primary then secondary" semantics, declare two rule-sets
in order in `apimock.toml`:

```toml
[service]
rule_sets = ["primary.toml", "secondary.toml"]
```

No special strategy variant is needed. The `ConditionalFallback`
proposal is preserved in `rfcs/archive/` via RFC 018, which contains
the full audit rationale.
