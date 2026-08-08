# RFC 044 — Release process: documentation and automation

**Status.** Proposed — design accepted by the project owner 2026-08-02;
**design amended and re-accepted 2026-08-03** (trigger correction,
draft-first flow, reprioritisation — see § Amendment).
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

### 3. crates.io publishing is not automated — and its script was silently broken

This is the largest gap, and it is invisible.

**Nothing publishes to crates.io.** No workflow step, no tracked script.
Yet `cargo install apimock` is advertised in the README and a docs.rs
badge sits at the top of it.

*Clarified 2026-08-03:* crates.io itself is **current** — all four
crates are published through 5.14.0. So the channel is not stale; the
publishing is being done by hand. That makes this a resilience gap
rather than a live breakage: a working procedure that exists only in the
maintainer's head, with the four-crate ordering constraint (§ 5) as its
sharpest edge and no script left to encode it.

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

### 4. The Release is publicly visible before it has any assets

Added 2026-08-03. This is a live, user-visible defect, not a process gap.

`release-executable.yaml` reacts to a Release that a human has already
created — and, today, already published. Only then does `build` start,
as a five-target matrix. Each target compiles and then runs
`gh release upload` for its own asset
(`release-executable.yaml:223`).

So the sequence a visitor sees is:

1. Release appears, **zero assets**.
2. Assets trickle in one at a time over several minutes as targets
   finish.
3. Release is finally complete.

Anyone landing on the Release page during step 1 or 2 sees an
incomplete release. Nothing in the current design prevents this, and no
amount of documentation fixes it — only CI owning Release *creation*
does, by building the artifact before it is visible.

### 5. Ordering is a real constraint, unlike npm

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

---

## Amendment — 2026-08-03

Two corrections to the design above, both accepted by the project owner.
The recommendation (B) is unchanged; what changes is the mechanism that
makes it work, and the priority order of this RFC's three parts.

### A1 — Option B does not work on the current trigger

`release-executable.yaml` fires on `release: types: [created]`. GitHub
documents `created` as firing when **a draft is saved**, not only when a
release is published.

So under today's wiring, saving a draft would start the entire chain
including `npm publish` — the draft would stage nothing, and Option B as
originally written is unimplementable.

**Correction: the publish jobs must trigger on `release: published`.**
`published` unambiguously means the release went live, which is exactly
the semantics Option B needs. `github.ref_name` remains the tag under
either event, so nothing else in the workflow changes.

This is a one-line fix, but without it the recommended design silently
degrades into Option A.

*Verify the `created`-on-draft behaviour against current GitHub
documentation before implementing — it was not confirmable from the
development environment where this amendment was written. If it turns
out `created` does not fire on draft save, `published` remains the
better trigger anyway, for being unambiguous.*

### A2 — CI should create the Release, and this was underrated

The original RFC ranked Release-creation automation as the least
valuable of its three parts, on the reasoning that it "saves one
`gh release create` per release". That reasoning was wrong on both
counts.

**On value:** it fixes Motivation § 4 — the Release being publicly
visible with no assets while a five-target matrix builds. Documentation
cannot fix that; only building the artifact before it is visible can.

**On cost:** `release-executable.yaml:223` already runs
`gh release upload`. The `gh` CLI is already present and already
authenticated in this workflow. `gh release create` is the same tool —
no new action, no new dependency.

### A3 — the amended flow

```
git push origin X.Y.Z              ← tag push: the only human trigger
        │
        ├─▶ version-consistency-check ─┐
        └─▶ quality-gate ──────────────┴─▶ create DRAFT Release
                                              (notes from CHANGELOG)
                                                    │
                          build matrix ─────────────┴─▶ upload all assets
                                                    │
        [ human reviews a complete draft, clicks Publish ]
                                                    │
                              release: published ───┴─▶ npm publish
                                                       crates.io publish
```

Properties this has that manual creation does not:

- The Release is visible only once complete.
- Release notes are derived from the CHANGELOG section for that version
  rather than retyped.
- Tag and Release cannot disagree — both derive from one push.
- The human approves a finished artifact instead of approving before
  anything exists.

The tag push remains the only human trigger, and it creates nothing
public on its own: a draft is not visible, and no publish happens until
the Publish click.

### A4 — reprioritisation

