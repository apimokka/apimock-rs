# Implementation Handoff — RFC 056, `toml_edit` for the save path

**Governing RFC.** [RFC 056](../../done/056-toml-edit-migration.md)
**Milestone.** 6.0.0 — RFC 048 § 11 item 1. **Blocks `set`.**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

`toml_writer` rewrites a configuration file from the in-memory model:
sorted keys, comments gone, canonical quoting. Make a save that changes
one value leave everything else in that file alone.

**This is what makes `set`'s claim honest.** RFC 048 promises `set`
edits configuration *"more safely than a person editing TOML by hand"*.
Deleting the comments that person wrote is not that.

## 2. Two of the three open questions are decided

### Q1 — rendered baseline or on-disk baseline? → **Keep the rendered baseline.**

`workspace.rs` stores a *rendered* baseline so that a freshly loaded
workspace has `rendered == baseline` by construction. Once rendering
preserves formatting, an on-disk baseline becomes plausible — and it
would detect edits made outside the session, which the current design
cannot see.

**Do not take that here.** It conflates two things:

- **preserving formatting** — this RFC, and
- **detecting external change** — which G1 already answered
  differently: a boot-time file list, an existence/mtime poll, and a
  *confirmation prompt*, not a text comparison.

Those are different mechanisms with different costs, and the second
belongs with what remains of RFC 042. Keep the baseline rendered, now
produced by the new path, so the invariant survives unchanged.

If in-place editing makes the rendered baseline awkward, that is a
finding worth escalating — not a licence to swap in the other design.

### Q3 — a file whose on-disk text changed since load? → **Refuse. Do not overwrite.**

In-place editing must re-read the text it edits, so unlike today's path
it *will* notice a file that changed underneath it. That makes this
unavoidable rather than optional.

**Establish first what `save` does today** in that situation — I believe
it overwrites, because it renders from the model and never looks, but
check rather than take my word.

Then: if the text changed since load, **fail with a conflict rather than
overwriting**. Silently discarding someone's external edit is the worst
available outcome, and RFC 053 already reserved `error.kind: "conflict"`
for exactly this.

**This is a behaviour change to `Workspace::save`** — report it plainly
in your review request, including what its signature has to become to
express the failure.

### Q2 — does `diff.rs` need in-place rendering? → **Establish from source.**

It diffs *models*, so it may not care about trivia at all. If it does
not, its call site keeps a canonical rendering and this change narrows
considerably — which would be the best outcome available. **Find out
before designing**, and say what you found either way.

## 3. Mutate a document; do not rebuild one

`toml_edit`'s value is that it keeps a parsed document's comments,
spacing and key order while you set a value inside it. Building a fresh
document from the model and serialising it preserves nothing — that is
today's behaviour with a new dependency, and it is the way this work
fails without anyone noticing.

So the save path needs the **original text**, which it does not take
today. Establish what that costs before designing around it.

## 4. The dependency

`toml_edit` is new. This project measures dependencies rather than
asserting them: report the added tree and check it against **`cargo
audit`** — the actual gate. Do **not** repeat the claim that
`cargo-deny` applies; owner decision D-04 dropped it on 2026-08-02, and
a handoff of mine asserted otherwise once already.

The workspace already depends on `toml "1"`, and `toml_edit` is the same
project's format-preserving sibling, so this is closer to enabling an
adjacent capability than adding an unrelated tree. Confirm that rather
than assume it.

## 5. Scope boundaries

- **In:** `toml_writer.rs`, the save path, and whatever `workspace.rs`
  needs to keep its invariant.
- **Out:** what is editable — the writer emits a deliberate editable
  subset; this changes *how* it writes, not *what*. `diff.rs`'s per-node
  model. `set` itself. External-change detection as a feature (that is
  RFC 042's remnant).
- **Out:** removing the `Info` diagnostic's *mechanism* — but **do**
  remove the comment-loss diagnostic itself, since it will be describing
  a limitation that no longer exists.

## 6. Evidence required

- **Write the first test first:** a file with comments, blank lines and
  hand-chosen key order survives a save that changes one value —
  asserted on the file's **bytes**, with the comments still present.
  Everything else in this RFC is in service of that assertion.
- `has_unsaved_changes` is `false` after load-with-no-edits and `true`
  after one edit. Both directions — the invariant is the thing most
  likely to break quietly.
- A save that changes file A leaves file B **byte-identical**.
- The comment-loss `Info` diagnostic is gone; `SaveResult` is otherwise
  unchanged in shape.
- Conflict behaviour (Q3) demonstrated: change a file on disk after
  load, save, and show it **fails rather than overwrites**.
- `diff.rs`'s per-node output unchanged for the same edits.
- Full suite green; report the count against the baseline you measured
  before starting.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 7. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package — including a rendered baseline
that proves awkward (§ 2 Q1), and the cost of threading original text
through the save path if it turns out to be large.
