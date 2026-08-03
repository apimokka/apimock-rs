# RFC 030 — Warning-clean baseline across the workspace

**Status.** Implemented (v5.15.0)
**Tracks.** M1 (Pipeline trust). Strict clippy has never passed on this
workspace, and because `-D warnings` halts the build at the first
crate that fails, the true size of the problem was unknown until now.
This RFC establishes a warning-clean baseline so that RFC 031 can gate
on it.
**Touches.** All four crates' `src/` trees and `crates/apimock/tests/`.
No public API, no TOML format, no runtime behaviour.

## Summary

Resolve every `cargo clippy --workspace --all-targets --all-features`
finding and every `rustc` build warning, so that the workspace compiles
and lints clean. Findings are fixed, not suppressed; `#[allow]` is
permitted only where a fix would be worse than the lint, and each one
carries a written justification.

This RFC produces no behaviour change. It is a prerequisite for
[RFC 031](./031-ci-quality-gates.md), which cannot gate on a lint that
does not pass.

## Motivation

The v5.14.0 handoff reported "21 findings in `apimock-routing`" and
noted that clippy "never reaches the other 3 crates because the
workspace build halts". Re-measuring on 2026-08-02 produced two facts
that change the shape of this work:

1. **The count is growing unobserved.** `apimock-routing` alone now
   produces 26 lib findings (27 including lib tests), up from 21, with
   no intervening RFC touching lint policy. Nothing in CI would have
   surfaced this.
2. **The real scope is roughly five times what was reported.** Running
   clippy *without* `-D warnings` lets the build continue past
   `apimock-routing`. The workspace-wide total is **130 findings**:

   | Crate | Findings |
   |---|---|
   | `apimock` (incl. `tests/`) | 48 |
   | `apimock-config` | 30 |
   | `apimock-routing` | 27 |
   | `apimock-server` | 17 |
   | **Total** | **130** |

Additionally, `cargo build` itself is not warning-free — it emits at
least three plain `rustc` warnings (two unused imports, one unused
variable), one of which clippy also reports as an error under
`-D warnings`.

The point of this RFC is not tidiness. It is that a lint gate nobody
can turn on provides no signal, and the drift from 21 to 26 to a
now-visible 130 is direct evidence that unenforced discipline does not
hold across sessions.

## Guide-level explanation

After this RFC:

```bash
cargo build --workspace --all-targets              # no warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings   # exits 0
```

Nothing a user of apimock observes changes. No config file, response,
CLI flag, or library signature is altered.

## Reference-level explanation

### Scope

All 130 clippy findings plus all `rustc` build warnings, across:

- `crates/apimock/src/` and `crates/apimock/tests/`
- `crates/apimock-config/src/`
- `crates/apimock-routing/src/`
- `crates/apimock-server/src/`

The finding mix is idiom-level. The recurring categories observed:
derivable `impl` (×4), `map_or` simplification (×3), overindented doc
list items (×3), collapsible `if` (×2), plus single instances of
`unwrap` after `is_some` (×3 across different fields), manual
`Option::map`, `Iterator::fold` on a `Try` type, needless reference,
large `Err` variant, unused import, and others.

### Fix policy

1. **Default: fix the finding.** These are idiom lints; the idiomatic
   form is nearly always the clearer one.
2. **`#[allow(...)]` is a fallback, not a shortcut.** It is acceptable
   only when the lint's suggested form is genuinely worse — measurably
   less readable, or wrong for the surrounding code. Every `#[allow]`
   carries a `// clippy: <reason>` comment on the line above. An
   `#[allow]` without a justification comment fails review.
3. **No lint is disabled workspace-wide.** No blanket `#![allow]` at
   crate root, and no `[lints]` table entry that silences a category
   across the workspace. Suppression stays local and visible.
4. **`clippy::result_large_err`** is the one finding likely to warrant
   a structural fix rather than a suppression — boxing the large `Err`
   variant changes an error type's shape. If that turns out to touch a
   public error type, it is **out of scope for this RFC** and must be
   escalated as a separate change request, because error types are part
   of the API surface the GUI consumes.

### Explicit non-change scope

- No test may be modified, disabled, or deleted to make a lint pass.
  If a lint fires inside a test, the fix is to the test's code, never
  to its assertions.
- No public API signature, no `pub` item's type, no TOML schema.
- No `unsafe` may be introduced.
- No dependency added or removed.
- No file split — that is RFC 040's scope, even where clippy's finding
  sits in one of the oversized files.

### Slicing

Four independently reviewable slices, one per crate, in dependency
order so that any cross-crate ripple surfaces early:

1. `apimock-routing` (27)
2. `apimock-config` (30)
3. `apimock-server` (17)
4. `apimock` incl. `tests/` (48)

Each slice is a separate review unit. A slice is complete when clippy
is clean *for that crate* and the full test suite still passes.

## Required tests

No new tests. The existing suite is the regression harness — its job
here is to prove that 130 idiom edits changed nothing.

Evidence required at review:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-targets      # captured to show zero warnings
cargo test --workspace                     # not --lib; see Unresolved questions
```

Test counts must be **identical** before and after, per crate. A
changed test count in a lint-only RFC means something was modified that
should not have been.

## Acceptance criteria

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   exits 0.
2. `cargo build --workspace --all-targets` emits zero warnings.
3. `cargo test --workspace` passes, with per-crate test counts
   unchanged from the pre-RFC baseline.
4. `cargo fmt --check` exits 0.
5. Every `#[allow]` introduced carries a justification comment.
6. No item in the non-change scope above was touched.

## Drawbacks

130 mechanical edits across four crates is a large diff for zero
functional gain, and a large diff is a place where a real mistake can
hide. This is why the RFC is sliced per crate, forbids test changes,
and requires unchanged test counts — the review is looking for the one
edit among 130 that was not mechanical.

## Rationale and alternatives

**Alternative A (chosen): fix all, then gate.** Highest confidence.
The gate that follows is meaningful from day one.

**Alternative B: fix most, `#[allow]` the remainder.** Faster to green.
Rejected as the default because a suppression written under schedule
pressure is indistinguishable from one written after analysis, and the
project has just demonstrated that invisible debt accumulates. Retained
as a per-finding fallback under the fix policy above.

**Alternative C: advisory (non-blocking) gate first.** Lowest
disruption. Rejected: an advisory gate on a codebase with 130 findings
is a wall of noise nobody reads, and it would not have caught the
21→26 drift either.

The owner selected A on 2026-08-02.

## Unresolved questions

1. **Does `clippy::result_large_err` touch a public error type?** If
   so, it leaves this RFC's scope per the fix policy above.

## Addendum — the recorded test gate was understating coverage

Resolved during this RFC's drafting, on 2026-08-02.

The project's recorded gate has been `cargo test --workspace --lib`,
reported as 212 passing. That command skips the 48 integration-test
files under `crates/apimock/tests/`. Running the full suite:

```
cargo test --workspace     → 371 passed; 0 failed  (exit 0)
```

Breakdown: 22 + 3 + 140 + 15 + 60 + 116 + 14 + 1. The 159 tests the
`--lib` gate never ran **all pass** — so this is not a hidden defect,
it is a hidden *asset*: the project has substantially better test
coverage than its own release evidence has been claiming.

Two consequences:

- The baseline this RFC must hold unchanged is **371**, not 212.
- [RFC 031](./031-ci-quality-gates.md) makes `cargo test --workspace`
  the mandatory gate. `--lib` is retired; it appears to have been an
  accident of a fast-feedback command becoming the recorded one, not a
  deliberate scope decision.
