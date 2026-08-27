# Acceptance / QA Checklist — RFC 060, write-path property testing

**Governing RFC.** [RFC 060](../../done/060-write-path-property-testing.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md) —
the invariants and the decisions; nothing else needed in hand.

## A. The acceptance test — does it catch the bug it was written for

- [ ] RFC 058's fix reverted locally, the **idempotence property fails**.
- [ ] The failure **shrinks to a minimal config carrying a `[prefix]`
      section**, and the shrunk config is printed.
- [ ] Fix restored; property passes again.
- [ ] **Both halves reported** — failing, and shrinking. A property that
      fails without shrinking has not met the bar.

## B. The four invariants

- [ ] **Idempotence** — `load → save → load → save` is a fixed point.
- [ ] **Preservation** — untargeted comments, blank lines and key order
      survive byte-identically.
- [ ] **Locality** — an edit to one file leaves **every** other file in
      the write set byte-identical, not just the obvious neighbour.
- [ ] **Conflict safety** — a file changed underneath is refused, and
      **nothing partial is written**.

## C. The generator

- [ ] Generates **valid configs from the schema**, built from the same
      types `Workspace::load` produces — not random bytes, not a
      parallel model.
- [ ] Covers: `[prefix]` present *and* absent, comments in assorted
      positions, key order, multiple rule sets, header and body
      conditions, `respond` variants.
- [ ] Small enough to read and understand.

## D. Fit for CI

- [ ] Fixed seed, so a failure is reproducible from the log alone.
- [ ] **CI runtime impact measured and reported** as a number.
- [ ] `proptest` is a **dev-dependency only**; nothing changes for
      anyone installing apimock. Report its transitive tree.

## E. Scope

- [ ] The 22 existing tests in `save.rs` pass **unmodified**.
- [ ] **No production code changed.** If a property found a bug, it is
      reported, not fixed here.
- [ ] Any findings are listed with enough detail to triage — minimal
      config, which invariant, what happened.

## F. Gates

- [ ] Full suite green; count reported against `main`'s baseline.
- [ ] `cargo fmt --all --check` clean.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
