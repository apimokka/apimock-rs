# RFC 061 — Test on the platforms we ship

**Status.** **Accepted** — approved by the project owner 2026-08-20.
**Not yet implemented.**
[Handed off](../handoffs/061-cross-platform-ci/implementation-handoff.md) 2026-08-20,
with its open questions decided. Blocking for 6.0.0.
**Tracks.** Release quality. **Blocking for 6.0.0** in my view.
**Touches.** `.github/workflows/ci.yaml`. No production code.
**Depends on.** Nothing.

## Summary

CI runs on `ubuntu-latest` only, across all six jobs. The release
workflow ships **Windows and macOS** binaries. **No test has ever run on
either platform.** Add them to the test matrix before 6.0.0.

## Motivation

### The gap, measured

Every job in `ci.yaml` — `fmt`, `clippy`, `test`, `msrv`, `audit`,
`lockfile` — is `runs-on: ubuntu-latest`.

`release-executable.yaml` builds five targets:

```
aarch64-unknown-linux-musl · x86_64-unknown-linux-gnu
x86_64-unknown-linux-musl  · aarch64-apple-darwin
x86_64-pc-windows-msvc
```

and its build step is `cargo build --release --target … --locked`.
**Build only — no `cargo test` on any of them.**

So macOS and Windows users install binaries on which not one assertion
has ever executed. Compilation is the entire guarantee.

### Why this became urgent at v6 rather than earlier

v5 got away with it because the server **read** files. A path bug on
Windows meant a 404 or a startup failure — visible, local, recoverable.

**v6 writes.** `apimock set` creates `apimock.toml`, creates rule-set
files, rewrites existing ones, and resolves `--rule-set` paths against
`service.rule_sets`. A path-handling bug on Windows now edits the
user's files, and RFC 056's whole promise is that it edits them
faithfully.

Concretely, code shipped this cycle that is *about* path semantics and
has never run on Windows:

- RFC 058's `is_purely_current_dir`, written with `Path::components()`
  **specifically** to be correct on Windows — reasoned about carefully,
  never executed there.
- `set`'s rule-set path matching, which canonicalises and falls back
  when the file does not exist yet.
- The bootstrap path that decides `./apimock.toml` versus
  `apimock.toml`, where a missing `./` already caused one real defect
  (RFC 057's review, § 3).
- **The W7 acceptance script — the definition of v6 being complete —
  has never run on Windows.**

Each of those is correct as far as anyone knows. Nobody knows.

## Goals

1. `cargo test --workspace` runs on Windows and macOS in CI.
2. The W7 acceptance script runs on all three.
3. Failures found this way are fixed or explicitly accepted **before**
   6.0.0, not discovered by a user afterwards.

## Non-goals

- Testing every release target. `x86_64-pc-windows-msvc` and
  `aarch64-apple-darwin` are what GitHub runners offer; the musl targets
  stay build-only.
- Running the full six-job matrix everywhere. `fmt`, `clippy`, `msrv`,
  `audit` and `lockfile` are platform-independent and stay on Ubuntu.
- Cross-compilation testing, emulation, or self-hosted runners.

## Design

Add a matrix to the **`test` job only**:

```yaml
test:
  strategy:
    fail-fast: false
    matrix:
      os: [ubuntu-latest, windows-latest, macos-latest]
  runs-on: ${{ matrix.os }}
```

`fail-fast: false` matters — RFC 047's `verify-published` needed the
same thing for the same reason. One platform failing must not hide
whether the others pass.

Everything else stays as it is.

### What to expect

**Assume this goes red on the first run.** Likely candidates: path
separators in test fixtures and assertions, line endings in the
byte-identity comparisons RFC 056 and RFC 060 depend on, temp-directory
semantics, and file-locking behaviour on Windows where a save rewrites a
file another handle holds.

A red first run is the RFC succeeding, not failing. The findings are the
deliverable.

## Testing and verification

- The `test` job passes on all three platforms — or its failures are
  listed, triaged, and each one fixed or explicitly accepted with a
  reason.
- The W7 script runs green on all three.
- **CI wall-clock impact reported.** Three platforms is roughly three
  times the test-job cost; if that is unacceptable the answer is a
  narrower trigger, not dropping the platforms.
- `fail-fast: false` verified by observing that one platform's failure
  does not cancel the others.

## Risks

| Risk | Mitigation |
|---|---|
| **Windows failures delay 6.0.0** | That is the point. A defect found now costs a delay; found later it costs a user's config file |
| CI cost and wall-clock triples for the test job | Measured and reported; narrow the trigger if needed rather than dropping coverage |
| Windows-only failures nobody can reproduce locally | The runner log is the reproduction; if a fix cannot be verified, say so rather than guessing |
| Flaky Windows file-locking makes CI unreliable | Real risk. If it appears, quarantine the specific test and record it — do not disable the platform |

## Unresolved questions

1. **Do macOS and Windows run on every push, or only on `main` and
   release branches?** Every push is the honest default and the most
   expensive. A narrower trigger is defensible if cost demands it, but
   it must not be "only at release", which reproduces today's gap with
   extra steps.
2. **If Windows fails in a way that needs real work, does 6.0.0 wait?**
   My recommendation is yes for anything in the write path, and a
   documented known-issue for anything cosmetic. The call is the
   owner's, but it should be made with the failures in hand rather than
   in advance.
