# Handoff — Tranche 3: performance

**Governing RFCs.** [071](../../accepted/071-share-application-state.md)
(shared state), [077](../../accepted/077-per-request-work.md)
(per-request work). Accepted 2026-09-01.
**Milestone.** Next minor. Independent of tranches 1 and 2.
**Baseline.** `main` @ `5d9e5bc`.

---

## 0. Order is not optional here

**071 first, then 077.** 071 is roughly **20× the cost of the entire 077
cluster**. Doing the tail first means measuring noise, concluding the
fixes did little, and possibly abandoning them.

## 1. Reproduce before fixing — this tranche especially

The audit measured **15.05× latency growth at 2,500 rules on requests
that match no rule at all**. I confirmed the mechanism structurally
(`AppState` holds `Config` by value, and `server.rs:362` clones it under
a lock per request) but **did not reproduce the number**.

> **Reproduce it first and report what you get.** If it does not
> reproduce, the finding is wrong and we should say so rather than
> shipping a change justified by a number nobody could repeat. A
> performance RFC whose measurement cannot be reproduced is a
> refactoring with a story attached.

## 2. Why our own benchmark missed this

Worth understanding before you add tests.

`cargo bench --bench routing` measures `find_matched` in isolation:
25.8 ns at 1 rule, 654 ns at 100. Those numbers are real and they are
**the small part**. The clone happens outside what the benchmark
measures, so a cost ~20× larger than the benchmarked operation was
invisible to the benchmark that exists.

**That is the lesson to encode**, not just the fix: the new cases must
be at the **HTTP level**, not the matcher level, and must include
**requests that match nothing** — the shape where the defect is
clearest, because the cost is proportional to configuration size rather
than to work done.

## 3. The traps

**071 — check for interior mutability before assuming nothing mutates.**
The RFC says nothing mutates `AppState` after startup. `TraceEmitter`
already holds `Arc<TraceConfig>` and is shared by design. **Check the
other fields rather than trusting that sentence** — if something does
mutate, `Arc` alone is wrong and you should stop and report.

**071 — the mutex goes too.** Removing the clone but keeping the lock
leaves every request serialised through it. Both halves, or the
concurrency assertion in § 4 will not pass.

**071 — record the hot-reload decision, do not make it by accident.**
Plain `Arc` closes the door on replacing state at runtime; `ArcSwap`
would not. Config hot-reload is a real gap (WireMock, json-server and
Mockoon all have it; ours is a `// todo:`). The RFC recommends plain
`Arc` now and says the decision should be recorded. **Say in your
package which you chose and why.**

**077 — measure each item; drop what shows nothing.** Four independent
micro-fixes. An unmeasured optimisation is churn. If one of P-05, P-06,
P-07 or P-09 shows no measurable gain, **say so and drop it** — that is
a legitimate outcome and a better one than four changes with one
justification between them.

**077 — P-05 must not change content-type detection.** The read-twice
shape exists because the code reads as text and falls back to binary,
and RFC 065's review established that fallback as load-bearing for
content-type. **Pin current detection with tests before touching it.**

## 4. Acceptance

**071**
- [ ] The 15× measurement reproduced **and reported**, before the fix
- [ ] After: latency at 2,500 rules flat relative to 1 rule for a
      **non-matching** request, within noise
- [ ] Concurrent requests scale — they cannot today, past the mutex
- [ ] Interior-mutability check done and reported
- [ ] Hot-reload decision stated

**077**
- [ ] Each of P-05/P-06/P-07/P-09 measured individually
- [ ] A directory with many files: latency flat versus a directory with
      few. **This assertion does not exist today and is what would have
      caught P-06**
- [ ] Content-type detection unchanged, pinned by tests written first
- [ ] Anything showing no gain is dropped, with the measurement quoted

**Both**
- [ ] Behaviour identical — these RFCs change no semantics
- [ ] Benchmark suite gains the HTTP-level cases from § 2
- [ ] Gates green; **API baseline diff empty or declared**
- [ ] CI green on all 12 jobs before merge

## 5. Not in scope

- Caching file contents in memory — invalidation design, and it
  interacts with the absent hot-reload.
- Restructuring `Config` or the response pipeline.
- Connection limits (tranche 1, RFC 074).

## 6. Report back

`.git-exclude/review-request/audit-t3-performance/`, including the
before/after numbers for 071, the per-item measurements for 077,
**anything you dropped and why**, and the hot-reload decision.
