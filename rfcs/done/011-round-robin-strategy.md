# RFC 011 — RoundRobin rule-evaluation strategy

**Status.** Implemented (v5.9.0)
**Tracks.** RFC 007 follow-up — adding the round-robin (next-in-cycle)
strategy that RFC 007 Unresolved §2 flagged as "common in mock servers,
worth adding in a follow-up."
**Touches.** `apimock-routing` (`strategy.rs`, `rule_set.rs`),
`apimock-config` (`view.rs` — `ServiceStrategy` editing,
`workspace/edit.rs`), documentation, examples.

## Summary

`RoundRobin` is a stateful strategy: the server remembers which
response was last returned for a given URL-match group, and advances
to the next on each subsequent matching request. It is the standard
mechanism for testing "happy path then error" sequences without
modifying config between requests — a daily workflow for frontend
developers and integration test authors.

RFC 007 omitted it because it requires mutable state on `RuleSet`,
which the otherwise pure `find_matched` function does not carry.
This RFC proposes a minimal state model: an `AtomicUsize` index
per rule set, advanced on each `RoundRobin` call.

## Motivation

Common mock-server patterns that are awkward without round-robin:

- **First request succeeds, second fails** (error-handling test):
  rule 1 returns 200, rule 2 returns 503. Without round-robin,
  users either restart the server or write a Rhai middleware.
- **Paginated API simulation**: rule 1 returns page 1, rule 2
  returns page 2, rule 3 returns page 3, then cycle back.
- **Flaky network simulation**: alternating success and timeout for
  resilience testing.

WireMock, Mountebank, and MSW all support response sequencing.
It is among the most-requested features in mock-server tools.

## Guide-level explanation

```toml
[service]
strategy = "round_robin"
```

With this strategy, matching rules within a rule set are returned in
order, cycling back to the first after the last:

```toml
[[rule]]
when.request.url_path = "/api/orders"
respond.text = '{"status":"processing"}'

[[rule]]
when.request.url_path = "/api/orders"
respond.text = '{"status":"completed"}'
```

- Request 1 → `processing`
- Request 2 → `completed`
- Request 3 → `processing` (cycle)
- …

### Interaction with multiple matching rules

`RoundRobin` considers **all matching rules** in list order and
advances a per-rule-set counter for each request that produces a
match. Requests that match no rule do not advance the counter.

### Reset

The cycle resets on server reload (config change). There is no
explicit reset API in v1; a follow-up RFC can add one if needed.

## Reference-level explanation

### Strategy variant

```rust
pub enum Strategy {
    FirstMatch,
    UniformRandom { seed: Option<u64> },
    WeightedRandom { seed: Option<u64> },
    Priority { tiebreaker: PriorityTiebreaker },
    /// Cycle through matching rules in order, one per request.
    RoundRobin,
}
```

### State model

`RuleSet` gains one additional field:

```rust
pub struct RuleSet {
    // … existing fields …

    /// Per-rule-set round-robin counter. Incremented on each
    /// `RoundRobin` match; wraps at `usize::MAX`.
    ///
    /// `AtomicUsize` because `find_matched` takes `&self` (not
    /// `&mut self`) and must be callable from concurrent Tokio tasks
    /// without locking the entire rule set.
    #[serde(skip)]
    pub round_robin_counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
```

`#[serde(skip)]` + `Arc` means:
- Deserialization initialises to zero (default).
- Cloning `RuleSet` for `compute_derived_fields` shares the counter.
- After the initial `new()` pass, all rule-set clones in the server
  share the same `Arc` — a single authoritative counter per rule set.

### find_matched update

```rust
Strategy::RoundRobin => {
    use std::sync::atomic::Ordering;

    let matches: Vec<&Rule> = self
        .rules
        .iter()
        .enumerate()
        .filter(|(idx, r)| r.when.is_match(parsed_request, *idx, rule_set_idx))
        .map(|(_, r)| r)
        .collect();

    if matches.is_empty() {
        return None;
    }

    let idx = self.round_robin_counter
        .fetch_add(1, Ordering::Relaxed)
        % matches.len();

    Some(matches[idx].respond.clone())
}
```

`Ordering::Relaxed` is correct here: we need atomicity (no torn
read/write) but not sequential consistency — a slight reorder of
concurrent increments is acceptable. Two simultaneous requests
getting the same index is a race condition that is tolerable in a
mock server (vs. a bank transaction system).

### RuleSet::new update

Initialise `round_robin_counter` after deserialization:

```rust
ret.round_robin_counter = std::sync::Arc::new(
    std::sync::atomic::AtomicUsize::new(0)
);
```

For `compute_derived_fields` (which clones `self`), the `Arc` clone
is shared — all rules in the same rule-set share the same counter,
which is the desired behaviour.

### Config editing

`ServiceStrategy::RoundRobin` added. In `cmd_update_root_setting`:

```rust
"round_robin" => Strategy::RoundRobin,
```

### Tests

New `strategy/tests.rs` or additions to `rule_set` tests:

1. **`round_robin_cycles_through_matching_rules`** — Two matching
   rules; assert responses alternate A→B→A→B.
2. **`round_robin_skips_non_matching_rules`** — Three rules where
   only two match the URL; assert cycling ignores the non-matcher.
3. **`round_robin_single_match_always_returns_same`** — Only one
   rule matches; assert it is always returned.
4. **`round_robin_concurrent_increments_are_atomic`** — Spawn N
   Tokio tasks simultaneously calling `find_matched`; assert no
   panic and all returned indices are in range.

## Drawbacks

1. **Mutable state on an otherwise pure `RuleSet`.** The `AtomicUsize`
   makes `RuleSet` no longer purely declarative. This complicates
   snapshot semantics — should `RouteCatalogSnapshot` include the
   current counter? Probably not; the counter is operational, not
   declarative. Clearly document this distinction.
2. **Clone semantics changed.** Pre-RFC, cloning a `RuleSet` gives
   an independent copy. Post-RFC, clones share the `Arc` counter.
   Any code that clones a `RuleSet` expecting independence (e.g.
   tests that check `RuleSet::new` in isolation) must be audited.
3. **Non-determinism in tests.** Round-robin is deterministic from
   the server's perspective but non-deterministic from a test
   perspective when the initial counter state is unknown. Tests
   must either reset the counter or consume a known number of
   requests first.
4. **No persistence.** The counter resets on reload. Users who need
   persistent sequencing must use Rhai middleware.

## Rationale and alternatives

**Alternative A: `find_matched` takes `&mut self`.** Would allow a
plain `usize` counter. Rejected: `&mut self` requires a mutex at
the call site; `AtomicUsize` is lock-free and the relaxed ordering
is sufficient.

**Alternative B: counter on each individual `Rule` rather than the
rule set.** Would allow per-rule cycling independent of other rules.
More powerful but significantly more complex — deferred.

**Alternative C (this RFC): per-rule-set `AtomicUsize`, shared via
`Arc` across clones.** Minimal state, lock-free, correct for the
common case.

## Unresolved questions

1. **`RoundRobin` + `WeightedRandom` composition.** What if a user
   sets `strategy = "round_robin"` but some rules have `weight`
   fields? Ignore `weight` under `RoundRobin` — document clearly.
2. **Counter visibility in snapshot.** Should `RuleSetView` expose
   the current counter value so a GUI can show "next response will
   be rule #3"? Useful but adds snapshot churn on every request.
   Deferred — the GUI can infer from event count if it tracks the
   trace channel.
3. **Reset API.** A `Workspace::reset_round_robin(rule_set_id)`
   method would let tests reset state without reloading config.
   Useful; deferred to a follow-up RFC.
