# Acceptance / QA Checklist — RFC 041, error type shape

**Governing RFC.** [RFC 041](../../done/041-error-type-shape.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md) —
decisions, sizes and the variant tables; you need nothing else in hand.

Evidence in the review-request package, not an assertion that it passes.

## A. The lint, which is the acceptance test

- [ ] **All 15 `#[allow(clippy::result_large_err)]` deleted.** Show
      `grep -rn result_large_err crates/` returning nothing.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
      clean.
- [ ] `cargo clippy --workspace --all-features -- --force-warn clippy::result_large_err`
      reports **zero** sites. Paste the (empty) output.
- [ ] Exactly **two** variants were boxed: `ConfigError::ConfigParse` and
      `RoutingError::RuleSetParse`. No `io::Error` variant was boxed.

## B. Nothing observable changed

- [ ] **Every error's `Display` string is byte-identical to before.**
      These reach users through diagnostics and `validate` output.
- [ ] `Error::source()` still reaches the underlying `toml::de::Error`
      through the box.
- [ ] `validate` on a malformed config produces the same message and the
      same exit code as before this change.
- [ ] A malformed rule set produces the same message as before.

## C. `kind()`

- [ ] All six enums have a `kind()` and a `#[non_exhaustive]` kind enum.
- [ ] Every variant of every enum maps to the right kind — one test per
      enum, covering all its variants.
- [ ] `WorkspaceError` has **its own** kinds (`Config`, `InvalidRoot`),
      not `ConfigError`'s.
- [ ] Kind enums live beside their errors, not in a shared module.
- [ ] **Nothing delegates to, or from, `envelope.rs`'s `ErrorKind`.**
      That is the CLI contract and stays separate.

## D. `#[non_exhaustive]` — across the surface

- [ ] All six error enums carry it.
- [ ] **Every re-exported public type carries it**, per the list derived
      from each crate's `lib.rs` (§ 3 of the handoff). Report the list.
- [ ] Structs with no public fields, and types that are `pub` but not
      re-exported, were correctly **excluded**.
- [ ] **Every type constructible from outside before is still
      constructible after.** Enumerate them with a construction path.
- [ ] `HeaderConditionPayload` and `BodyConditionPayload` gained
      construction (`Default` or `new()`); say which and why.
- [ ] `kind()` was added to the six error enums and **nothing else**.
- [ ] A `compile_fail` doctest proves it is load-bearing — remove the
      attribute, confirm the doctest **turns green when it should have
      stayed red**, restore it, confirm red again. Report that you ran
      this, not just that the doctest exists.
- [ ] A `compile_fail` doctest exists for **one error enum and one
      payload struct**, both verified load-bearing by removing the
      attribute and confirming the doctest wrongly turns green.

## E. Migration guide

- [ ] An entry for the boxing break, naming what stops compiling
      (constructing or destructuring the two parse variants).
- [ ] An entry for the `#[non_exhaustive]` break explaining the
      **pattern** — `default()` then assign for construction, a wildcard
      arm for matching, `kind()` for branching on errors — rather than
      one entry per type.

## F. Gates

- [ ] Full suite green; count reported against `main`'s baseline.
- [ ] `cargo fmt --all --check` clean.
- [ ] `get`, `set`, `validate` and `match-test` behave as before.
