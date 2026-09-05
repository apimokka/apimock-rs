# RFC 066 — Branching and merge policy: who moves code, and when

**Status.** Implemented — owner approved 2026-08-27, effective
immediately. Filed in `done/` rather than `accepted/`, following
[RFC 000](./000-rfc-lifecycle-policy.md)'s precedent: `done/` normally
means "released with a version", and a process policy has no version —
its adoption *is* its implementation, and this one must be in force
before the 6.0.0 release it governs, not after it.
**Tracks.** Cross-cutting process policy. Not tied to any feature.
**Touches.** No code. Governs `.git-exclude/roles/`'s silence on
version control, and mirrors to `.git-exclude/rules/` on approval, the
way [RFC 000](./000-rfc-lifecycle-policy.md) does.

## Summary

Nothing in this project states who may merge to `main`. The dev team has
merged four implementation branches (RFC 064, its Amendment 1, RFC 065,
and the pre-6.0.0 documentation block), each time on the strength of a
line in an architect review disposition rather than a rule. It has
worked every time. It is still undocumented, and the next thing on the
schedule — the 6.0.0 release sequence — is the one area where "who
pushes what" has already caused real damage.

This RFC writes down the practice that works, and draws the one boundary
that is currently only implicit: **the release line is not the dev
team's to move.**

## Motivation

### The role documents are silent

`mid-capability-model-operating-instructions.md` § 15 ends the
implementation task at *"a review request has been prepared"*, and adds
*"not considered approved until the high-capability model completes its
review."* Merging appears in **none** of § 2 (responsibilities), § 11
(allowed local decisions) or § 13 (prohibited behaviours).

So merging is neither granted nor forbidden. It has been authorised
case-by-case, by me, in review dispositions.

### The organization document asks for exactly this and was never answered

`ai-multi-agent-software-development-organization-and-workflow.md` § 13
lists what every project must supplement, and the second entry is
**"Branching and version-control policy"**. This project supplemented
the RFC lifecycle (RFC 000) and the Rust/CLI conventions. It never
supplemented this one.

### The gap has already cost something

Not hypothetical, and not the dev team's doing:

- **A commit landed on the wrong branch.** I believed I was on `main`,
  was on a feature branch, and `git push -q origin main` succeeded
  trivially by pushing nothing. I reported a fix as delivered that was
  not on `main`. Cause: a **shared working tree**, and no rule saying to
  check.
- **A force-push needed ad-hoc owner approval** mid-incident, because no
  policy said whether one was permissible.
- **Three releases were blocked** by publisher records binding to a
  workflow *filename*, discovered during a live release rather than
  before one.

Each was survivable. The release sequence is where the same class of
mistake is least survivable, and it is next.

## Goals

1. State who may merge what, and the preconditions.
2. Draw the release-line boundary explicitly.
3. Record the shared-working-tree hazard as a rule, not folklore.
4. Stay short enough to be read.

## Non-goals

- A branching model (git-flow, trunk-based). The project's practice —
  short-lived branches off `main`, fast-forward merges — already works;
  this describes it, it does not redesign it.
- CI or release procedure, which have their own documents
  (`RELEASING.md`).
- Anything about *what* to implement.

## Policy

### 1. The dev team merges its own implementation branches

Confirming existing practice. After an architect review disposition says
to merge, the implementer may merge to `main` and push, without further
approval.

**Preconditions, all required:**

| | |
|---|---|
| **Reviewed** | An architect review exists and its disposition says merge |
| **Green** | **Every** CI job green **on the branch head being merged** — not on an earlier commit, not on `main` |
| **Clean** | `main` has not moved; if it has, rebase or merge and re-run CI |
| **Verified** | After pushing, confirm `git rev-parse HEAD` == `git rev-parse origin/main`. **A quiet `git push` is not evidence** — it succeeds when it pushes nothing |
| **Reported** | The merge commit hash and the CI run id go in the review-request package |

Delete the branch, locally and on `origin`, after merging.

### 2. The release line is not the dev team's to move

**Prohibited without explicit owner instruction**, regardless of any
review disposition:

- Creating, pushing or deleting a **tag**
- Pushing to a `release/*` branch
- Any commit that changes a version number — `Cargo.toml`,
  `Cargo.lock`, `package.json`, or `version.sh`'s output
- Publishing to crates.io or npm, or dispatching a workflow that does
- Editing a workflow file that a publisher record binds to
  (`release-publish.yaml`) — **the binding is to the filename**, so a
  rename silently breaks publishing

Rationale: releases are the owner's timing decision for majors and the
architect's for minor/patch, and the publish path has failed three times
in ways only visible during a live release. A review disposition
approves *code*; it never approves a release action.

