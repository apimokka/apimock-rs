# RFC 086 — For a minor release, the cut authorises the publish

**Status.** **Implemented — in force from 2026-10-07.** Owner approved
2026-10-07, **scoped to minor releases**: *"My proposal is around minor
version releases only."* A process policy, so adoption is
implementation, as with RFC 080 and RFC 081. **First used for 6.4.0, and
its owed check is satisfied:** the architect published at `00:42:09Z` and
`release-publish.yaml` ran two seconds later as run `37553345299` (event
`release`, ref `6.4.0`), completing 12/12. A user-authenticated `gh`
publish fires the workflow exactly as the owner's click does.
**Origin.** The owner, 2026-10-07: *"When I authorize the cut, you may
publish GitHub release in addition to tagging."* The same wish they
stated when RFC 081 was being written: *"I want it independent of my
manual operation as possible."*
**Amends.** [RFC 081](./081-tiered-release-confirmation.md) (who
publishes), [RFC 066 § 2](./066-branching-and-merge-policy.md) as
amended by its Amendment 5, and `RELEASING.md`.
**Supersedes.** [RFC 083](../archive/083-publish-without-a-click.md), moved to `archive/` on approval.
**Touches.** Documents only. **No workflow file changes.**

## Summary

When the owner authorises the cut of a **minor release** (`X.Y.0`), that
authorisation also covers **publishing the GitHub Release**: the
draft-to-published transition that fires `release-publish.yaml`. The
architect does it, on either tier, once the build phase is green on the
tag and recorded. The owner keeps the right to reserve the publish when
authorising, and still publishes **majors**, **patch releases that RFC
081 classifies as Tier B**, and any release coordinated with an
embargoed security advisory.

| release | who publishes |
|---|---|
| **minor** `X.Y.0` | **the architect**, on the owner's cut authorisation, on either tier, unless the owner reserves it |
| **patch** `X.Y.Z` (`Z` > 0) | unchanged: **RFC 081 decides**. Tier A, the architect may; Tier B, the owner |
| **major** `X.0.0` | **the owner** |
| coordinated with an **embargoed advisory** | **the owner**, whatever the version |

## Motivation

### What the owner's click protects today, measured

RFC 081 kept the click for Tier B because it protects three things CI
cannot see. A month later, here is where each one stands:

1. **"Should this release happen now?"** The owner answers that when
   authorising the cut. The click repeats a decision already taken.
2. **"Are the notes true?"** For the claim types this project has
   actually got wrong in public, CI now answers it. RFC 084 checks API
   claims against the baselines, checks that every advisory ID resolves,
   requires an unaffectedness claim to cite its advisory record, and
   checks dependency versions against the lockfile. It passed on its
   first live run (6.3.0). What remains unchecked is **the fairness of
   prose**.
3. **crates.io cannot unpublish.** True, and unchanged by who clicks.
   It is answered by the gates before the click, which remain.

So the click's remaining content is one human reading the prose of the
notes before they become permanent. In practice: for 6.3.0 the owner
authorised the cut **before the notes existed** (*"Cut is authorized if
ready and reasonable"*), so the click was the only point at which they
could have read them.

### Why not RFC 083's mechanism

RFC 083 would have had CI publish with no click at all. It needed either
a long-lived credential or a restructuring of the publish path to
`workflow_call`, because a release published with Actions'
`GITHUB_TOKEN` does not fire `release-publish.yaml`. **Neither is needed
here.** The architect publishes with `gh`, authenticated as a user just as
the owner's browser is. GitHub's documented rule is that only events
created by Actions' own `GITHUB_TOKEN` are suppressed, so `release:
published` should fire as it does on the owner's click. **That is
documented behaviour, not yet observed under this RFC**, so § 4 makes
the first release prove it. No new credential, and no workflow changes.

## Design

### 1. What a cut authorisation covers

For a **minor release**: **tag and publish**, on either tier, unless the
owner says otherwise. For a patch or a major, it covers the tag only, as
before; who publishes is in the Summary's table.

The owner may **reserve** the publish when authorising: for example
*"cut it, but I'll publish"*, or *"not before 09:00 UTC"*. A reservation
binds that release only.

### 2. Preconditions — all of them, on the tag, by run id

The architect publishes only when **every** one of these holds, and has
recorded each in `.git-exclude/release/<version>/` **before** publishing:

- CI green **on the tagged commit**, with the `package` job confirmed to
  have run (not skipped);
- the build phase green **on the tag**: `version-consistency-check`,
  every `build` leg, `assert-draft-release` and
  `assert-changelog-claims`;
- the draft checked independently: five assets, and notes equal to the
  CHANGELOG section;
- the tier classification and its four results (RFC 081 § 5). The tier
  no longer decides *who* publishes, but it is still computed, recorded
  and reported.

This is the practice used for 6.2.1 and 6.3.0, written down so that it
binds rather than depending on attention.

### 3. Where the owner still publishes

- **A release coordinated with an embargoed or not-yet-public security
  advisory.** Its timing must be exact and agreed with a third party,
  and publishing the release is effectively publishing the advisory. The
  owner publishes, or names the minute.

- **Majors.** The owner publishes. *(The proposal had suggested majors
  be covered by the cut authorisation; the owner decided otherwise on
  approval.)*
- **Patch releases** remain under RFC 081 unchanged. A Tier B patch,
  which is typically a security fix such as 6.2.1, is the owner's to
  publish.

### 4. After publishing

**On the first release under this RFC, confirm by run id that
`release-publish.yaml` fired on the architect's publish.** If it did not,
nothing has reached a registry. The owner publishes that release from the
UI, and this RFC is revisited before it is used again.

The architect verifies **both registries independently**, not from the
workflow's own checks (which still do not cover the core npm package):
all four crates, and the core and three platform npm packages with
`dist-tags.latest`. It then reports. A partial publish is diagnosed and
re-run by the architect (RFC 066 Amendment 2 already covers the
wait/retry side), and reported plainly.

### 5. What changes in the documents

- **RFC 081**: an amendment stating that a cut authorisation covers
  publishing, per this RFC. Tier A/B become a *classification that is
  reported*, no longer a rule about who clicks.
- **RFC 066 § 2**: the publish clause amended to match, as Amendment 5
  did for RFC 081.
- **`RELEASING.md`**: the flow's *"publish the draft — Tier A: the
  architect may; Tier B: the owner"* line, and the sections around it.
- **RFC 083**: moved to `archive/`, superseded. Its `GITHUB_TOKEN`
  finding stays readable there.

## What is honestly lost

**One human reading the notes' prose before it becomes permanent.** The
architect writes the notes, and under this RFC also publishes them, so on
the prose this is self-review. That is the weakest review in the system,
and RFC 081 said so.

It is bounded by three things. RFC 084 now checks the claim types that
have actually been wrong here. The owner can still ask to see the notes
before authorising, or reserve the publish. And the one category where
an error is both irreversible *and* time-critical, a coordinated
advisory, keeps the owner's own click.

## Non-goals

- **Continuous deployment.** A release still starts only with the
  owner's authorisation; nothing reaches a registry from a merge.
- **Changing any gate.** Every check that runs today still runs and
  still must be green; this RFC adds the requirement that each is
  verified by run id and recorded before publishing.
- **Workflow changes.** None.

## Unresolved

1. ~~**Majors**~~ — **decided on approval: the owner publishes majors**,
   and the RFC covers **minor releases only**.
