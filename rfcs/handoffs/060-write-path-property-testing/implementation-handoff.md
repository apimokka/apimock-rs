# Implementation Handoff — RFC 060, property-test the write path

**Governing RFC.** [RFC 060](../../accepted/060-write-path-property-testing.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0. **No production code** — unless a property finds a bug
**Self-contained.** Everything you need is here. RFC 060 is the
authority; if the two disagree, report it rather than following this.

---

## 1. Why this exists — a bug that 22 passing tests said nothing about

RFC 058: `respond_dir` grew by one `./` segment on **every**
`Workspace::save()`, unbounded, in released code, hit by the GUI.

`workspace/tests/save.rs` had 22 tests at the time, including
`save_preserves_comments_blank_lines_and_key_order` and
`save_of_one_file_leaves_the_other_byte_identical`. **Every one of them
saved once.** The invariant that fails — *save twice and the file stops
changing* — was not expressed anywhere.

It was found by a person running `apimock set` five times and noticing
the file looked wrong. That is not a mechanism; it is luck. This RFC
builds the mechanism.

**Keep that story in mind while writing the generator.** The config
shape that broke was not exotic — a rule set with a `[prefix]` section,
saved more than once. The bug hid in the *number of saves*, not in the
config's complexity.

## 2. The four invariants

Stated over **all valid configs**, not particular ones:

1. **Idempotence.** `load → save → load → save` reaches a fixed point.
   The file stops changing.
2. **Preservation.** Anything the user wrote that the edit did not
   target comes back byte-identical — comments, blank lines, key order.
3. **Locality.** An edit to one file leaves every other file in the
   workspace byte-identical.
4. **Conflict safety.** A file changed underneath is refused, not
   overwritten, and **nothing partial is written**.

**Invariant 1 alone would have caught RFC 058** on the first generated
config carrying a `[prefix]` section.

## 3. The three open questions, decided

### Which dependency? **`proptest`, as a dev-dependency only.**

Shrinking on structured data is the deciding factor — a minimal
counter-example is the deliverable (§ 4), and `proptest` shrinks
composite values better than `quickcheck`.

**Dev-dependency only**, so nothing changes for anyone installing
apimock. Add it to the crate that needs it, not the workspace root, and
pin it the way every other dependency here is pinned.

If it pulls a transitive tree you think is unreasonable for this
project, **stop and say so** rather than adding it quietly — the
supply-chain posture is deliberate here and `cargo-deny` was dropped in
D-04, so nothing will catch a surprise for you.

### Before or after 6.0.0? **Before.**

Its entire value is finding the RFC-058-shaped bug that may still be in
there. Finding one after the release is strictly worse.

### Does locality extend across files? **Yes — all files in the write set.**

RFC 056 checks the whole write set before writing anything, so the
property should hold for every file, not just the edited one. That is
the guarantee the code claims; test the claim.

## 4. Requirements on the properties themselves

- **Generate valid configs from the schema**, not random bytes. The
  question is whether a legitimate config survives a legitimate edit,
  not whether the parser rejects garbage. Build from the same types
  `Workspace::load` produces, so the generator cannot drift into testing
  fiction.
- **Cover the shapes that actually vary:** presence and absence of
  `[prefix]`, comments in assorted positions, key order, multiple rule
  sets, header and body conditions, `respond` variants.
- **A failure must shrink to a minimal reproducing config and print
  it.** A property test reporting "failed on some input" is barely
  better than none. This is a requirement, not a nicety.
- **Fixed seed in CI**, so a failure is reproducible from the log alone.
- Keep the generator small and readable. Its value is the shrunk
  counter-example, not volume.

## 5. The 22 existing tests stay

They are regression tests for specific past defects —
`save_refuses_rather_than_overwrites_a_file_changed_on_disk` records a
real decision. Properties find the case nobody imagined; examples pin
the case that already bit. Keep both, unmodified.

## 6. Evidence required

- **The acceptance test is that it catches RFC 058.** Revert that fix
  locally (`respond_dir_prefix` resolved and written back), confirm the
  idempotence property **fails** *and* **shrinks to a config with a
  `[prefix]` section**, then restore the fix. **Report both halves.** If
  it does not catch the bug it was written for, it is not the right
  property, and a passing suite would be worse than no suite.
- Each of the four invariants has a property, and each runs enough cases
  to be meaningful.
- **CI runtime impact measured and reported.** RFC 031 balanced gate cost
  before; the same trade applies, and a slow gate gets skipped.
- The 22 existing tests pass unmodified.
- `cargo fmt --all --check`; `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`.

## 7. If the properties find bugs

**Report them; do not fix them in this change.** Each finding gets
triaged on its own — some may be pre-6.0.0 blockers, some may not, and
that is the owner's call with the finding in hand.

A property test that lands alongside three opportunistic fixes is
unreviewable in exactly the way RFC 043's handoff described. The
deliverable here is the mechanism plus a list of what it caught.

## 8. Escalation

Blocking issues and design questions go in a
`.git-exclude/review-request/` package.

Escalate if: `proptest` brings a dependency tree you would not choose;
the generator cannot express one of the four invariants without
duplicating production logic (that would make the test tautological);
or the properties find something serious enough that you think the
release should wait.
