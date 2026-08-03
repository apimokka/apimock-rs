# Task Breakdown / PR Plan — RFC 030

**Governing RFC.** [RFC 030](../../proposed/030-warning-clean-baseline.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Four slices, four review units. Each is independently implementable,
testable, and reviewable. Do not combine them.

---

## Why this order

Dependency order — `routing` is the lowest crate, `apimock` the façade.
Working upward means any cross-crate ripple from a lint fix surfaces in
the slice that caused it, not three slices later. `apimock` is last and
largest because 34 of its 48 findings are in `tests/`, which is the
lowest-risk code in the workspace.

---

## Slice 1 — `apimock-routing` (27 findings)

**Scope.** `crates/apimock-routing/src/` only.

**Why first.** Lowest crate in the dependency chain; nothing depends
downward on it. It is also the crate whose count drifted 21 → 26
unobserved, so it is where the highest concentration of accumulated
idiom debt sits.

**Watch for.** `view/build.rs` (collapsible-if findings),
`rule_set/rule/when/request/` operator modules. The exhaustive matches
in `body_op_name` and friends are load-bearing — do not "simplify" an
exhaustive match into a catch-all arm. That exhaustiveness is what
caught the missing `StructuralContains` arm in v5.14.0.

**Exit.** `cargo clippy -p apimock-routing --all-targets --all-features
-- -D warnings` clean; `cargo test --workspace` still 371.

---

## Slice 2 — `apimock-config` (30 findings)

**Scope.** `crates/apimock-config/src/` only.

**Highest-risk slice.** This crate holds the public `Workspace` /
`EditCommand` / view-type surface the GUI depends on (DEC-014,
additive-only). A lint fix here is the most likely to drift into an API
change without anyone noticing.

**Watch for.** `workspace/edit.rs` is 822 ELOC — do not split it
(RFC 040's scope). Any finding touching a `pub` signature is an
escalation, not a fix. `Option<Vec<_>>` preservation semantics
(DEC-017: `None` = preserve, `Some([])` = clear, `Some([…])` = replace)
must survive untouched; a `map_or`-style simplification near that logic
deserves extra care.

**Exit.** `cargo clippy -p apimock-config …` clean; suite still 371.

---

## Slice 3 — `apimock-server` (17 findings)

**Scope.** `crates/apimock-server/src/` only.

**Smallest slice.** Includes findings in `tls.rs` and `trace.rs`.

**Watch for.** `tls.rs` findings get called out individually in the
review request rather than folded into a bulk commit — TLS is the one
security-relevant path in this codebase. `trace.rs` is 514 ELOC; do not
split it. The `broadcast` channel wiring and `ReloadableCertResolver`'s
`RwLock` are concurrency-sensitive; a "needless reference" or
"let-binding" fix near either gets an explicit note.

**Exit.** `cargo clippy -p apimock-server …` clean; suite still 371.

---

## Slice 4 — `apimock` façade + integration tests (48 findings)

**Scope.** `crates/apimock/src/` and `crates/apimock/tests/`.

**Largest but lowest-risk.** The bulk sits in `tests/` — notably
`tests/util/test_setup.rs`, `tests/util/http/test_response.rs`, and the
`tests/server/**` tree.

**Watch for.** The § 5 rule is absolute here: fix test *code*, never
test *assertions*. These 159 integration tests have never been part of
the recorded gate and are about to become the baseline — breaking one
now would be expensive to diagnose.

`src/cmd/match_test.rs` and `src/args.rs` carry the src-side findings.

**Exit.** Full workspace clippy clean under `-D warnings`; suite 371.

---

## Cross-slice completion

After slice 4, one final verification pass over the whole workspace, and
one review-request package covering all four slices (per
[`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)).

If a slice reveals that the total finding count differs from the 130
recorded here — clippy versions shift — report the actual number rather
than reconciling to this document. This plan's counts are a 2026-08-02
measurement on rustc 1.97.1, not a specification.