> ### Amendment 5 — adopted 2026-09-06: RFC 081 tiers the publish step
>
> [RFC 081](../accepted/081-tiered-release-confirmation.md) replaces
> § 2's blanket publish prohibition with two tiers. The clause
>
> > Publishing to crates.io or npm, or dispatching a workflow that does
>
> now reads
>
> > Publishing to crates.io or npm, or causing the draft→published
> > transition that dispatches a workflow that does — **except for a
> > Tier A release as defined in RFC 081 § 2**, which the architect may
> > publish, reporting the classification.
>
> **Tier A** is a release where all four of RFC 081 § 2's tests hold: no
> `### Security` section in its CHANGELOG entry, no `crates/*/public-api.txt`
> change since the previous release tag, no new or lowered default that
> can refuse a previously-accepted request or connection, and no major
> bump. Anything else is **Tier B** and is the owner's to publish, as
> before.
>
> **Nothing else in § 2 moves.** Tags, `release/*`, version-number
> commits and the publisher-bound filename rule all stand, and
> Amendment 2's wait/retry carve-out is untouched. In particular the
> **tag** remains owner-instructed: Tier A loosens *publish*, not *cut*,
> so a release still cannot begin without the owner.
>
> **Why.** The owner asked whether the publish click was necessary at
> all, given green CI. Largely it was not — it has never caught anything
> here; both real release failures (6.0.0's npm propagation race, and
> publisher records binding to a workflow filename) happened after it or
> upstream of it. What it protects is narrower: whether the release
> should happen *now*, whether the notes are *true*, and the fact that
> crates.io has no unpublish. RFC 081 keeps the gate exactly there and
> moves the asset/notes eyeballing into CI, where it binds on both
> tiers.

> ### Amendment 2 — adopted 2026-08-30: a wait/retry carve-out for the publish workflow
>
> **Adopted — owner approved 2026-08-30.** § 2's clause *"editing a
> workflow file that a publisher record binds to
> (`release-publish.yaml`)"* is **narrowed** to exclude:
>
> > **A wait, retry or poll that changes nothing about what publishes,
> > in what order, or with what credentials.**
>
> Everything else in § 2 stands: no tags, no `release/*`, no
> version-number commits, no publishing, and no change to the publish
> *content* or order without explicit owner instruction.
>
> **Why.** § 2 exists because the publish path has failed in ways only
> visible during a live release. 6.0.0 failed exactly that way —
> `npm-core-publish` lost a race with npm's registry propagation and
> left a partially published release. The fix is to make that job
> *wait longer*, which is on the same side of § 2's concern as the
> clause itself. As written, § 2 forbids the repair of the failure mode
> it was written to guard against.
>
> **How this surfaced.** I wrote a handoff instructing the dev team to
> make that edit, without mentioning the prohibition — in a policy I had
> written five days earlier. They stopped and escalated rather than
> proceeding, which is what § 2 is for and is why it stays.
>
> A one-off authorisation for this edit would leave the next person in
> the identical position, so the boundary moves rather than the
> instance.
>
> **Scope, deliberately tight.** "Changes nothing about what publishes,
> in what order, or with what credentials" is the test. Adding a
> `sleep`, a retry loop, or a poll-until-resolvable qualifies. Reordering
> jobs, adding or removing a published artefact, or touching
> `id-token`/registry configuration does not — those stay owner-only.

> ### Amendment 3 — adopted 2026-09-04: a direct push to `main` is verified too
>
> **Adopted with RFC 080's acceptance** (2026-09-04), which argued that
> under trunk-based development this stops being advisory: with no
> branch to absorb a red build, an unverified push is how `main` goes
> red and stays red. Proposed 2026-09-01; the text below is unchanged.
>
> § 1's **Green** precondition governs merges. § 6 permits the architect
> to commit documents straight to `main`. Nothing covered the gap, and
> the gap turned out to matter:
>
> **`main`'s CI was red for roughly 40 hours across nine consecutive
> architect pushes** (2026-08-31 `3d8bdd8` → 2026-09-01 `3b16397`), from
> two dead documentation links introduced by the first of them. It was
> found by the dev team, whose branch inherited the failure, and fixed
> outside their tranche's scope because it was blocking them.
>
> Throughout that window every dev-team branch was verified by hash,
> job by job. None of that was applied to `main` itself.
>
> **Therefore: after any direct push to `main`, confirm its CI run
> completes green before moving on.** Not the branch you came from — the
> run triggered by that push. If it fails, fix it before starting the
> next thing; a red `main` is inherited by everyone who branches from it,
> and the person who finds it is not the person who broke it.
>
> This is the same **Green** discipline § 1 already requires of the dev
> team. The asymmetry was never justified — it existed because "no review
> needed" was silently read as "no verification needed".

### 3. The working tree is shared

More than one agent works in the same checkout. Therefore:

- **Check `git branch --show-current` immediately before every commit
  and every push.** Do not infer the branch from what you last did.
- If the tree holds someone else's uncommitted work, commit **with an
  explicit pathspec** (`git commit -- <paths>`) so their files are not
  swept in. Never `git commit -a` on a tree you did not leave.
- Never `git checkout`/`switch` away from a branch carrying another
  agent's uncommitted changes.
- Never `git stash` work you did not create.

### 4. Force-push

`--force` is prohibited. `--force-with-lease` requires **explicit owner
approval for that specific push**, and only to a branch no one else is
working on. Never to `main` or a `release/*` branch.

### 5. Branch naming

