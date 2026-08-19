# RFC 042 — External change detection: correct the contract, don't build a watcher

**Status.** Proposed — awaiting owner approval.
**Tracks.** v6 API quality; GUI integration. **Rescoped** after the
owner's G1 answer (2026-08-17) removed its original premise.
**Touches.** `crates/apimock-config/src/workspace.rs` — chiefly its
documentation, plus `sync_from_disk`'s handling of node identity.
**Depends on.** Nothing. Interacts with
[RFC 057](../accepted/057-set-command.md)'s addressing finding.

## Summary

The reload machinery G1 asks for is already built. What is not correct
is what its documentation promises: `sync_from_disk` tells callers that
node identity survives a reload, and it does not. Fix the contract, and
stop there.

## Motivation

### What G1 removed

This RFC was originally "incremental reconciliation for
`sync_from_disk`" — make reloading cheaper so it could be done often.
The owner's G1 answer settled the question that premise rested on:

> app should not start to act differently triggered automatically. Also,
> app should not continue consuming machine resource to watch if change
> occurs.

So: **no filesystem watcher, no `notify` dependency, no automatic
behaviour change.** Detection is a poll; the response is to *ask*, not
to act.

A reload that happens only when a person confirms it does not happen
often, and something that does not happen often does not need to be
incremental. **G1 did not shrink this RFC; it deleted its reason for
existing.** What follows is what is actually left.

### What is already built

Both halves of G1's design exist and are public:

- `Workspace::has_external_changes()` (`workspace.rs:240`) — polls
  mtime + size against a snapshot taken at `load()`, returns `false` on
  stat errors to avoid spurious signals.
- `Workspace::sync_from_disk()` (`workspace.rs:272`) — reloads, and on
  parse error leaves the workspace untouched.

Its own doc comment shows the intended usage, and it is exactly G1's
shape — poll, then reload on the caller's initiative:

```rust
if ws.has_external_changes() {
    ws.sync_from_disk().unwrap();
}
```

Nothing here needs building. The *asking* belongs to the GUI and the
CLI, not to a library that must not act on its own.

### The defect: a public doc that contradicts its own implementation

`sync_from_disk`'s documentation states:

> NodeIds for unchanged addresses (same rule-set path, same rule index)
> are **preserved** across the reload. NodeIds for addresses that no
> longer exist are dropped; new addresses get fresh IDs.

The implementation is three lines long:

```rust
let fresh = Self::load(self.root_path.clone())?;
// NodeIDs are re-seeded from scratch; GUI callers should treat a sync
// like a fresh load and re-query all NodeIds from the new snapshot.
*self = fresh;
```

`Self::load` builds a `Workspace` with a `Default` (empty) `IdIndex` and
`seed_ids()` mints a fresh UUID per address. `*self = fresh` then discards
the old index entirely. **No identity is preserved.** The inline comment
says so, directly beneath the doc comment that says the opposite.

A GUI author reading the public documentation would cache NodeIds across
a sync and hold stale handles — silently, because a UUID that no longer
maps to anything looks exactly like one that does until it is used.

The mechanism the doc describes was to be `reseed_after_edit`, named in
`seed_ids`'s comment as *"Step 2 will call a more careful
`reseed_after_edit`"*. **It appears nowhere in the codebase** — the
reference in that comment is its only occurrence.

This is the same family as RFC 057's finding: `NodeId` is
session-scoped, and documentation that implies otherwise causes silent
breakage rather than loud failure.

## Goals

1. Make `sync_from_disk`'s documentation true.
2. Decide, explicitly, whether identity preservation is worth building —
   and record the answer either way.
3. Leave `has_external_changes()` as it is. It already does G1's job.

## Non-goals

- A filesystem watcher, `notify`, or any background thread. G1.
- Automatic reload. G1: the app must not act differently on its own.
- Incremental reconciliation. G1 removed the premise; see § Motivation.
- Reworking `NodeId` into a durable identifier. RFC 057 settles that a
  CLI must not use it at all; a GUI holding one for a session is fine.

## Design

Two options, and this RFC recommends the first.

### Option A — correct the documentation (recommended)

State what happens: a sync is a fresh load, all NodeIds are reassigned,
and callers must re-query. Delete the `reseed_after_edit` reference,
which describes work nobody is scheduled to do.

Add the note the GUI actually needs: after a sync, re-read the tree
rather than re-using handles. That is a one-line habit, and the GUI is
re-rendering anyway — it just called `has_external_changes()`.

**Cost: a documentation change. Benefit: the API stops lying.**

### Option B — implement preservation

Make `sync_from_disk` reuse the existing `address_to_id` map for
addresses that still exist, minting IDs only for new ones. `IdIndex::insert`
already returns the existing id when the address is present
(`id_index.rs:63`), so the machinery is close: the work is to carry the
old index into the fresh workspace rather than discard it.

**But note what "unchanged address" means.** `NodeAddress` is positional
(`Rule { rule_set: usize, rule: usize }`). An external edit that inserts
a rule mid-file shifts every later index, so preservation by address
would *reassign* identity for rules a person would say did not change —
and, worse, would *preserve* an identity onto a different rule that
inherited the index. Option B is not obviously safer than Option A; it
is safer only if addresses are stable, and an external edit is exactly
the case where they are not.

**Recommendation: Option A.** Preservation is a real feature with a real
cost and a subtle failure mode, wanted by no user we have heard from —
G2 says the GUI is primarily read-only. Documenting the truth costs
nothing and removes the trap today.

## Testing and verification

- A test asserting that NodeIds **do** change across `sync_from_disk`,
  so the documented behaviour is pinned by a test rather than by prose.
  If Option B is ever chosen, that test is the thing that flips.
- `has_external_changes()` returns `true` after an external write and
  `false` after a subsequent sync.
- A parse error during sync leaves the workspace fully usable — already
  claimed by the docs; worth an assertion.
- No new dependency appears in `Cargo.toml`. G1 as a test.

## Risks

| Risk | Mitigation |
|---|---|
| Option A reads as giving up on a feature | It is a decision, recorded, with the cost of the alternative stated; Option B stays available |
| The GUI already relies on the documented-but-false behaviour | Worth asking the GUI team directly — if they cache NodeIds across a sync, they have a live bug and this RFC found it |
| Someone re-adds a watcher later "for convenience" | G1 is recorded in ROADMAP and restated here as a non-goal |

## Unresolved questions

1. **Does the GUI currently cache NodeIds across a sync?** If yes, this
   stops being a documentation RFC and becomes a bug report with a fix
   attached. Ask before implementing.
2. **Should `has_external_changes()` distinguish "changed" from
   "deleted"?** It returns `false` on stat error, so a deleted file
   reads as unchanged. Defensible for temp-file churn, wrong for a file
   a person removed. G1's "ask the user" response needs to know which
   happened — but the CLI has no caller for this yet, so the question
   can wait for one.
