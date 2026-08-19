# RFC 039 — An additive-only gate for the public API

**Status.** **Accepted** — approved by the project owner 2026-08-20.
**Not yet implemented, and deliberately not for 6.0.0** — the design is
settled; the gate is enabled after 6.0.0 ships. See § When this turns on.
**Tracks.** CI quality gates; API stability. Follows
[RFC 031](../done/031-ci-quality-gates.md).
**Touches.** `.github/workflows/ci.yaml`, a checked-in API baseline per
crate, `CONTRIBUTING`-level documentation.
**Depends on.** A stable 6.0.0 public surface. Related:
[RFC 052](../accepted/052-non-exhaustive-public-types.md),
[RFC 041](./041-error-type-shape.md).

## Summary

Add a CI job that fails when a crate's public API changes without the
change being declared. Design it now; **switch it on after 6.0.0 ships**,
not before.

## Motivation

### The failure this exists to catch already happened

Risk R-09 in `ROADMAP.md` records it plainly: RFC 040 added three fields
to `TraceConfig`, a public struct that was not `#[non_exhaustive]`. That
is a breaking change, and it went unnoticed until a later review. RFC 052
was then spent making five public types `#[non_exhaustive]`, and RFC 041
proposes spending more of the same budget on the six error enums.

Each of those is a fix for one instance. **None of them is a mechanism
that would have told us at the time.** The risk register says so
directly: *"Exactly the class RFC 039's additive-only gate exists to
catch, and 039 is not built."*

`#[non_exhaustive]` prevents a *category* of break. It does not tell you
when you removed a method, narrowed a return type, or changed a
signature. Only a diff of the public surface does that.

### What CI checks today, and the gap

`ci.yaml` runs six jobs: `fmt`, `clippy` (`-D warnings`), `test`,
`msrv` (`cargo check` on the pinned 1.91.0), `audit` (`cargo-audit`), and
`lockfile`. Every one of them answers *"is the code correct and safe?"*

**None answers *"did the public API change?"*** — and the project's
distribution model makes that the question with the widest blast radius:
a library on crates.io, four crates deep, with a GUI (G5) and now a CLI
contract (RFC 053) built on top.

## Goals

1. A CI job that fails when the public API of any of the four crates
   changes and the baseline was not updated in the same commit.
2. Make the API diff **visible in review**, so approving a break is a
   deliberate act with the diff in front of the reviewer.
3. Zero false positives from formatting, private items, or dependency
   bumps.

## Non-goals

- Deciding *whether* a break is allowed. That is semver's job and the
  owner's. This gate makes the break **visible and declared**, not
  forbidden.
- Blocking 6.0.0. The v6 surface is still moving; a gate over a moving
  surface produces noise and gets ignored. See below.
- Replacing `#[non_exhaustive]`. Complementary: one prevents a class of
  break, the other detects all of them.

## When this turns on

**After 6.0.0 ships, as the first gate of the 6.x line.**

The whole value is that a failure means something. Turning it on now
would mean a baseline churning on nearly every v6 commit — RFCs 041, 052
and 057 all move the surface deliberately — and a gate that fails
constantly during normal work is a gate people learn to update without
reading. That is worse than no gate, because it manufactures the
appearance of review.

6.0.0 is precisely when the surface stops moving and the promise begins.
Baseline the API **at the 6.0.0 tag** and let the first 6.x change be the
first thing the gate sees.

## Design

### Tool

`cargo-public-api`, with the baseline checked into the repository —
one file per crate, e.g. `crates/<name>/public-api.txt`.

A checked-in baseline is the point. A tool that only diffs against the
previous release tells CI; **a file in the diff tells the reviewer**, in
the pull request, next to the change that caused it. That is the goal
in § Goals 2, and it is not achievable with an ephemeral check.

### Toolchain constraint, and why it does not conflict with MSRV

`cargo-public-api` builds rustdoc JSON, which needs a **nightly**
toolchain. The workspace pins `rust-version = "1.91.0"` and CI has an
`msrv` job asserting it.

These do not collide, because the API job proves nothing about what
compiles for users: it is an inspection tool, run on its own toolchain,
in its own job. **The `msrv` job stays the authority on what the crates
support.** Say this in the job's comment, so nobody later "fixes" the
inconsistency by aligning them.

Pin the nightly to a dated version and bump it deliberately. An
unpinned nightly makes this job the flakiest in CI, and a gate that
fails for unrelated reasons is a gate that gets bypassed.

### The workflow

1. Job builds the current public API for each crate.
2. Diffs it against the checked-in baseline.
3. **Identical** → pass.
4. **Differs** → fail, printing the diff, with a message naming the two
   valid responses: update the baseline in this commit (declaring the
   change), or undo the change.

Nothing is auto-updated. The commit that changes the API contains the
baseline change, so `git log` on that file becomes the API's changelog —
worth having on its own.

### Interaction with RFC 043

RFC 043 moves code between modules without changing the public API. If
that refactor produces a baseline diff, the gate has caught a real leak
of module structure into the public surface — which is exactly what RFC
043 § Testing wants to know. Complementary, not conflicting.

## Testing and verification

- A deliberate breaking change on a branch (remove a public method) makes
  the job **fail**, with a readable diff. Prove the gate fires, not just
  that CI is green.
- A deliberate additive change also fails until the baseline is updated —
  additive-only means *declared*, not *unchecked*.
- An internal-only change (rename a private fn, move a module) produces
  **no** baseline diff.
- Job runtime is reported. If it materially slows CI, say so — RFC 031
  balanced this before and the same trade applies.

## Risks

| Risk | Mitigation |
|---|---|
| **Baseline updated reflexively without reading the diff** | The central risk, and the reason for the 6.0.0 timing: a gate that fires rarely gets read. Enabling it during v6 churn would guarantee the failure mode |
| Nightly flakiness blocks unrelated work | Pin a dated nightly; bump deliberately |
| Confusion with the `msrv` job | Documented in the job comment; different toolchains, different questions |
| Four baselines to maintain | They only change when the API changes, which after 6.0.0 should be rare — and if it is not, that is itself the finding |

## Unresolved questions

1. **All four crates, or only `apimock` and `apimock-config`?** The
   façade and the config crate are what external callers and the GUI
   use. `apimock-routing` and `apimock-server` are public by necessity of
   the workspace split more than by intent. Recommend all four initially —
   discovering that a "internal" crate has external consumers is worth
   knowing — and narrow on evidence.
2. **Does the gate block, or warn, for its first 6.x cycle?** Blocking
   from day one is the honest reading of a gate. A warn-only shakedown
   period is defensible but risks becoming permanent.
3. **Should the baseline files ship in the published crates?** They are
   build-irrelevant; likely `exclude`d in `Cargo.toml`. Check against RFC
   032's packaging work rather than assuming.
