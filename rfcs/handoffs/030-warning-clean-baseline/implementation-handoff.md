# Implementation Handoff — RFC 030, Warning-clean baseline

**Governing RFC.** [RFC 030](../../done/030-warning-clean-baseline.md)
**Milestone.** M1 (Pipeline trust) → v5.15.0
**Status.** Inherited from RFC 030 (Proposed, approved for implementation
2026-08-02)
**Companion docs.** [`task-breakdown-pr-plan.md`](./task-breakdown-pr-plan.md) ·
[`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. Purpose

Make the workspace compile and lint clean, so that
[RFC 031](../../done/031-ci-quality-gates.md) can install a blocking
clippy gate. This is a prerequisite, not an improvement in its own right.

## 2. Background

Strict clippy has never passed on this workspace. Because `-D warnings`
halts the build at the first failing crate, the reported figure has
always been partial — the v5.14.0 handoff said "21 findings in
`apimock-routing`" and explicitly noted that coverage of the other three
crates was unknown.

It is now known. Measured 2026-08-02 on rustc 1.97.1:

| Crate | Clippy findings |
|---|---|
| `apimock` (incl. `tests/`) | 48 |
| `apimock-config` | 30 |
| `apimock-routing` | 27 |
| `apimock-server` | 17 |
| **Total** | **130** |

`apimock-routing` alone drifted from 21 to 26 lib findings between
sessions with no RFC touching lint policy — because nothing in CI
watches it. `cargo build` also emits three plain rustc warnings (two
unused imports, one unused variable).

## 3. Applicable requirements

From RFC 030: resolve every clippy finding and every rustc build
warning across all four crates, without changing behaviour, without
changing the public API, and without touching tests except where a lint
fires inside test code.

## 4. Change scope

- `crates/apimock/src/`, `crates/apimock/tests/`
- `crates/apimock-config/src/`
- `crates/apimock-routing/src/`
- `crates/apimock-server/src/`

Changes are limited to what a clippy or rustc diagnostic points at.

## 5. Explicit non-change scope

Do **not**:

- Modify, disable, `#[ignore]`, or delete any test assertion. If a lint
  fires inside a test, fix the test's *code*, never its assertions.
- Change any public API signature, `pub` item's type, or TOML schema.
- Introduce `unsafe`.
- Add or remove a dependency.
- Split any file — that is RFC 040's scope, even where a finding sits
  in `workspace/edit.rs` (822 ELOC) or `trace.rs` (514 ELOC).
- Add a blanket `#![allow]` at crate root or a `[lints]` table entry
  that silences a category workspace-wide.
- Reformat beyond what `cargo fmt` produces.

## 6. Required implementation

### Fix policy

1. **Default: fix the finding.** These are idiom lints; the idiomatic
   form is nearly always clearer.
2. **`#[allow(...)]` is a fallback.** Acceptable only when the suggested
   form is genuinely worse. Every `#[allow]` carries a
   `// clippy: <reason>` comment on the line above. An `#[allow]`
   without justification fails review.
3. **Suppression stays local and visible.** Item-level or
   expression-level only.

### Escalation trigger

`clippy::result_large_err` may want the `Err` variant boxed. **If the
error type involved is public** — anything re-exported from a crate's
`lib.rs`, notably `WorkspaceError`, `ApplyError`, `SaveError`,
`ConfigError`, `RoutingError`, `ServerError` — **stop and escalate.**
Public error types are part of the contract the GUI consumes; changing
one is a design decision, not a lint fix. Leave it, note it, and raise
a design request.

### Order

Four slices in dependency order, so cross-crate ripple surfaces early.
See [`task-breakdown-pr-plan.md`](./task-breakdown-pr-plan.md).

## 7. Required tests

**No new tests.** The existing suite is the regression harness; its job
is to prove that 130 idiom edits changed nothing.

**Baseline that must hold: 371 tests passing.** Note this is `cargo test
--workspace`, *not* `--workspace --lib`. The project's recorded gate has
been `--lib` (212 tests), which skips 48 integration-test files under
`crates/apimock/tests/`. The full suite currently passes at 371:

```
22 + 3 + 140 + 15 + 60 + 116 + 14 + 1 = 371
```

Per-crate counts must be **identical** before and after. A changed test
count in a lint-only change means something was touched that should not
have been.

## 8. Required documentation updates

None. This RFC has no user-visible effect. Do not update `docs/`,
`README.md`, or `CHANGELOG.md` — the CHANGELOG entry is written at
release-preparation time for the whole of M1.

## 9. Acceptance criteria

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   exits 0.
2. `cargo build --workspace --all-targets` emits zero warnings.
3. `cargo test --workspace` passes at 371, per-crate counts unchanged.
4. `cargo fmt --check` exits 0.
5. Every `#[allow]` introduced carries a justification comment.
6. Nothing in § 5 was touched.

## 10. Prohibited shortcuts

- Reaching for `#[allow]` because a fix is fiddly. The fix policy sets
  the bar; schedule pressure does not lower it.
- Silencing a lint category workspace-wide to clear a batch.
- "While I was in there" refactoring. Every hunk in the diff must be
  traceable to a diagnostic.
- Adjusting a test to accommodate a change in behaviour. If behaviour
  changed, the edit was wrong — revert it.

## 11. Compatibility constraints

The `Workspace` / `EditCommand` / view-type surface is additive-only
(DEC-014) and the GUI team depends on it. This RFC should not touch it
at all; if a lint appears to require touching it, that is the escalation
trigger in § 6.

## 12. Security constraints

No `unsafe`. No change to TLS handling in `apimock-server/src/tls.rs`
beyond what a lint diagnostic points at, and any such change gets called
out explicitly in the review request rather than folded into a bulk
commit.

## 13. Known risks

| Risk | Mitigation |
|---|---|
| One of 130 mechanical edits silently changes behaviour | Per-crate slicing; unchanged test counts; full suite green per slice |
| A lint fix cascades into a public API change | Escalation trigger in § 6 — stop, don't improvise |
| Reviewer fatigue across a large diff hides a real defect | Four separate review units, not one 130-finding PR |

## 14. Required evidence

Captured per slice, and again for the whole:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-targets       # capture output; must be empty
cargo test --workspace                      # capture the per-crate result lines
```

## 15. Required review-request format

Prepare a review-request package under
`.git-exclude/review-request/030-warning-clean-baseline/` with an entry
point that lets the reviewer start without other context. Per § 9.2 of
the workflow document it must include:

1. Implementation summary
2. Addressed requirements (map back to RFC 030)
3. Changed files
4. Important implementation decisions — **list every `#[allow]` added,
   with its justification**
5. Differences from the approved design
6. Executed tests and results (the 371 baseline, per-crate)
7. Build and static-analysis results
8. Unresolved issues — including anything hit by the § 6 escalation
   trigger
9. Known limitations
10. Requested review focus

Reviewer's focus will be: which hunks are *not* purely mechanical.
Make those easy to find.
