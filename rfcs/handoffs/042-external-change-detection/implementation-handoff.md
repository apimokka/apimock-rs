# Implementation Handoff — RFC 042, external change detection

**Governing RFC.** [RFC 042](../../accepted/042-external-change-detection.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0. **Non-breaking** — no signature changes
**Self-contained.** Everything you need is here. RFC 042 is the
authority; if the two disagree, report it rather than following this.

---

## 1. Read this first — this is a documentation fix, and that is the point

You are not building a feature. The feature exists. **The public
documentation on it is false**, and correcting it is the whole job.

Expect this to be a small change. Do not go looking for more to do; if
you find yourself implementing node-identity preservation, you have
crossed into § 3, which is explicitly not this task.

## 2. The defect

`Workspace::sync_from_disk`'s doc comment
(`crates/apimock-config/src/workspace.rs:260`) states:

> NodeIds for unchanged addresses (same rule-set path, same rule index)
> are **preserved** across the reload. NodeIds for addresses that no
> longer exist are dropped; new addresses get fresh IDs.

The implementation is three lines (`workspace.rs:273`):

```rust
pub fn sync_from_disk(&mut self) -> Result<(), WorkspaceError> {
    let fresh = Self::load(self.root_path.clone())?;
    // Replace the entire workspace state. NodeIDs are re-seeded from
    // scratch; GUI callers should treat a sync like a fresh load and
    // re-query all NodeIds from the new snapshot.
    *self = fresh;
    Ok(())
}
```

`Self::load` builds a `Workspace` with a `Default` (empty) `IdIndex`, and
`seed_ids()` mints a fresh UUID per address. `*self = fresh` then
discards the old index entirely. **No identity is preserved.**

The inline comment says exactly that, **directly beneath the doc comment
that says the opposite**. One of them is wrong, and it is the public one.

The mechanism the doc describes was to be `reseed_after_edit`, named at
`workspace.rs:297` as *"Step 2 will call a more careful
`reseed_after_edit`"*. **That is its only occurrence in the codebase** —
it was never built.

**Why it matters:** a GUI author reading the public documentation caches
NodeIds across a sync and holds stale handles. It fails silently, because
a UUID that no longer maps to anything looks exactly like one that does
until it is used.

## 3. What is deliberately NOT being built

The owner's **G1** answer (2026-08-17) settled the design question this
RFC originally existed for:

> app should not start to act differently triggered automatically. Also,
> app should not continue consuming machine resource to watch if change
> occurs.

So: **no filesystem watcher, no `notify` dependency, no background
thread, no automatic reload.** Detection is a poll; the response is to
*ask the user*, and the asking belongs to the GUI or CLI, not to a
library that must not act on its own.

Both halves already exist and are public:

- `Workspace::has_external_changes()` (`workspace.rs:241`) — polls mtime
  and size against a snapshot taken at `load()`, returning `false` on
  stat errors to avoid spurious signals from temp-file churn.
- `Workspace::sync_from_disk()` (`workspace.rs:273`) — reloads, leaving
  the workspace untouched on parse error.

**Leave both alone.** `has_external_changes` already does G1's job.

**Identity preservation is not being implemented.** RFC 042 § Design
weighed it and chose to document the truth instead, because
`NodeAddress` is positional (`Rule { rule_set: usize, rule: usize }`) and
an external edit that inserts a rule mid-file shifts every later index —
so preservation "by address" would reassign identity for rules a person
would say did not change, and *preserve* an identity onto a different
rule that inherited the index. It is not obviously safer than saying
nothing; it is only safer when addresses are stable, and an external edit
is precisely when they are not.

## 4. The work

**a. Make the doc comment true.** State what happens: a sync is a fresh
load, every NodeId is reassigned, callers must re-query. Say it in the
doc comment, where the false claim is now.

**b. Delete the `reseed_after_edit` reference** at `workspace.rs:297`.
It describes work nobody is scheduled to do, and a forward reference to
a function that does not exist reads as a plan rather than a fiction.

**c. Add the note the caller actually needs.** After a sync, re-read the
tree rather than re-using handles. That costs a GUI nothing — it just
called `has_external_changes()` and is re-rendering anyway.

**d. Pin it with a test.** See § 5 — the test is what stops this
regressing into prose again.

## 5. Evidence required

- **A test asserting NodeIds *do* change across `sync_from_disk`.** This
  is the important one: it pins the documented behaviour to an assertion
  instead of a sentence. If identity preservation is ever built, this is
  the test that flips.
- `has_external_changes()` returns `true` after an external write, and
  `false` after a subsequent `sync_from_disk()`.
- A parse error during sync leaves the workspace **fully usable** — the
  docs already claim this; assert it.
- **No new dependency in any `Cargo.toml`.** G1 as a test — a diff
  showing `Cargo.toml` and `Cargo.lock` untouched is the evidence.
- The doctest on `has_external_changes` still compiles (it is a
  `rust,no_run` example and is already part of the suite).
- Full suite green with the count against `main`'s baseline;
  `cargo fmt --all --check`; `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`.

## 6. One question I could not answer for you

**Does the GUI currently cache NodeIds across a sync?** If it does, it
has a live bug and this RFC found it — and that changes this from a
documentation fix into a bug report with a fix attached.

I cannot answer it from this repository, and it is being asked
separately. **Do not block on it**: the documentation is false either
way, and correcting it is right either way. Mention it in your
submission if you see anything relevant.

## 7. Escalation

Blocking issues and design questions go in a
`.git-exclude/review-request/` package.

Escalate if: preserving identity turns out to be trivial *and* free of
the positional-address problem in § 3 (that would reopen a decision I
made, and I would want to know); or if `has_external_changes` proves
wrong in some way while you are testing it — it is out of scope to
change, but not out of scope to report.
