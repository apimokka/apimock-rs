# RFC 056 — Preserve what people wrote: `toml_edit` for the save path

**Status.** Proposed — awaiting owner approval.
**Tracks.** v6. [RFC 048](./048-v6-cli-interface-concept.md) § 8 and
§ 11 item 1 — the prerequisite that makes `set`'s central claim true.
**Touches.** `crates/apimock-config/src/toml_writer.rs`, and — see § 2 —
`workspace.rs`'s baseline snapshot and `workspace/diff.rs`.
**Blocks.** `set` (RFC 048 item 5).

## Summary

`toml_writer` rewrites a configuration file from the in-memory model:
sorted keys, no comments, canonical quoting. Replace it with in-place
editing so that a person's comments and formatting survive a write.

## Motivation

### The claim `set` is going to make

RFC 048 promises `set` modifies configuration *"more safely than a
person editing TOML by hand."* A tool that silently deletes the
comments a person wrote does not clear that bar — it does something a
careful person would not do.

### Why it is the way it is, and why that stops applying

The writer's own module doc records the trade honestly:

> Per the GUI extension spec §6 (「コメント保持は best effort（必須要件
> ではない）」) and §11 (「完全なコメント保持」 is explicit non-goal),
> the save path is allowed to lose comments and key ordering. The
> `SaveResult` returned by `Workspace::save` includes an `Info`
> diagnostic noting this so the GUI can warn the user once.
>
> Future work could swap this module for `toml_edit` to preserve
> formatting; the public `Workspace::save` API would not change.

That was negotiated for **the GUI**, which owns the file during a session
and can surface one dialog. Two things have changed since:

- **A CLI has no dialog**, and **an agent never reads an `Info`
  diagnostic.** The mitigation that made the trade acceptable does not
  exist on the surface `set` will use.
- **G5 (2026-08-17): the GUI moves onto the v6 contract.** So the
  consumer the exception was written for ends up on the surface where
  the exception does not hold.

### The finding — canonical rendering is load-bearing beyond the save path

The module doc says swapping it "would not change the public
`Workspace::save` API". True, and not the whole picture. `render_*` has
**three** callers:

| Caller | Uses the rendering as |
|---|---|
| `workspace/save.rs` | the bytes written to disk |
| `workspace/diff.rs` | input to per-node diffing |
| `workspace.rs` | **the baseline for change detection** |

That third one is the problem, and `workspace.rs` explains why in its own
words:

> A naive baseline would store the literal on-disk text. But our writer
> produces canonicalised TOML … which almost never byte-matches a
> hand-edited file. With "on-disk" baseline, `has_unsaved_changes` would
> return `true` right after a load with no edits… Storing the *rendered*
> baseline solves this: a freshly loaded workspace has **rendered ==
> baseline by construction**.

**Canonicalisation is not an incidental property of the writer — it is
the mechanism that makes change detection work.** "Rendered == baseline
by construction" holds *because* rendering discards everything a human
might vary.

So this is not a module swap. It is a change to an invariant three
subsystems rest on, and the RFC exists to say that before someone
discovers it midway.

## Goals

1. A `save` that touches one value leaves every comment, key order and
   quoting style in that file otherwise intact.
2. Change detection keeps working — `has_unsaved_changes` stays `false`
   on a freshly loaded, unedited workspace.
3. `Workspace::save`'s public signature is unchanged.
4. The `Info` diagnostic warning about comment loss is **removed**, not
   left lying about a limitation that no longer exists.

## Non-goals

- Changing what is editable. The writer emits an *editable subset*
  deliberately; this RFC changes how it writes, not what.
- Rewriting `diff.rs`'s per-node model.
- `set` itself.
- Preserving formatting of files the user never edited — those are
  already untouched, because only diverging files are rewritten.

## Proposed design

Direction, not prescription — the implementer decides having read the
three call sites.

**Mutate a parsed document, do not rebuild one.** `toml_edit`'s value is
that it keeps a document's trivia — comments, spacing, key order — and
lets you set a value inside it. Building a fresh `toml_edit::Document`
from the model and serialising it would preserve nothing and would be
the current behaviour with a new dependency.

That implies the save path needs the **original text**, which it does not
take today. Establish from source what that costs.

**The baseline invariant needs a decision, not an accident.** With
formatting preserved, on-disk text becomes a plausible baseline again.
Two shapes, and the choice belongs in the review request with its
reasoning:

- **Keep a rendered baseline**, now produced by the same in-place path,
  so "rendered == baseline by construction" still holds.
- **Move to an on-disk baseline**, which becomes honest once rendering
  no longer canonicalises — and which would detect a file edited outside
  the session, something the current design cannot see.

The second is more capable and more disruptive. Neither is obviously
right; what is wrong is picking one silently.

## Dependency

`toml_edit` is a new dependency, and this project's standard is to
measure that rather than assert it. Report the added tree, and check it
against `cargo audit` — the actual gate (RFC 033's D-04 dropped
`cargo-deny`; do not repeat the claim that it applies).

Note the workspace already depends on `toml` `"1"`, and `toml_edit` is
the same project's format-preserving sibling — so this is closer to
enabling an adjacent capability than adding an unrelated tree.

## Testing and verification

- **A file with comments, blank lines and unusual key order survives a
  save that changes one value** — asserted on the file's bytes, with the
  comments still present. This is the whole point of the RFC and should
  be the first test written.
- `has_unsaved_changes` is `false` after load-with-no-edits, and `true`
  after one edit. The invariant, both directions.
- Only diverging files are rewritten — an untouched rule-set file is
  byte-identical after a save that changed a different file.
- The `Info` diagnostic about comment loss is gone, and nothing else in
  `SaveResult` changed shape.
- `diff.rs`'s per-node output is unchanged for the same edits.
- Full suite green; report the count against the baseline at the time.

## Risks

| Risk | Mitigation |
|---|---|
| The baseline invariant breaks quietly, and `save` starts rewriting every file | It is a named goal with a test in both directions, not an assumption |
| In-place editing needs the original text the save path does not have | Establish the cost from source before designing; if it is large, that is an escalation |
| Scope creeps into `diff.rs`'s model | Explicit non-goal; `diff.rs` consumes the rendering, it does not own it |
| A new dependency is waved through | Measured and reported, against `cargo audit` |

## Unresolved questions

1. **Rendered baseline or on-disk baseline?** § Proposed design. The
   more interesting half is that an on-disk baseline could detect
   external edits the current design cannot see — which is adjacent to
   G1's answer about mtime polling, and might overlap with what remains
   of RFC 042.
2. **Does `diff.rs` need in-place rendering at all?** It diffs *models*,
   and may not care about trivia. If it does not, its call site can keep
   using a canonical rendering and the change narrows considerably.
   Establish rather than assume.
3. **What happens to a file that is not valid TOML on disk but parsed
   into the model successfully?** Not currently possible — parsing is
   what produced the model — but in-place editing makes the question
   real, because it must re-parse the text it is editing.
