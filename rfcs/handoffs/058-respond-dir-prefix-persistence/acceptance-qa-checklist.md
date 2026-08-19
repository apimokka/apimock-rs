# Acceptance / QA Checklist — RFC 058, `respond_dir` persistence

**Governing RFC.** [RFC 058](../../accepted/058-respond-dir-prefix-persistence.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md) —
decisions and mechanism; you do not need any other RFC in hand.

Every box needs captured evidence in the review-request package, not an
assertion that it passes.

## A. The bug is dead

- [ ] **Load+save is a fixed point.** Save three times; the file is
      **byte-identical** after each. Show the checksums.
- [ ] RFC 057's W7 script, run **three times over**, leaves the config
      byte-stable after the first run.
- [ ] `apimock set rule …` run five times in a fresh directory: the
      `[prefix]` block never grows.

## B. Nothing the user wrote is disturbed

- [ ] A rule set with **no `[prefix]` section** still has none after a
      save. This is the manufacture-from-nothing half — do not skip it
      because § A passes.
- [ ] `respond_dir = "responses"` round-trips unchanged.
- [ ] `respond_dir = "."`, written explicitly, round-trips unchanged.
- [ ] A hand-written file with comments and non-canonical formatting
      still survives a save (RFC 056's guarantee, re-proved here because
      this RFC edits the write path's inputs).

## C. The narrow repair, and only the narrow repair

- [ ] `respond_dir = "././."` collapses to `"."` on the next save.
- [ ] `respond_dir = "./responses"` does **not** collapse.
- [ ] `respond_dir = "responses"` does **not** change.
- [ ] No file is written by a command that was not already going to
      write. The repair rides along with a save; it is never standalone.

## D. Runtime behaviour is unchanged

- [ ] **A real request returns file-backed content** from a rule set
      with a `respond_dir`. Assert the response body, not the field —
      the field is what was wrong before, so it is not evidence.
- [ ] The same, from a rule set with **no** `respond_dir`.
- [ ] The same, run from a different working directory than the config
      lives in — this is the case the resolution exists for.
- [ ] A `respond_dir` pointing at a missing directory still fails
      `Prefix::validate`, with the same message as today.

## E. The breaking change

- [ ] `Prefix` is `#[non_exhaustive]`.
- [ ] A `compile_fail` doctest proves it is load-bearing (RFC 052's
      pattern) — remove the attribute locally and confirm the doctest
      **fails**, then restore it. Report that you did.
- [ ] **No other type gained the attribute.** `RuleSet`, `Rule` and
      `Respond` are out of scope; if you added it to any of them, say so.
- [ ] The migration guide gains an entry for `Prefix`.

## F. Gates

- [ ] Full suite green; count reported against `main`'s baseline.
- [ ] `cargo fmt --all --check` clean.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- [ ] `apimock get`, `set`, `validate` and `match-test` behave as before.
