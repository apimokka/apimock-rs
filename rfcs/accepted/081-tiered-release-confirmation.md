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
job that runs before any draft becomes publishable. The net effect is
*less* ceremony and *more* enforcement.

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
| No `crates/*/public-api.txt` changed since the previous release tag | `git diff <prev-tag>..<tag>` |
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
every asset is attached and before any human or agent sees a
publishable draft:

- **Exactly the five expected assets are present**, named for the tag:
  `Linux-aarch64-musl`, `Linux-x64-gnu`, `Linux-x64-musl` (`.tar.gz`),
  `macOS-aarch64`, `Windows-x64` (`.zip`).
- **The release notes are non-empty and byte-identical** to the
  `CHANGELOG.md` section for this tag.

If either fails, the build phase fails. A draft short an asset — the
case `RELEASING.md` warns about, where "publishing triggers npm/crates.io
regardless of whether every target succeeded" — becomes unreachable
rather than merely discouraged.

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
