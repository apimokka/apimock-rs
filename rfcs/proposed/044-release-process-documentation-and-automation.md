# RFC 044 — Release process: documentation and automation

**Status.** Proposed — design accepted by the project owner 2026-08-02.
Stays in `proposed/` per the 4-folder lifecycle ([RFC 000](../done/000-rfc-lifecycle-policy.md));
implementation is sequenced after v5.15.0 ships (see § Sequencing).
**Tracks.** Release pipeline. The release process is undocumented, its
first step is manual, and one of its two distribution channels
(crates.io) has no automation at all — its only script was broken by the
5.1.1 workspace split and then deleted.
**Touches.** `.github/workflows/release-executable.yaml`, a new
crates.io publish path, `.github/CONTRIBUTING.md` and/or a release
runbook, and the release trigger. **No crate source, no public API.**

## Summary

Write down how a release is actually cut, automate the parts that are
currently manual or missing — notably crates.io publishing — and decide
deliberately **where the human confirmation point sits** between "a
release is intended" and "artifacts are irreversibly published".

## Motivation

### 1. The release procedure is undocumented

Nothing in the repository describes how to cut a release. RFC 032 added
a single line to `CONTRIBUTING.md` about `version.sh`; beyond that there
is no statement of the order of operations, what the gates expect, or
what to do when one fails.

This matters more now than it did a month ago, because the path just
grew three things that did not exist before: RFC 032's
`version-consistency-check`, RFC 031's `quality-gate`, and a corrected
npm version state. A release manager is expected to know about all
three from nothing.

### 2. Release creation is manual, and nothing says so

`release-executable.yaml` triggers on `release: created`. No workflow
creates a Release, so a human must — via the web UI or `gh release
create`. That is a legitimate design, but it is an *implicit* one: the
only way to discover it is to notice that no workflow does it.

### 3. crates.io publishing does not exist — and its script was silently broken

This is the largest gap, and it is invisible.

**Nothing publishes to crates.io.** No workflow step, no tracked script.
Yet `cargo install apimock` is advertised in the README and a docs.rs
badge sits at the top of it.

A script existed once (`cargo-publish.sh`, added in `751843c`, removed
incidentally in `1898f9f`):

```sh
cd crates
crates="apimock-routing apimock-config apimock-server"
for crate in $crates; do
    cd $crate
    cargo package
    cargo publish
    cd ..
done
cd ..
cargo package
cargo publish
```

Two defects, both worth recording:

- **It was already broken before deletion.** The final `cargo package;
  cargo publish` runs at the workspace root, which since 5.1.1 is a
  *virtual manifest with no package* — the façade moved to
  `crates/apimock/`. So the façade crate, the one users actually
  install, could not have been published by this script since 5.1.1.
  **This is the same root cause that broke `version.sh`** (RFC 032
  § Motivation 1): the workspace split silently invalidated a
  path-dependent tool and nothing reported it.
- **No failure handling.** A failed `cargo publish` mid-loop is ignored
  and the loop continues, potentially publishing a dependent crate
  against an index entry that never landed.

### 4. Ordering is a real constraint, unlike npm

The four crates must be published in dependency order —
`apimock-routing` → `apimock-config` → `apimock-server` → `apimock` —
and each must be resolvable on the index before the next is published,
because `[workspace.dependencies]` pins them by version
(`apimock-config = { version = "5", path = … }`). Publishing out of
order fails. The npm path has no equivalent constraint, so this is a
class of problem the existing automation has never had to handle.

## Goals

1. A written release procedure that a person who has never cut a release
   can follow.
2. crates.io publishing that exists, respects dependency order, and
   fails loudly.
3. A deliberate, documented human confirmation point.
4. Release creation automated to the extent that is compatible with (3).

## Non-goals

