# RFC 081 — Tiered release confirmation: automate the eyeballing, keep the judgement

**Status.** **Accepted** — owner approved 2026-09-06. Both unresolved
questions resolved on acceptance; see below.
**Tracks.** Process / release. Amends
[RFC 066](../done/066-branching-and-merge-policy.md) § 2 and
`RELEASING.md`.
**Touches.** `rfcs/done/066-branching-and-merge-policy.md` (§ 2, one
clause), `RELEASING.md`,
`.github/workflows/release-executable.yaml` (one new job). No source, no
change to what publishes or in what order.
**Origin.** The owner, 2026-09-05, on being asked to click Publish for
6.1.0: *"Is it a necessary step? This time only or forever? In my
mental model, you may publish it because nothing bad was found in CI.
Of course, I concern about security, privacy and release stability."*

## Summary

Today every release requires the owner to publish the draft, because
RFC 066 § 2 forbids the architect from "publishing to crates.io or npm,
or dispatching a workflow that does."

Replace that blanket rule with two tiers:

- **Tier A** — the architect may publish. Routine releases with no
  security content, no public API change, and no new default that can
  refuse something previously accepted.
- **Tier B** — the owner publishes. Everything else.

And **move the checks a human currently performs by eye into CI**, as a
job that turns either failure into a red build phase on the tag — paired
with a precondition that a draft is not published until that phase is
green (Amendment 1 B). The net effect is *less* ceremony and *more*
enforcement.

## Motivation

### The click has never caught anything here

Both release failures this project has actually had happened where the
click could not help:

- **6.0.0's partial npm publish** — `npm-core-publish` lost a race with
  registry propagation. That is *after* the transition; the click had
  already happened.
- **Three blocked releases from publisher records binding to a workflow
  *filename*** — a configuration fact, upstream of the draft entirely.

Neither was preventable by a person looking at a draft.

### What a human checks at the draft, a machine checks better

`RELEASING.md` § "The draft — what to check before clicking Publish"
lists exactly two things: the notes read right, and all five assets are
present. The second is a counting task a distracted human can fail and
a job cannot. The first is only checkable against the CHANGELOG, which
is also mechanical.

### But CI cannot see three things, and they are the ones that matter

1. **Whether the release should happen *now*** — an embargo, a
   coordinated disclosure, a dependency fix worth waiting for. No job
   knows the calendar.
2. **Whether the release notes are *true*.** CI asserts the CHANGELOG
   section exists; nothing asserts it is accurate. Release notes go to
   crates.io as a permanent public record.
3. **It cannot undo.** crates.io has no unpublish — only yank, with the
   version number permanently consumed.

### And the architect is currently reviewing its own work

I write the release notes. Under a blanket "architect may publish" I
would also approve them. That is self-review of the artifact class with
this project's highest observed error rate — documentation. In this
session alone: a statement declaring apimock 5.0.0–5.19.0 *unaffected*
by an advisory they were vulnerable to (caught before it shipped), and
an API-stability promise that was never approved, contradicted the RFC
it cited, and stood published for weeks.

The owner's instinct is right that the ceremony is disproportionate for
a routine release. It is not right that CI green covers everything — it
covers everything *except* the reasons the gate exists.

## Goals

1. Remove the confirmation step where it is ceremony.
2. Keep it where the failure is irreversible, public, or a judgement
   call.
3. Make the classification **mechanical**, so "which tier is this?" is
   never an argument.
4. Enforce in CI what is currently trusted to attention.

## Non-goals

- Changing *what* publishes, in what order, or with what credentials.
  RFC 066 § 2's remaining prohibitions and Amendment 2's carve-out are
  untouched.
- Changing who decides a **major**'s timing. That stays the owner's.
- Continuous deployment. A release stays a deliberate act.

## Design

### 1. The gate is the draft→published transition, not a UI click

`release-publish.yaml`'s `on:` block has **no `push:` trigger** — it
fires only on `release: types: [published]`. Its own header comment
calls this "a structural fact, not a convention." So "publishing" means
*causing that transition*, by any means: the GitHub UI, `gh release
edit --draft=false`, or the API. This RFC governs who may cause it, not
which button they use.

