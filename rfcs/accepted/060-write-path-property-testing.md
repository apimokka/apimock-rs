# RFC 060 — Property-test the config write path

**Status.** **Accepted** — approved by the project owner 2026-08-20.
**Not yet implemented.**
[Handed off](../handoffs/060-write-path-property-testing/implementation-handoff.md) 2026-08-20,
with its open questions decided.
**Tracks.** Release quality; data integrity. Recommended before 6.0.0.
**Touches.** `crates/apimock-config/src/workspace/tests/` — new
property tests. No production code, unless a property finds a bug.
**Depends on.** [RFC 056](../accepted/056-toml-edit-migration.md) (the
write path), [RFC 058](../accepted/058-respond-dir-prefix-persistence.md)
(the bug that motivates this).

## Summary

Every test on the save path is example-based. RFC 058's defect survived
in released code because no example happened to save twice. Add
generated-input property tests for the invariants the write path
promises.

## Motivation

### The bug that should have been caught by a test nobody wrote

RFC 058: `respond_dir` grew by one `./` segment on **every**
`Workspace::save()`, unbounded, in released code, hit by the GUI.

`workspace/tests/save.rs` had **22 tests** at the time, including
`save_preserves_comments_blank_lines_and_key_order` and
`save_of_one_file_leaves_the_other_byte_identical`. Every one of them
saved **once**. The invariant that fails — *save twice and the file
stops changing* — was not expressed anywhere, so 22 passing tests said
nothing about it.

It was found by a human running `apimock set` five times in a row and
noticing the file looked odd. **That is not a repeatable quality
mechanism.**

### The shape of the gap

Example tests answer *"does this specific config survive?"* The write
path's real promises are universal:

- **Idempotence.** `load → save → load → save` reaches a fixed point.
- **Preservation.** Anything the user wrote that the edit did not target
  comes back byte-identical.
- **Locality.** An edit to one file leaves every other file untouched.
- **Conflict safety.** A file changed underneath is refused, not
  overwritten, and nothing partial is written.

Each is a statement about *all* valid configs. Testing them by example
tests them at whichever handful of configs someone thought of — and the
config shape that broke was the one nobody wrote down: a rule set with a
`[prefix]` section, saved more than once.

## Goals

1. Express the four invariants above as properties over generated
   configs.
2. Generation covers the shapes that actually vary: presence/absence of
   `[prefix]`, comments in assorted positions, key order, multiple rule
   sets, header and body conditions, `respond` variants.
3. A failing property **shrinks to a minimal reproducing config** and
   prints it — a property test that reports "failed on some input" is
   barely better than none.

## Non-goals

- Property-testing the matching engine or the CLI. Different surfaces,
  different invariants; this RFC is the write path only.
- Replacing the 22 example tests. They document specific past bugs and
  stay — see § Design.
- Fuzzing for panics on malformed input. Related and worth doing, but a
  separate question from "does a valid config survive a round trip".

## Design

### Properties, stated precisely

```
∀ valid workspace W, ∀ edit E applicable to W:
  save(apply(load(W), E))  then  save(load(·))  ⇒  byte-identical
```

and, for the untargeted remainder:

```
∀ W, E: every byte of W not semantically touched by E
        is present, unchanged, after save
```

**The first property alone would have caught RFC 058** on the first
generated config carrying a `[prefix]` section.

### Generator, not fuzzer

Inputs are **valid configs**, generated from the schema — not random
bytes. The question is whether a legitimate config survives a legitimate
edit, not whether the parser rejects garbage.

Keep the generator small and readable. A generator nobody understands
produces failures nobody can act on, and the value here is in the
shrunk counter-example, not in volume.

### The existing tests stay

The 22 examples in `save.rs` are regression tests for specific past
defects — `save_refuses_rather_than_overwrites_a_file_changed_on_disk`
records a real decision. Properties and examples answer different
questions: properties find the case nobody imagined, examples pin the
case that already bit. Keep both.

### Dependency

The workspace has **no property-testing dependency today** (`proptest`,
`quickcheck` and `arbitrary` are all absent). One is needed, and adding
it is a real decision for a project that has been careful here — see
§ Unresolved 1.

## Testing and verification

- **The properties fail against a deliberately reintroduced RFC 058.**
  Revert that fix locally, confirm the idempotence property fails and
  **shrinks to a config with a `[prefix]` section**, then restore. If it
  does not catch the bug it was written for, it is not the right
  property.
- Each property runs enough cases to be meaningful, with a fixed seed in
  CI so a failure is reproducible from the log alone.
- CI runtime impact is measured and reported. RFC 031 balanced gate cost
  before; the same trade applies.
- The 22 existing tests still pass, unmodified.

## Risks

| Risk | Mitigation |
|---|---|
| Flaky or slow CI | Fixed seed, bounded case count, runtime reported before merge |
| A generated failure nobody can act on | Shrinking is a Goal, not a nice-to-have; the deliverable is a minimal config |
| Generator drifts from the real schema and tests fiction | Generate from the same types `Workspace::load` produces, not a parallel model |
| **Properties find more bugs than we want to fix before 6.0.0** | That is the RFC working. Triage then, with the finding in hand — better than shipping and learning later |

## Unresolved questions

1. **Which dependency, and is a dev-dependency acceptable?**
   `proptest` has shrinking that works well on structured data;
   `quickcheck` is smaller and simpler. Both would be
   **dev-dependencies only** — no effect on what users install. Given
   D-04 dropped `cargo-deny`, the supply-chain posture is worth a
   sentence from the owner rather than my assumption.
2. **Before or after 6.0.0?** I recommend before, because its whole
   value is finding the RFC-058-shaped bug that is still in there. But
   it is the item most likely to *delay* the release by succeeding, and
   that trade is the owner's.
3. **Does the locality property extend across rule-set files?** RFC 056
   checks the whole write set before writing; whether "untouched file
   stays byte-identical" is asserted for every file or only the edited
   one changes how much the generator must build.