- Changing the version-numbering scheme, release cadence, or the
  major-version decision (owner's, always).
- Changing `version-consistency-check` (RFC 032) or `quality-gate`
  (RFC 031). This RFC composes with them; it does not modify them.
- Changing the build matrix, published targets, or archive layout.
- Any crate source change.

## The central design question

Automation shortens the path between intent and irreversible publication.

Today, creating a GitHub Release is a deliberate, high-ceremony act, and
everything downstream follows from it. If a bare tag push triggers the
chain instead, then `git push --tags` — a low-ceremony action people
perform from habit — becomes: build → upload assets → `npm publish` →
(once this RFC lands) `cargo publish`. Both registries are effectively
append-only; `npm deprecate` and `cargo yank` hide a version, they do
not remove it.

RFC 032 enlarged this: `npm/package.json` now carries a version that has
never been published, so a publish would **succeed** rather than being
rejected as a duplicate. RFC 031's evidence gathering was blocked
precisely by this hazard.

So the question is not "how do we automate this" but **where the human
confirmation point goes**.

### Options

| | Trigger | Confirmation point | Blast radius |
|---|---|---|---|
| **A** | Tag push → everything | none | Largest — a habitual command publishes |
| **B** | Tag push → build + upload to a **draft** Release; human publishes the Release → publish jobs | the publish click | Small; confirmation sits where the human is already looking |
| **C** | `workflow_dispatch` with a version input | starting the workflow | Small; nothing fires from a bare tag |
| **D** | Status quo — manual Release creation | creating the Release | Small, but undocumented and leaves crates.io unautomated |

**Recommendation: B**, with GitHub **environment protection rules** on
the publish jobs regardless of which trigger is chosen. Environments add
required-reviewer approval to a job, which is orthogonal to the trigger
and is the mechanism actually designed for this. B also puts the
confirmation at the same moment as release-notes review, which is where
a human is already paying attention.

Option A is not recommended at any scale this project operates at.

## Reference-level explanation

### Release procedure documentation

A runbook covering, at minimum:

- Pre-release checks and the order they run in.
- `./version.sh --update <version>` and what it touches (RFC 032).
- CHANGELOG preparation.
- Tagging convention: `X.Y.Z`, no `v` prefix.
- What each gate checks and the most likely reason each fails.
- crates.io publish order and what to do when one crate fails partway.
- Rollback: `cargo yank`, `npm deprecate`, and their limits.

Location is the implementer's call — `CONTRIBUTING.md`, a docs page, or
a `RELEASING.md` — but it must be discoverable from the repository root.

### crates.io publishing

Restored as a workflow job, not a loose script. Requirements:

- Publish in strict dependency order.
- **Fail the whole job on any single crate's failure.** The deleted
  script's silent continue-on-error is the specific behaviour not to
  reproduce.
- Handle index propagation between publishes. Recent cargo waits for the
  index by default; the implementation must confirm this rather than
  assume it, and fall back to an explicit wait if not.
- Be idempotent-safe: re-running after a partial failure must not fail
  merely because earlier crates are already published.
- Use trusted publishing if available. The release workflow already
  declares `id-token: write` and npm trusted publishing is already in
  use (`e9a56d0`), so the pattern is established here.

### Composition with existing gates

The dependency graph must remain intact:

```
version-consistency-check ─┐
                           ├─▶ build ─▶ npm publish ─▶ (new) crates.io publish
quality-gate ──────────────┘
```

crates.io publishing goes **after** the npm path or in parallel with it,
but never before `build` — the same gates that protect npm must protect
crates.io.

## Testing and verification

The hard part: this cannot be fully exercised without publishing, and
publishing is irreversible.

- Structural verification of the dependency graph (as done for RFC 032).
- `cargo publish --dry-run` for every crate, in order — this validates
  packaging without publishing.
- `cargo package` artifacts inspected.
- The trigger and gating behaviour verified on a disposable branch where
  possible, under the same guardrails as RFC 031's Decision 001.
- **No test Release, and no test publish to either registry.**

The first real exercise is a real release. That is inherent, and the
runbook's rollback section exists because of it.

## Risks

| Risk | Mitigation |
|---|---|
| Automation makes an accidental publish easier | The whole point of the confirmation-point decision; environment protection as a second layer |
| Partial crates.io publish leaves an inconsistent set | Fail loudly, document the recovery path, make re-runs safe |
| A first-run defect surfaces during a real release | Design against v5.15.0's observed behaviour (see Sequencing) |
| Trusted publishing misconfiguration | Out of repository visibility; confirm before relying on it |

## Sequencing

**This RFC is drafted now but should not be implemented before v5.15.0
ships.**

v5.15.0 is the first end-to-end run of the repaired release path —
`version-consistency-check`, `quality-gate`, corrected npm versions, and
the first npm publish since 5.7.0. None of it has been observed on a real
runner. Automating on top of a pipeline nobody has watched run once would
repeat the mistake this RFC documents: tooling that looks correct,
silently stops working, and is not noticed for several releases.

Observations from the v5.15.0 release become inputs to this RFC's
implementation, and any that contradict this design should amend it
before work starts.

## Release-authority note

Per the project owner's delegation (2026-08-02): **timing and cadence of
minor and patch releases are the architect's to manage**; major-version
timing remains the owner's alone.

That delegation covers *deciding when to release*. It does not extend to
executing an irreversible outward-facing publish without confirmation —
which is the distinction this RFC's confirmation point formalises.

## Unresolved questions

1. **Which trigger option (A–D)?** Recommendation is B; owner decision,
   since it determines how much ceremony stands between intent and
   publication.
2. **Are crates.io trusted-publishing credentials configured?** Not
   verifiable from the repository. Must be confirmed before
   implementation, or the first run fails for an unrelated reason.
3. **Has anything been published to crates.io since 5.1.1?** If the
   façade has been missing from crates.io since then, `cargo install
   apimock` may be installing a stale version, and that is a user-facing
   problem this RFC should address explicitly rather than incidentally.
4. **Does the runbook live in `CONTRIBUTING.md`, `RELEASING.md`, or
   `docs/`?** Interacts with RFC 034's information architecture if the
   answer is `docs/`.