### 2. Two tiers, classified mechanically

A release is **Tier A** when **all four** hold:

| test | read from |
|---|---|
| The CHANGELOG entry has no `### Security` section | `CHANGELOG.md` |
| No `crates/*/public-api.txt` changed since the previous release tag — **and the baselines exist at both tags; absence is not a pass** (Amendment 1 A) | `git diff <prev-tag>..<tag>` |
| No new or lowered default that can refuse a previously-accepted request or connection | the CHANGELOG's Added/Changed sections |
| The major component did not change | the version |

Otherwise it is **Tier B**.

**Tier A** — the architect may publish, and reports the classification
with the four results.
**Tier B** — the owner publishes, as today.

Every test reads an artifact that already exists. Two are fully
mechanical; the third is a judgement, and it is deliberately worded to
fail *towards* Tier B — if you are unsure whether a new default can
refuse something, it can.

**6.1.0 would be Tier B** on three of the four: it has a `### Security`
section, `apimock-server/public-api.txt` changed, and it adds five
limits and an allowlist that refuse previously-accepted requests. This
RFC would not have loosened anything about the release that prompted
it — which is the point. It is not written to make the current release
easier.

### 3. CI asserts what the human eyeballed

Add a final job to `release-executable.yaml`, `needs: [build]`, after
every asset is attached and before anyone *should* publish the draft:

> **Corrected by Amendment 1 B.** This section originally said "before
> any human or agent sees a publishable draft." That is false — the
> draft is created before `build` and is publishable while this job is
> still running. The job cannot block the transition; it fails the
> build phase loudly. Hence Amendment 1 B's precondition: **the build
> phase must be green on the tag before the draft may be published.**

- **Exactly the five expected assets are present**, named for the tag:
  `Linux-aarch64-musl`, `Linux-x64-gnu`, `Linux-x64-musl` (`.tar.gz`),
  `macOS-aarch64`, `Windows-x64` (`.zip`).
- **The release notes are non-empty and byte-identical** to the
  `CHANGELOG.md` section for this tag.

If either fails, the build phase fails — **red, on the tag, before the
release is fit to publish**. A draft short an asset is the case
`RELEASING.md` warns about, where "publishing triggers npm/crates.io
regardless of whether every target succeeded"; that case now announces
itself instead of relying on someone counting. Combined with
Amendment 1 B's precondition — do not publish until the build phase is
green — it is what stops such a draft reaching the registries.

**This is stricter than today for every tier**, including Tier B.

### 4. RFC 066 § 2 — the one clause that changes

> Publishing to crates.io or npm, or dispatching a workflow that does

becomes:

> Publishing to crates.io or npm, or causing the draft→published
> transition that dispatches a workflow that does — **except for a
> Tier A release as defined in RFC 081 § 2**, which the architect may
> publish, reporting the classification.

Everything else in § 2 stands unchanged: no tags, no `release/*`, no
version-number commits, no change to publish content or order, and
Amendment 2's wait/retry carve-out exactly as written.

Note the tag itself remains owner-instructed. Tier A loosens *publish*,
not *cut* — so a release still cannot begin without the owner.

### 5. Classification is declared, never inferred silently

The release record states the tier and the four results **before**
publishing, not after. A Tier A release that turns out to have been
Tier B is a process failure to report, in the same way a bar-changing
narrowing is.

## Testing and verification

- The new job **fails** when an asset is missing — prove it fires by
  deleting one from a draft on a throwaway tag, not by observing green.
- The new job **fails** when the notes and the CHANGELOG section differ.
- A dry-run classification against the last three releases (6.0.0,
  5.19.1, 5.19.0), reporting which tier each would have been. If any
  reads Tier A that a person would call Tier B, the tests in § 2 are
  wrong and this RFC needs changing before it is adopted.

## Risks

| Risk | Mitigation |
|---|---|
| A Tier A release turns out to carry security content | The `### Security` test is mechanical, and the third test fails towards Tier B. A misclassification is reportable, not silent. |
| The architect publishes its own inaccurate notes | Unchanged from today for Tier A's content, which by definition has no security claims and no API change — the two places notes have actually been wrong here. Tier B, where they could be, still needs the owner. |
| Tiering becomes an argument each release | § 2's tests read existing artifacts, and § 5 requires them reported. |
| The new CI job blocks a legitimate release | It only asserts asset count and notes-match — both fixable by re-running the build phase, neither reachable after publish. |

