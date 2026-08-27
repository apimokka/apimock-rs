# RFC 066 — Branching and merge policy: who moves code, and when

**Status.** Proposed — awaiting owner approval.
**Tracks.** Cross-cutting process policy. Not tied to any feature.
**Touches.** No code. Governs `.git-exclude/roles/`'s silence on
version control, and mirrors to `.git-exclude/rules/` on approval, the
way [RFC 000](../done/000-rfc-lifecycle-policy.md) does.

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

## Unresolved questions

1. **Should the architect merge its own RFC/documentation commits, or
   route through review too?** Today I commit RFC and ROADMAP changes to
   `main` directly, which § 1 does not cover. It has not caused a
   problem, and a self-review is not a review — but the asymmetry should
   be a decision, not an omission. **Recommend: keep direct commits for
   documents no one implements against, and require a branch for
   anything the dev team will build from** — a handoff they read is worth
   the same care as code.
