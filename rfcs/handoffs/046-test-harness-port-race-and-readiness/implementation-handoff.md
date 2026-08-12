# Implementation Handoff — RFC 046, Test harness port race and readiness

**Governing RFC.** [RFC 046](../../done/046-test-harness-port-race-and-readiness.md)
**Milestone.** M3 — **P0, the only P0 in the milestone**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

The integration harness loses ports to itself, and hides the evidence
when it does. Fix both. Do not change what any test asserts.

This is a **release-gate** fix, not housekeeping: `quality-gate` in
`.github/workflows/release-executable.yaml` runs `cargo test --workspace`
on every tag push, so a flake here fails a release.

## 2. Establish these from source before designing

Read RFC 046 § Motivation for the diagnosis, then verify it yourself.
Two things the RFC deliberately does **not** answer, because the answers
decide the design and must not be inherited from a document:

1. **What address(es) does the server actually bind, and can it accept a
   pre-bound listener or report its bound address?** Find this in the
   server crate. This determines whether the preferred design (bind port
   0, hand the listener over, read the port back) is available.
2. **Which tests assert on a bound address?** A previous attempt during
   RFC 036 regressed **every IPv6 bound-address test** and was reverted
   in full. Name those tests in your review request before you change
   anything, so we both know what the blast radius is.

If your reading contradicts anything in RFC 046 § Motivation, **report
the contradiction** — that is a useful result, not a problem. The RFC's
reading of `crates/apimock/tests/util/test_setup.rs:117` and
`launch_impl` is line-cited, so it is checkable.

## 3. The two defects

**Defect 1 — the port is probed, not held.** `dynamic_port()` binds a
random port, matches `Ok(_)` which drops the listener immediately, and
returns the number. The server binds it later, after a `tokio::spawn` and
a 400 ms sleep. Anything may take it in between; two tests may also probe
the same port concurrently and both see it free.

**Defect 2 — readiness is a fixed sleep, and startup failure is
discarded.** `launch_impl` spawns the server and sleeps 400 ms. The
spawned task's result is dropped, so a failure to bind is never reported;
the test proceeds and fails at its first request with a connection error
that names nothing.

**Fix both.** Defect 1 alone leaves a harness that cannot distinguish
"not ready yet" from "failed to start", which is the reason this bug has
been mysterious rather than obvious.

## 4. Direction, not prescription

Preferred, if § 2.1 allows it: bind port 0, let the OS assign, hand that
listener to the server, read the port back from it. No window exists
because the port is never released.

If the server must bind by number, hold the listener until the server
binds and retry on collision — and **say in the review request why the
preferred design was not available.** Narrowing a race is worse than
closing one, so that choice needs its reasoning recorded.

Either way: replace the fixed sleep with a connection poll until success
or timeout, make the timeout message name the address and port, and
propagate the spawned task's error instead of dropping it.

## 5. Scope boundaries

- **In scope:** `crates/apimock/tests/util/test_setup.rs`, and a minimal
  additive accessor on the server if § 2.1 requires one.
- **Out of scope:** what any test asserts; suite runtime as a goal;
  `env::set_current_dir` in `TestSetup.current_dir_path` — that is a real
  process-global hazard, it is recorded in `ROADMAP.md`'s findings table,
  and it is **not yours to fix here**. Do not make it worse.
- If the server-side change grows beyond a small accessor, **stop and
  escalate** rather than widening scope.

## 6. Evidence required

A single green run is not evidence for a flakiness fix.

- `cargo test --workspace` **at least 10 consecutive times**. Report the
  number of runs actually performed and the number that failed. Zero
  failures is the bar. If any run fails, the fix is not done — report it
  rather than re-running until it looks clean.
- The IPv6 bound-address tests named in § 2.2 pass, and are named again
  in the results.
- **Demonstrate Defect 2 is fixed:** temporarily force a bind failure,
  show the test now fails with a message naming the cause, then revert
  the forcing and say so explicitly.

## 7. Escalation

Per project convention, anything blocking or any design question goes in
a `.git-exclude/review-request/` package, not only in chat. That includes
the § 2 findings if they contradict the RFC.