## Amendment 1 — adopted 2026-09-06: two of this RFC's own claims were wrong

Both found by the § Testing dry-run classification, in the review of
§ 3's implementation
(`.git-exclude/reviewed/081-tiered-release-confirmation/REVIEW-001.md`).
The exercise existed to test these tests; it did.

### A. T2 must require the baselines to *exist*, not merely to be unchanged

**§ 2's second test as written can pass without checking anything.**
`crates/*/public-api.txt` was added in `1b7ebec` on 2026-08-31 — *after*
6.0.0 was tagged on 2026-08-28. So for 6.0.0 and 5.19.1,

```
git diff <prev-tag>..<tag> -- 'crates/*/public-api.txt'
```

is empty because **the files did not exist at either tag**, not because
the public API held still. 6.0.0's own CHANGELOG documents real
breaking library changes across that boundary — six types becoming
`#[non_exhaustive]`, error variants boxed — that this test therefore
could not have seen. Read literally it *passes*; read for what it is
trying to establish it is **not applicable**.

**T2 is amended to:**

> No `crates/*/public-api.txt` changed since the previous release tag —
> **and the baselines exist at both tags**. If they are absent at
> either, the test is *not applicable*, which resolves to **Tier B**.
> Absence is never a pass.

In practice this changes nothing going forward: every release from
6.1.0 on carries the baselines. It is amended anyway, because "it
closes on its own" is exactly what would have been said about RFC 039's
gate, and this is the same defect — **a check whose name asserts more
than its mechanism delivers**. Finding it in the RFC written to tighten
release confirmation is the reason it is written down rather than
quietly patched.

### B. § 3's job cannot prevent an early publish, and § 3 said it could

`release-executable.yaml`'s job order is:

```
version-consistency-check + quality-gate → create-draft-release → build → assert-draft-release
```

The draft is created **before** `build`, so it exists and is publishable
while `build` and `assert-draft-release` are still running. § 3's claim
that the job runs *"before any human or agent sees a publishable
draft"* is **false**. It runs before anyone *should* publish — a
different thing. The job fails the build phase loudly; it cannot block
the transition, and no placement inside that workflow could make it.

Under the pre-081 rule this was academic: the owner published, after the
build finished. **Under Tier A it is not** — there is no second party,
so an architect publishing on sight of a draft, or without checking the
build phase, bypasses the assertion entirely.

**Therefore, a precondition on publishing, both tiers:**

> **The build phase must be green on the tag before the draft may be
> published.** Confirm the run, by id, on the tag being published — not
> that "a green run exists". A draft is publishable from
> `create-draft-release` onward, which is well before anything has been
> asserted about it.

This is RFC 066 Amendment 3's discipline — verify the run, never assume
it — applied to the release path, and it is what makes § 3's job
load-bearing rather than advisory. Mirrored into `RELEASING.md` § "The
draft — who publishes it, and what to check".


## Unresolved questions

1. ~~**Should Tier A require any second party at all?**~~ ✅ **Resolved
   on acceptance 2026-09-06 — no.** The recommendation stood inside the
   accepted document, so Tier A needs no second party and no
   cooling-off. Flagged in plain sight rather than assumed: if you
   meant to accept the tiering but wanted a cooling-off, say so and it
   goes in.
2. ~~**Should the § 3 job also assert the tag is signed?**~~ ✅
   **Resolved on acceptance 2026-09-06 — not folded in.** It was
   offered as "out of scope unless the owner wants it," and no
   instruction came, so it stays out. Recorded here rather than
   dropped, because the finding behind it stands: nothing enforces tag
   signing. Of 169 tags, **123 are signed and 46 are not** — every
   unsigned one is from `0.9.0` through `2.9.4`, so signing has been
   universal since 3.x and includes 6.1.0. An assertion would pass on
   everything current while turning an old habit into a rule. Worth a
   one-line RFC of its own whenever it comes up.
