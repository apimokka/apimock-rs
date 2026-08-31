# RFC 079 — Remove code that says something untrue

**Status.** Proposed — awaiting owner approval.
**Tracks.** Maintainability. External audit 2026-09-01, F-10, M-03a/b,
M-04a–e, M-09.
**Touches.** Several crates; each item is small and independent.

## Summary

A cluster of Low findings sharing one property: **the code asserts
something that is not true.** A `validate()` that validates nothing, a
binding that is always `true`, a statement with no effect, public
functions nothing calls.

Individually trivial. Together they are the reason a reader cannot trust
what they are looking at.

## Motivation

The audit's Low findings are mostly cleanups, and a list of cleanups is
easy to decline. These are worth doing because of what they cost a
*reader* — including the next auditor, and the GUI team now reading this
API:

- **F-10 / M-04e** — three public `validate()` methods that are no-ops
  (`rule_set.rs:343`, `guard.rs:10`, `default_respond.rs:9`). A caller
  reasonably assumes calling them means something. One of them,
  `RuleSet::validate()`, is public API a consumer can reach.
- **M-04d** — dead public API: `bad_request_response` is never called
  (and RFC 068 has a use for it), plus unused items in `tls.rs` and
  `control.rs`.
- **M-04c** — `let http_method_validate = true;`, a binding that is
  always true and therefore a branch that never varies.
- **M-04a** — `let _ = Path::new(dir_prefix);` — a statement with no
  effect, left where a reader expects meaning.
- **M-04b** — clones an `Arc` into a spawned task purely to drop it.
- **M-03a** — 32 sites of `let _ = write!` swallowing `fmt::Error`.
- **M-09** — `HttpMethod`'s `Display` renders `"HTTP Method is GET"`, a
  sentence where a value is expected. Anything interpolating it produces
  nonsense.

## Goals

1. A public `validate()` either validates or does not exist.
2. No binding, statement or clone that exists without effect.
3. `Display` renders a value.

## Non-goals

- Behaviour change. If removing something changes behaviour, it was not
  dead — **stop and report**.
- Removing `#[allow(dead_code)]` items that exist for a stated future
  reason. `[guard]` is a documented stub with an RFC-recorded
  disposition; leave it and its `validate()` alone unless the no-op is
  itself misleading, in which case say so rather than deleting.

## Design

Each item stands alone. Two need judgement rather than deletion:

**The no-op `validate()` methods** — removing a public method is a
**breaking change** and RFC 039's gate will show it. Options: remove
(breaking, cleanest), implement (if there is something to validate), or
document as intentionally trivial. **Recommend documenting for now** and
removing at the next incompatible release, so this RFC stays non-breaking.

**`bad_request_response`** — do not delete. RFC 068 and audit F-09 both
need it. Leaving a dead function that is about to be used is correct;
note *why* it is unused rather than removing it.

**M-03a's 32 `let _ = write!` sites** — a `fmt::Error` from a `String`
writer cannot occur in practice. The honest fix is a comment saying so
at the pattern's definition, not 32 edits. **Do the minimum that makes
it readable**; churn across 32 sites for an impossible error is worse
than the current state.

## Testing and verification

- Full suite green; **no behaviour change anywhere**.
- The public-API baseline diff is **empty**, or every change in it is
  declared and explained. If removing something moves the baseline, it
  was public API and needs the decision above.
- `Display` output pinned by a test.

## Risks

| Risk | Mitigation |
|---|---|
| A "dead" item is reachable from outside | The API baseline is the check — it lists everything public. Consult it before deleting |
| Cleanup churn obscures real changes | Keep it to one commit per item, or one PR with a clear per-item list |
| Removing a no-op `validate()` breaks a consumer | Hence the recommendation to document rather than remove within 6.x |
