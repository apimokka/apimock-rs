# RFC 071 — Stop deep-cloning application state on every request

**Status.** Accepted — owner approved 2026-09-01.
**Tracks.** Performance. External audit 2026-09-01, P-01 (the High
finding on the code-quality axis) and P-02.
**Touches.** `crates/apimock-server/src/server.rs`.

## Summary

Every request takes a mutex and deep-clones the whole `AppState` —
including `Config`, which holds every rule set and every rule. The audit
measured **15.05× latency growth at 2,500 rules on requests that match
no rule at all**.

## Motivation

`server.rs:362`:

```rust
let shared_app_state = { app_state.lock().await.clone() };
```

`AppState` holds `config: Config` **by value** (`server.rs:55-65`). So
the clone copies the entire parsed configuration — rule sets, rules,
conditions, respond blocks — once per request.

**Verified structurally 2026-09-01**; the audit's measurement is the
consequence.

Two things make this worse than an ordinary inefficiency:

- **The cost is paid on requests that match nothing.** It is not
  proportional to work done; it is proportional to how much
  configuration exists. The audit puts it at roughly 20× the cost of the
  matching the project actually benchmarks — so our own `routing`
  benchmark, which measures `find_matched` in isolation at 25.8 ns to
  654 ns, is measuring the small part.
- **It undercuts a stated use case.** "Large mock data" is something
  this project invites; the per-request cost grows with exactly the
  thing users are encouraged to scale.

The mutex is the second half. Every request serialises through one lock
to take that clone, so the clone is not merely expensive but
non-concurrent.

## Goals

1. Per-request cost independent of configuration size.
2. No mutex on the read path.
3. The `routing` benchmark suite gains a case that would have shown this.

## Non-goals

- Config hot-reload. Sharing state immutably makes reload *harder*, not
  easier, and reload is not implemented today (the audit's F-15 notes
  the `// todo:`). If reload is wanted later it needs its own design —
  see Unresolved.
- Restructuring `Config` itself.

## Design

`Arc<AppState>`, cloned per request as a pointer bump. The mutex goes:
nothing mutates `AppState` after startup.

Each field currently moved out of the clone (`config`, `middlewares`,
`tracer`, and the two canonicalised path caches) is then read through
the `Arc`.

That is the whole change. It is small, and its smallness is worth
stating plainly — the finding is significant and the remedy is not.

## Testing and verification

- **A benchmark that would have caught this.** The current `routing`
  bench measures `find_matched` alone and therefore missed a cost ~20×
  larger. Add an HTTP-level case at 1, 100 and 2,500 rules, including
  **requests that match nothing** — the shape where the defect is
  clearest.
- Latency at 2,500 rules must be flat relative to 1 rule for a
  non-matching request, within noise.
- Concurrent requests must not serialise: throughput under N concurrent
  clients should scale, where today it cannot past the mutex.
- Full suite green; behaviour identical.

## Risks

| Risk | Mitigation |
|---|---|
| `Arc` makes future mutation harder | True, and deliberate. Nothing mutates it today; a future reload design should own that problem explicitly rather than inherit a per-request clone that happens to allow it |
| The measurement does not reproduce | Then the finding is wrong and we should say so. Reproduce it **before** changing anything, and report the number |
| Hidden interior mutability in a field | `TraceEmitter` already holds `Arc<TraceConfig>` and is shared by design. Check the others rather than assuming |

## Unresolved questions

1. **Does removing the mutex close the door on config hot-reload?**
   Hot-reload is a real gap (WireMock, json-server and Mockoon all have
   it; the audit lists it as Missing). `Arc<ArcSwap<AppState>>` or
   equivalent would give both sharing and replacement.
   **Recommend plain `Arc` now** — it fixes a measured defect today, and
   swapping `Arc` for `ArcSwap` later is a contained change. But the
   decision should be recorded rather than made by accident.