`<rfc-number>-<slug>` for RFC work (`rfc065-response-body-source-model`);
a descriptive slug otherwise (`docs-pre-6-0-0`). Short-lived: cut from
`main`, merged back, deleted.

## Adoption

On approval: Status → Implemented, mirror to
`.git-exclude/rules/001-branching-and-merge-policy.md` (RFC 000's
pattern), and add a pointer from the mid-capability role document's § 15
so an implementer meets it where their task ends.

## Risks

| Risk | Mitigation |
|---|---|
| Codifying practice makes it rigid | § 1 grants more autonomy than exists today; only § 2 restricts, and only where a live failure has occurred |
| § 2 reads as distrust | It is not about capability. The publish path binds to filenames and publisher records that no agent can inspect — the constraint is on the *action*, not the actor. It binds me equally |
| A policy nobody reads | Kept short; § 3 stated as rules rather than narrative; pointer added where the implementer's task ends |

## 6. The architect's own commits

**Direct to `main`.** RFCs, handoffs, ROADMAP entries, review records,
index updates — all of it.

> ### Amendment 1 — 2026-08-28: handoffs go to `main`, not a branch
>
> This section originally required a Developer Handoff to sit on a
> branch and be shown to the owner before the dev team was told to
> start. **That was wrong and is withdrawn**, on the owner's
> observation: no one works concurrently here — the dev team and I
> alternate, coordinated by the owner — so branch isolation solves a
> problem this project does not have.
>
> It also actively failed. The owner went looking for
> `rfcs/handoffs/post-6-0-0-release-process/implementation-handoff.md`
> and **found nothing**, because it was on a branch. A rule meant to get
> a handoff read before work starts had made it invisible instead.
>
> **What I was actually trying to fix**, and what does fix it: three
> handoffs in the 064/065 series were wrong in ways that mattered — a
> fourth parser I had not listed, an instruction that could not be built
> as written, a path that did not exist, and later a worked example
> (`gte` → `get`) that the edit-distance rule ruled out. The corrections
> for those are already adopted and are about *content*, not location:
>
> - **Give the search, not the list.** Where a change has a findable
>   signature, say "grep for X and fix every live occurrence" rather than
>   naming three files. A list invites being treated as complete.
> - **A worked example is a claim, and gets verified before it ships** —
>   the same bar the dev team's packages are held to.
>
> Branch-versus-`main` was ceremony I mistook for a control. The owner
> reviews a handoff by reading it, which works exactly as well on `main`
> — and the thing that actually gets it read is **telling them the
> path**, which § 1's reporting requirement already covers.

**§ 1 is unaffected.** The dev team keeps working on branches: theirs
carry code, and the branch is what gets a CI run before merging — the
**Green** precondition, which has caught real defects (a Windows-only
test failure on RFC 065's first push). That is branch isolation earning
its keep, which is precisely what my own document commits were not.

> ### Amendment 4 — adopted 2026-09-04: RFC 080 supersedes the branching practice
>
> [RFC 080](../accepted/080-trunk-based-development.md) adopts
> **trunk-based development**: work lands on `main` directly, with no
> implementation branch, except for changes that can behave differently
> on Windows or macOS — those keep a short-lived branch, because only
> CI's `test` job is matrixed and the development machine is Linux.
>
> This RFC's Non-goals declined to choose a branching model, so 080 is
> a new decision rather than a contradiction of this one. **Everything
> here about *who* moves code survives unchanged.** Specifically:
>
> - **§ 1** — the disposition-then-proceed rule stands; "merges its own
>   implementation branch" now reads "commits to `main`". The
>   preconditions (Reviewed / Green / Clean / Verified / Reported) are
>   unchanged, with **Clean** becoming a `git pull --ff-only` before
>   pushing rather than a merge-or-rebase decision.
> - **§ 2** — untouched. The release line is still not the dev team's.
> - **§ 3** — the shared-tree hazard shrinks but does not vanish:
>   concurrent uncommitted edits still collide. With one branch, the
>   `HEAD`-moving class of failure — both recorded incidents — cannot
>   occur.
> - **§ 4** — untouched, and now load-bearing. Force-pushing `main` was
>   already prohibited outright; it is the rule standing between a
>   mistake and rewritten shared history, and must not be softened for
>   convenience.
> - **§ 5** — applies to 080's carve-out branches only.
> - **§ 6** — untouched, and now carries Amendment 3 as binding.
>
> **This document's own evidence argued for 080's carve-out** before
> 080 existed: the closing note above records a Windows-only test
> failure caught on RFC 065's first push, "branch isolation earning its
> keep." That is exactly the case 080 § 3 preserves, and the only case
> it preserves.


## Unresolved questions

1. ~~**Should the architect merge its own RFC/documentation commits, or
   route through review too?**~~ ✅ **Resolved on acceptance
   2026-08-27** — the recommendation stood inside the accepted document,
   so it is adopted as § 6 below. Flagged here in plain sight rather
   than assumed silently: if the owner meant only to accept § 1–§ 5 and
   leave this open, say so and § 6 comes back out.