| | Original | Amended | Why |
|---|---|---|---|
| crates.io automation | 1st | **1st** | Unchanged. Zero automation, ordered four-crate publish, script broken then deleted |
| Release creation + draft-first | 3rd | **2nd** | Fixes a live user-visible defect (Motivation § 4), and is nearly free |
| Runbook | 2nd | **3rd** | Still required, but a shorter document once the process is mostly CI |

The runbook does not disappear — a human still pushes a tag, reviews a
draft, and clicks Publish, and the recovery paths still need writing
down. It simply has less to describe once CI owns the mechanical parts.

### A5 — `cargo publish --workspace` does the hard part already

Established during the v5.15.0 release, 2026-08-04, on cargo 1.97.1.

This RFC's crates.io section was written as though ordered multi-crate
publishing were logic to be built. It is not. A single command does it:

```sh
cargo publish --workspace
```

Observed behaviour in the real 5.15.0 publish:

```
Uploaded apimock-routing v5.15.0 to registry `crates-io`
note: waiting for apimock-routing v5.15.0 to be available at registry `crates-io`.
      3 remaining crates to be published
Published apimock-routing v5.15.0 at registry `crates-io`
Uploading apimock-config v5.15.0 …
```

Cargo resolved the dependency order itself, and **waited for each crate
to be available on the index before publishing the next** — the exact
step the deleted `cargo-publish.sh` never did and the one a hand-rolled
loop is most likely to get wrong. Exit 0, no partial state.

`--dry-run` composes with `--workspace`, so the whole set can be verified
without uploading — a genuinely useful pre-flight the original design did
not contemplate.

**Consequences for this RFC:**

- The crates.io automation shrinks from a script with ordering,
  propagation-waiting, and idempotency logic to approximately one
  workflow step. Three of the five original requirements are satisfied by
  cargo, not by us.
- What remains ours: deciding whether it runs on `release: published`
  alongside the npm jobs, and confirming the credential path (still
  unverified — see Unresolved question 2).
- **Reprioritise again.** With the work this small, the runbook is the
  larger remaining gap:

  | | A4 (2026-08-03) | A5 (2026-08-04) |
  |---|---|---|
  | Release creation + draft-first | 2nd | **1st** — fixes a live user-visible defect |
  | Runbook | 3rd | **2nd** — the process still lives only in the maintainer's head |
  | crates.io automation | 1st | **3rd** — one command, already proven in production |

This is the second reprioritisation of the same three items. Both moves
came from establishing a fact rather than reasoning further — the
incomplete-release window in A2, and cargo's actual behaviour here. Worth
noting as a pattern: this RFC's rankings have been wrong twice, in both
cases because I estimated effort from a description rather than from
running the thing.

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

**Superseded 2026-08-04 — see § Amendment A5.** Cargo does the ordering
and the index waiting itself. The requirements below were written before
that was established and overstate the work; they are kept because the
reasoning about *what could go wrong* remains the acceptance bar, even
though cargo now satisfies most of it.

Original requirements:

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

1. ~~**Which trigger option (A–D)?**~~ ✅ **Resolved 2026-08-03: B**, on
   the amended mechanism in § Amendment A1/A3 — tag push, CI-created
   draft, human Publish, publish jobs on `release: published`.
2. **Are crates.io trusted-publishing credentials configured?** Not
   verifiable from the repository. Must be confirmed before
   implementation, or the first run fails for an unrelated reason.
   Note that crates.io publishing has evidently been done *somehow*
   through 5.14.0 (see item 3), so some credential path exists — whether
   it is trusted publishing or a personal token is unknown.
3. ~~**Has anything been published to crates.io since 5.1.1?**~~
   ✅ **Resolved 2026-08-03: yes, it is current.** The sparse index
   shows all four crates published through **5.14.0** (2026-08-01). The
   speculation that `cargo install apimock` might be serving a stale
   version was wrong.

   This weakens this RFC's urgency but not its conclusion: the manual
   crates.io process exists and works, but lives only in the
   maintainer's head — undocumented, unautomated, and with its only
   script broken and then deleted. The case is "a working process with
   no written form or safety net", not "a broken channel".
4. **Does the runbook live in `CONTRIBUTING.md`, `RELEASING.md`, or
   `docs/`?** Interacts with RFC 034's information architecture if the
   answer is `docs/`.
