# RFC 031 — CI quality gates on push and pull request

**Status.** Proposed
**Tracks.** M1 (Pipeline trust). No CI workflow currently runs
`cargo fmt --check`, `cargo clippy`, or `cargo test`. Every quality
claim in this project's history rests on manual session discipline.
**Touches.** `.github/workflows/` (new `ci.yaml`), `.github/CONTRIBUTING.md`.
No crate source, no public API, no runtime behaviour.

## Summary

Add a `ci.yaml` workflow that runs format, lint, test, and MSRV checks
on every push to `main` and every pull request, and make those checks
blocking. Retire `cargo test --workspace --lib` as the recorded gate in
favour of the full `cargo test --workspace`.

Depends on [RFC 030](./030-warning-clean-baseline.md): a blocking lint
gate cannot be installed while the lint fails.

## Motivation

The v5.14.0 handoff named this "the single most important operational
finding": the repository's only Rust-side CI is a release-time
`cargo build --release --locked`, triggered by GitHub's
`release: created` event. Nothing runs on push or pull request.

The cost of that gap is now measurable rather than theoretical:

- Strict clippy drifted from 21 findings to 26 in `apimock-routing`
  between sessions, with no RFC touching lint policy.
- The workspace-wide clippy total — invisible because `-D warnings`
  halts at the first failing crate — is 130 (RFC 030).
- `cargo fmt` compliance lapsed across the v5.8.0–v5.14.0 sessions and
  had to be repaired in bulk during handoff preparation (DEC-030).
- The recorded test gate has been silently running 212 of 371 tests.

Every one of those is a regression that a five-minute CI job would have
caught on the commit that introduced it. None were caught, because the
job does not exist.

## Guide-level explanation

A contributor opening a pull request sees four required checks:

| Check | Command |
|---|---|
| Format | `cargo fmt --all --check` |
| Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Test | `cargo test --workspace` |
| MSRV | `cargo check --workspace` on the pinned `rust-version` toolchain |

A red check blocks the merge. There is no override path short of the
maintainer changing the workflow, which is deliberate — an override
that is easy to reach is the same as no gate.

## Reference-level explanation

### New workflow: `.github/workflows/ci.yaml`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read
```

Four jobs. `fmt` and `clippy` install the matching components; `test`
runs the full suite; `msrv` pins the toolchain to the `rust-version`
value in `[workspace.package]`.

Design points:

- **`permissions: contents: read`.** The CI workflow needs no write
  scope. The existing `release-executable.yaml` and `docs.yaml` both
  declare `contents: write, id-token: write`; CI must not inherit that
  posture.
- **`concurrency` with `cancel-in-progress`.** Superseded pushes stop
  burning runner minutes.
- **Dependency caching** via `actions/cache` on `~/.cargo` and
  `target/`, keyed on `Cargo.lock` — matching the existing release
  workflow's approach.
- **Jobs run in parallel**, not as a chain. A format failure should not
  hide a test failure; a contributor deserves the full picture in one
  run.

### MSRV job

`Cargo.toml` pins `rust-version = "1.91.0"`, but
`release-executable.yaml` installs `stable`. Nothing verifies the pin.
A dependency that raises its own MSRV, or a language feature used
without noticing, would break `cargo install apimock` for anyone on the
documented minimum while CI stayed green.

The MSRV job installs exactly the pinned version and runs
`cargo check --workspace`. It does not run tests — dev-dependencies may
legitimately require a newer toolchain than the library itself.

**When the pin must change,** it changes in `Cargo.toml` as a deliberate
edit, and the job follows. That is the point: the pin becomes a
statement the build verifies rather than a comment.

### Retiring `--lib`

The mandatory test command becomes `cargo test --workspace`. Per
RFC 030's addendum, the full suite is 371 tests and currently passes;
`--lib` runs 212 of them. `--lib` remains a perfectly good local
fast-feedback command and CONTRIBUTING may say so — it is simply no
longer what "the tests pass" means for this project.

### Release workflow

`release-executable.yaml` gains a dependency on the CI checks so that a
release build cannot start while any gate is red. The release workflow
is otherwise unchanged in this RFC; its packaging defects are
[RFC 032](./032-release-and-packaging-repair.md)'s scope.

### CONTRIBUTING

A short "Before you open a PR" section listing the four commands, so a
contributor can reproduce every gate locally before pushing.

## Required tests

This RFC's deliverable is CI configuration; correctness is demonstrated
by the workflow running, not by unit tests.

Evidence required at review:

1. A CI run on a branch where **all four checks pass** — proving green
   is reachable.
2. A CI run on a deliberately broken branch (one formatting violation,
   one clippy finding, one failing test) where **each check fails
   independently and the merge is blocked** — proving the gate has
   teeth. This second run is mandatory; a gate observed only in its
   passing state has not been tested.
3. Confirmation that the release workflow refuses to start with a red
   gate.

## Acceptance criteria

1. `ci.yaml` exists and triggers on push to `main`, on pull request,
   and manually.
2. All four checks pass on `main` at the merge commit.
3. Each check has been observed failing independently and blocking.
4. `permissions` is `contents: read`.
5. The mandatory test command is `cargo test --workspace`; no
   documentation or workflow still presents `--lib` as the gate.
6. The MSRV job pins the toolchain to `[workspace.package].rust-version`.
7. `release-executable.yaml` cannot proceed while a gate is red.
8. CONTRIBUTING documents the four commands.

## Drawbacks

1. **Every future PR now costs CI minutes and wall-clock time.** For a
   project of this size that is a few minutes; the alternative has
   already been priced, and it cost 130 clippy findings and a bulk
   reformat.
2. **A blocking gate can block at an inconvenient moment.** That is the
   function. The mitigation is that all four commands run locally in
   under a minute after the first build.
3. **MSRV pinning creates a second toolchain to keep working.** Real
   cost, accepted: an unverified MSRV pin is worse than no pin, because
   it makes a promise to users that nothing checks.

## Rationale and alternatives

**Alternative A (chosen): blocking gates on push and PR.**

**Alternative B: advisory checks that report but do not block.**
Rejected for the reason recorded in RFC 030 — this project has just
demonstrated that unenforced signals are ignored across sessions.

**Alternative C: keep the release-time build as the only gate.** Status
quo. Rejected: it catches compile errors only, at the last possible
moment, after a release has already been created.

**Alternative D: a pre-commit hook instead of CI.** Rejected as a
replacement — hooks are per-clone, opt-in, and invisible to review.
Reasonable as a *supplement*; out of scope here.

## Unresolved questions

1. **Should `cargo test` also run on macOS and Windows?** The release
   matrix builds five targets, but the test suite has only ever been
   verified on Linux, and the integration tests bind real sockets and
   touch the filesystem — the two areas most likely to differ per
   platform. A cross-platform test matrix is defensible but multiplies
   CI cost. Recommend deferring to a follow-up RFC with evidence, not
   deciding it here.
2. **Should `Cargo.lock` freshness be checked** (`cargo update --locked
   --dry-run`)? Cheap, and it would catch a manifest edit that forgets
   the lockfile. Candidate for RFC 033 alongside the other
   dependency-hygiene checks.
