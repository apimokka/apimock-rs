# Acceptance / QA Checklist — RFC 046

**Governing RFC.** [RFC 046](../../done/046-test-harness-port-race-and-readiness.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## Established from source, not inherited

- [ ] What the server binds, and whether it can take a pre-bound listener
      or report its bound address, **established from the server crate**
      and cited by line
- [ ] The IPv6 bound-address tests **named explicitly** before any change
- [ ] Any contradiction with RFC 046 § Motivation **reported**

## Both defects fixed

- [ ] The port is **held** from selection until the server binds it — or,
      if the fallback design was used, the reason the preferred one was
      unavailable is stated
- [ ] Readiness is a connection poll with a timeout, **not** a fixed sleep
- [ ] Timeout failure message names the address and port
- [ ] The spawned task's error is **propagated**, not discarded

## Evidence

- [ ] `cargo test --workspace` run **≥ 10 consecutive times**
- [ ] Number of runs performed **reported**, not implied
- [ ] Failures: **zero** — any failure reported rather than re-run away
- [ ] IPv6 bound-address tests named again in the results
- [ ] Bind failure **forced deliberately**, shown to fail with a message
      naming the cause, then **reverted** — with the revert stated

## Scope held

- [ ] No test's assertions changed
- [ ] `env::set_current_dir` untouched, and no worse
- [ ] Any server-side change is a small additive accessor; anything
      larger was escalated instead

## Gates

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
