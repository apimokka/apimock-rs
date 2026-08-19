# Acceptance / QA Checklist — RFC 041, error type shape

**Governing RFC.** [RFC 041](../../accepted/041-error-type-shape.md)
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

## D. `#[non_exhaustive]`

- [ ] All six error enums carry it.
- [ ] A `compile_fail` doctest proves it is load-bearing — remove the
      attribute, confirm the doctest **turns green when it should have
      stayed red**, restore it, confirm red again. Report that you ran
      this, not just that the doctest exists.
- [ ] **No non-error type gained the attribute.** `RuleSet`, `Rule` and
      `Respond` are R-09 work and out of scope; if you added it to any of
      them, say so.

## E. Migration guide

- [ ] An entry for the boxing break, naming what stops compiling
      (constructing or destructuring the two parse variants).
- [ ] An entry for the `#[non_exhaustive]` break, naming that exhaustive
      matches now need a wildcard arm, and pointing at `kind()` as the
      supported way to branch.

## F. Gates

- [ ] Full suite green; count reported against `main`'s baseline.
- [ ] `cargo fmt --all --check` clean.
- [ ] `get`, `set`, `validate` and `match-test` behave as before.
