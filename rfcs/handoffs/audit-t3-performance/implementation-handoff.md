# Handoff — Tranche 3: performance

**Governing RFCs.** [071](../../accepted/071-share-application-state.md)
(shared state), [077](../../accepted/077-per-request-work.md)
(per-request work). Accepted 2026-09-01.
**Milestone.** Next minor. Independent of tranches 1 and 2.
**Baseline.** `main` @ `88b3fc9`.
**Branch.** **Take one.** RFC 080 adopted trunk-based development —
ordinary work goes to `main` directly — but its § 3 carve-out keeps a
short-lived branch for changes that can behave differently on Windows
or macOS, and this tranche is squarely inside it: 077's P-05 reads
files and P-06 walks a directory listing. Only CI's `test` job is
matrixed and the development machine is Linux, so neither platform can
be checked before a push. Cut from `main`, merge once CI is green,
delete. RFC 066 § 5's naming still applies.

---

## 0. Order is not optional here

**071 first, then 077.** 071 is roughly **20× the cost of the entire 077
cluster**. Doing the tail first means measuring noise, concluding the
fixes did little, and possibly abandoning them.

## 1. Reproduce before fixing — this tranche especially

The audit measured **15.05× latency growth at 2,500 rules on requests
that match no rule at all**. I confirmed the mechanism structurally
(`AppState` holds `Config` by value — `server.rs:63` — and it is
deep-cloned under a lock once per request:
`grep -n 'app_state.lock()' crates/apimock-server/src/server.rs`,
currently line 459) but **did not reproduce the number**.

> Line numbers in this handoff are a convenience, not a contract. That
> clone was cited as `server.rs:362` when this document was drafted and
> had already moved before it went out. Search for the code, not the
> line.

> **Reproduce it first and report what you get.** If it does not
> reproduce, the finding is wrong and we should say so rather than
> shipping a change justified by a number nobody could repeat. A
> performance RFC whose measurement cannot be reproduced is a
> refactoring with a story attached.

## 2. Why our own benchmark missed this

Worth understanding before you add tests.

`cargo bench --bench routing` measures `find_matched` in isolation —
CPU-only, roughly 25.8 ns at 1 rule and 654 ns at 100 when the audit
ran it. Treat those as the audit's numbers to re-measure, not as
established fact. They are real and they are **the small part**: the
clone happens outside what that benchmark measures, so a cost ~20×
larger than the benchmarked operation was invisible to it.

**Correcting this handoff's earlier framing:** an HTTP-level benchmark
*does* already exist — `crates/apimock/benches/response_latency.rs`,
"the full path: TCP accept → hyper → routing → response". So the gap is
not "we only bench the matcher". It is narrower and more specific:
**neither bench exercises the shapes where these defects live.**

What to add, to `response_latency.rs` rather than to something new:

- **Many rules, and a request that matches none of them.** The shape
  where 071 is clearest, because the cost is proportional to
  configuration size rather than to work done.
- **A directory with many files versus one with few.** The shape that
  would have caught 077's P-06, and which § 4 asks you to assert on.

Extending a bench that already starts a real server and times what the
client sees is much less work than it sounds — read its header comment
first, particularly why the server starts once per group rather than
per iteration.

## 3. The traps

**071 changes the public API. Expect it, declare it, do not stall on
it.** RFC 071 does not mention this and its **Touches** line names only
`server.rs` — that is an understatement in my RFC, not something you
got wrong. `AppState` is public, and two baseline entries move when
`Config` becomes shared:

```
pub apimock_server::server::AppState::config: apimock_config::config::Config
pub fn apimock_server::server::AppState::new(Config, LoadedMiddlewares, TraceEmitter) -> Self
```

The field type changes, and so does the constructor's signature.
`AppState` is `#[non_exhaustive]`, so no outside crate builds one by
literal — but `new` is the sanctioned constructor and field reads
compile from anywhere.

**This is allowed.** RFC 039's gate declares changes; it does not
forbid them — *"deciding whether a break is allowed… is semver's job
and the owner's"* — and `docs/src/library/api-stability.md` now says so
correctly, after tranche 2 exposed that it did not. So:

- Update the baseline deliberately, in the same commit as the change.
- Say plainly in your package that the public API moved, and how.
- Add a migration-guide entry alongside the code, the way
  `docs/src/guides/migrating-to-6-1.md` now carries RFC 070's removed
  field. A library consumer meeting `AppState::new` as a compile error
  should have prose to read.

**Do not treat a baseline diff as a blocker or an escalation.** Tranche
2 lost a round-trip to exactly that, because I had assured them in
writing that the API was not in play. It was. Here it is, up front.

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
would not.

This is weightier than "a `// todo:`", which is how I first put it and
is not accurate. The project has already built the *classification* for
config reload — `ReloadHint` in `apimock-config/src/view.rs`, with
`requires_reload` and `requires_restart` — while
`apimock-server/src/control.rs:17` states plainly that
**"implementation of actual shutdown / reload wiring is stage-2 work"**.
TLS certificates already hot-reload (RFC 020, `reload_tls_certs`).

So the intention is on the record and partly built; only the config
wiring is deferred. Choosing plain `Arc` is still very likely right for
now — but make it a decision, not a side effect. The RFC recommends
plain `Arc` and asks that the choice be recorded. **Say in your package
which you chose and why, and whether it makes stage-2 harder.**

**077 — measure each item; drop what shows nothing.** Four independent
micro-fixes. An unmeasured optimisation is churn. If one of P-05, P-06,
P-07 or P-09 shows no measurable gain, **say so and drop it** — that is
a legitimate outcome and a better one than four changes with one
justification between them.

**077 — P-05 must not change content-type detection.** The read-twice
shape exists because the code reads as text and falls back to binary,
and RFC 065's review established that fallback as load-bearing for
content-type. Start from
`grep -n read_to_string crates/apimock-server/src/response/file_response.rs`.
**Pin current detection with tests before touching it.**

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
- [ ] Gates green; **API baseline diff empty or declared** — for 071 it
      will not be empty (§ 3), so: baseline updated in the same commit,
      the move described in the package, and a migration-guide entry
      written
- [ ] CI green on all 12 jobs before merge — on the branch head, and
      re-verified on `main` after the merge (RFC 066 Amendment 3, now
      binding under RFC 080)

## 5. Not in scope

- Caching file contents in memory — invalidation design, and it
  interacts with the absent hot-reload.
- Restructuring `Config` or the response pipeline.
- Connection limits (tranche 1, RFC 074).

## 6. Report back

`.git-exclude/review-request/audit-t3-performance/`, including the
before/after numbers for 071, the per-item measurements for 077,
**anything you dropped and why**, and the hot-reload decision.
