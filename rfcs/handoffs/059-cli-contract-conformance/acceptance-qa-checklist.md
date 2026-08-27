# Acceptance / QA Checklist — RFC 059, CLI contract conformance

**Governing RFC.** [RFC 059](../../done/059-cli-contract-conformance.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md) —
the contract is restated in its § 3; you need no other RFC in hand.

## A. The four repro commands

- [ ] `apimock get /a --bogus` → exit **2**, kind `usage`, stderr only.
- [ ] `apimock validate -c cfg --bogus` → exit **2**. It currently exits
      0 and prints "Validation passed".
- [ ] `apimock match-test /a --bogus` → exit **2** *for the flag*, not
      exit 1 for a missing argument.
- [ ] `apimock set rule … --bogus` → exit **2** (already correct; assert
      it stays correct).

## B. The case that motivated the RFC

- [ ] **`apimock validate -c cfg --strct` exits 2** and the message
      suggests `--strict`.
- [ ] The same near-match suggestion works on every command, not just
      `validate`.

## C. The conformance table

- [ ] One table covers every command × every scenario in the handoff § 5c.
- [ ] Each row asserts **exit code, `kind`, and stream** — not just the
      exit code.
- [ ] Inapplicable scenarios are stated explicitly, never omitted.
- [ ] **The table is proven able to fail**: make a command ignore
      unknown flags again, confirm the suite catches it, restore. Report
      that you ran this.

## D. `match-test` joins the contract

- [ ] `match-test --format json` emits a valid envelope — object,
      `schema`, `apimock`, exactly one of `result`/`error`.
- [ ] `match-test`'s **text output is byte-identical** to before.
- [ ] Text remains the default.

## E. The shared harness

- [ ] One `bin()` / `run_json()` / `run_stderr()`; the four duplicates
      are deleted.
- [ ] Every pre-existing test in `get_format.rs`, `set_format.rs`,
      `validate_format.rs` and `args.rs` passes **unmodified** against it.

## F. Nothing else moved

- [ ] No command's success output changed.
- [ ] No exit code changed for an invocation that was already correct.
- [ ] RFC 053's `kind` strings are unchanged — this enforces the
      taxonomy, it does not revise it.

## G. Gates

- [ ] Full suite green; count reported against `main`'s baseline.
- [ ] `cargo fmt --all --check` clean.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
