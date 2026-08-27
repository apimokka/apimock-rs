# Acceptance / QA Checklist — RFC 056

**Governing RFC.** [RFC 056](../../done/056-toml-edit-migration.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## The point of the whole RFC

- [ ] A file with **comments, blank lines and hand-chosen key order**
      survives a save that changes one value
- [ ] Asserted on the file's **bytes**, with comments still present
- [ ] This test was written **first**

## The invariant that breaks quietly

- [ ] `has_unsaved_changes` **false** after load-with-no-edits
- [ ] `has_unsaved_changes` **true** after one edit
- [ ] Baseline stayed **rendered** (Q1); if it proved awkward, that was
      **escalated** rather than swapped for the on-disk design
- [ ] A save changing file A leaves file B **byte-identical**

## Conflict, not overwrite (Q3)

- [ ] What `save` does today with an externally-changed file
      **established from source**, not taken from the handoff
- [ ] A file changed on disk after load now **fails rather than
      overwrites**, demonstrated
- [ ] The behaviour change to `Workspace::save` **reported**, including
      what its signature became

## `diff.rs` (Q2)

- [ ] Whether it needs in-place rendering **established from source**
- [ ] Finding reported either way
- [ ] Its per-node output unchanged for the same edits

## Mechanism

- [ ] A parsed document is **mutated**, not rebuilt from the model
- [ ] Original text threaded through the save path; if that cost was
      large, it was escalated

## Dependency

- [ ] `toml_edit`'s added tree **measured and reported**
- [ ] Checked against **`cargo audit`** — not `cargo-deny`, which this
      project does not use (D-04)

## Scope held

- [ ] What is editable unchanged — only how it is written
- [ ] No `set` work; no external-change detection as a feature
- [ ] The comment-loss `Info` diagnostic **removed**, since it would
      describe a limitation that no longer exists
- [ ] `SaveResult` otherwise unchanged in shape

## Suite and gates

- [ ] Baseline measured **before** starting; new count reported against it
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
