# Acceptance / QA Checklist — RFC 042, external change detection

**Governing RFC.** [RFC 042](../../done/042-external-change-detection.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md) —
the defect, and what is deliberately not being built.

Evidence in the review-request package, not an assertion that it passes.

## A. The documentation is now true

- [ ] `sync_from_disk`'s doc comment says a sync is a fresh load, that
      every NodeId is reassigned, and that callers must re-query. Quote
      the before and after.
- [ ] The doc comment and the inline comment beneath it now agree. They
      contradicted each other; that is the defect.
- [ ] The `reseed_after_edit` forward reference is gone.
- [ ] A caller is told what to do after a sync: re-read the tree, do not
      re-use handles.

## B. Pinned by a test, not by prose

- [ ] **A test asserts NodeIds change across `sync_from_disk`.** This is
      the acceptance bar — the behaviour is now enforced, not described.
- [ ] `has_external_changes()` is `true` after an external write.
- [ ] `has_external_changes()` is `false` after a subsequent sync.
- [ ] A parse error during sync leaves the workspace fully usable —
      queryable, and a later successful sync still works.

## C. Nothing was built that G1 forbids

- [ ] **`Cargo.toml` and `Cargo.lock` are untouched.** No `notify`, no
      new dependency of any kind. Show the empty diff.
- [ ] No background thread, no watcher, no timer.
- [ ] `has_external_changes` and `sync_from_disk` keep their signatures
      and their behaviour. Only documentation changed.
- [ ] Node-identity preservation was **not** implemented.

## D. Gates

- [ ] Full suite green; count reported against `main`'s baseline.
- [ ] `cargo fmt --all --check` clean.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- [ ] The `has_external_changes` doctest still compiles.
