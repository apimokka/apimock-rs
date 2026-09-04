# RFC 080 — Trunk-based development: `main` is the working branch

**Status.** **Accepted** — owner approved 2026-09-04, carve-out kept as
written. Adopts RFC 066 Amendment 3 with it (§ 2).
**Tracks.** Process. Supersedes RFC 066's implicit branching practice;
RFC 066's *who may move code* rules survive unchanged.
**Touches.** `rfcs/done/066-branching-and-merge-policy.md` (§ 1, § 3,
§ 4 rewording), `CONTRIBUTING`-level documentation. No source, no CI
configuration, no release path.
**Origin.** The owner, 2026-09-04: *"in my mental model, a single
branch, `main`, workflow works well enough now instead of feature
branch etc."*

## Summary

Adopt trunk-based development: work lands on `main` directly, with no
implementation branch, for everything except a narrow, evidence-backed
carve-out. RFC 066 explicitly declined to choose a branching model —
its Non-goals name *"a branching model (git-flow, trunk-based)"* as out
of scope — so this is a new decision rather than an amendment.

**The one exception this RFC argues for: changes that can behave
differently on Windows or macOS still get a short-lived branch**, because
those two platforms exist only in CI and cannot be checked before a
push. Everything else goes straight to `main`.

## Motivation

### Branches are not carrying the review here

In a typical project a branch exists so a pull request can host review.
This project does not review that way. Review happens in
`.git-exclude/review-request/` packages and is answered in
`.git-exclude/reviewed/` — prose, with reproductions, against a named
commit. The branch adds ceremony without adding the thing branches are
usually for. RFC 066 § 1's disposition-then-merge flow already treats
the review package, not the branch, as the gate.

### The working tree is shared, and branches make that dangerous

Two actors — the architect and the implementer — work in **one
checkout**. Branch switching in a shared tree has now caused two
recorded incidents:

- **2026-08-20.** The architect made three commits, including a security
  fix for RUSTSEC-2026-0258, believing they were on `main`. They were on
  the dev team's `rfc061-cross-platform-ci`. Every `git push origin main`
  reported success, because the local `main` ref was unchanged. The fix
  was then reported to the owner as landed. It was not.
- **2026-09-03.** Mid-review of audit tranche 2, the architect ran
  `git checkout main` in the shared tree while the implementer had
  `audit-t2-silent-wrongness` checked out and in progress. Their tree
  happened to be clean, so nothing was lost. The implementer caught it,
  merged rather than rebased to avoid a force-push needing owner
  approval, and recorded it in their package § 9b. RFC 066 § 3 names
  this hazard; it did not prevent it.

Both are *branch-switching* failures. Neither is possible when there is
one branch. A single trunk does not fix the shared tree — concurrent
uncommitted edits still collide — but it removes the whole class of
failure where work is lost or misplaced by moving `HEAD`.

### There is no second line to protect

The four crates publish together at one version. There is no maintenance
branch, no parallel release line, no long-running divergence. RFC 066 § 2
already reserves the release line — tags, `release/*`, version commits,
publishing — to the owner, and nothing in this RFC touches that.

## Goals

1. `main` is the working branch. No implementation branch for ordinary
   work.
2. Preserve every RFC 066 rule about *who may move code* and *what must
   be true before it moves*. This changes where work happens, not who
   decides.
3. Do not let a change reach `main` unverified on a platform we cannot
   test locally.

## Non-goals

- Changing who merges, who releases, or the review-package flow.
  RFC 066 §§ 1–2 stand.
- Removing gates. The same gates run; only their timing moves.
- Continuous deployment. Releases stay a deliberate, owner-driven act.

## Design

### 1. Ordinary work commits to `main`

Run the local gate set before pushing — `cargo fmt --check`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo test
--workspace`, and, when the public surface may have moved, the
`public-api` baseline. Push. Then **verify CI on the pushed commit**.

### 2. Verifying CI after the push is mandatory, not advisory

RFC 066 Amendment 3 proposed this and is still awaiting approval. Under
trunk-based it stops being a nicety: with no branch to absorb a red
build, an unverified push is how `main` goes red and stays red.

The precedent is on the record: nine consecutive architect pushes left
`main` red for roughly 40 hours (`3d8bdd8` → `3b16397`), from dead
`README.md` → `README.html` links. The cause was reading RFC 066 § 6's
"no review needed" as "no verification needed." **Approving this RFC
should approve Amendment 3 with it**; they are the same rule.

### 3. The carve-out: cross-platform-sensitive changes still get a branch

CI's `test` job is the only matrixed job — `[ubuntu-latest,
windows-latest, macos-latest]`. The development machine is Linux.
**Windows and macOS behaviour therefore cannot be observed before a
push.** Every other gate — `fmt`, `clippy`, `msrv`, `audit`,
`lockfile`, `package`, `public-api`, `rustdoc`, `docs` — runs locally
and on Linux only, so a local run genuinely predicts them.

This is not hypothetical. Both platforms have produced failures that no
local run could have caught:

- `3b16397` — "Fix Windows TOML path corruption in the new RFC 074 S-07
  tests." TOML treats `\` as an escape introducer, so a native Windows
  path inside a double-quoted TOML string breaks parsing.
- RFC 075's case-sensitivity question — macOS APFS defaults to
  case-insensitive; Linux is the case-sensitive outlier, and the local
  machine is the outlier.
- **RFC 066's own closing note**, written before this RFC existed:
  the **Green** precondition *"has caught real defects (a Windows-only
  test failure on RFC 065's first push). That is branch isolation
  earning its keep."* That sentence is the carve-out, argued by the
  document this RFC supersedes.

**So: use a short-lived branch when a change plausibly touches**
filesystem paths, filename case, line endings, file IO, or TLS
certificate/key handling. Merge it once CI is green, then delete it.
Everything else goes to `main`.

This is the one part of the RFC I would expect the owner to want to
strike, and it is easy to strike — the rest stands without it. My
recommendation is to keep it, because it is precisely the case where
"push and verify" costs a red `main` rather than a red branch, and
where the local gate set is known to be blind.

### 4. RFC 066 rewording

- **§ 1** ("the dev team merges its own implementation branches") →
  the implementer commits to `main` once the review disposition says
  to proceed. The disposition requirement is unchanged.
- **§ 1's preconditions** — Reviewed / Green / Clean / Verified /
  Reported — survive, with **Clean** ("`main` has not moved") becoming
  a `git pull --ff-only` before pushing rather than a merge-or-rebase
  decision.
- **§ 3** (shared working tree) gains: with one branch, never
  `git checkout` away from work in progress — the situation should not
  arise, and if it does, something is wrong.
- **§ 4** (force-push) is unchanged and becomes load-bearing. It
  already prohibits force-pushing to `main` outright — *"`--force` is
  prohibited. `--force-with-lease` requires explicit owner approval for
  that specific push, and only to a branch no one else is working on.
  **Never to `main`**"* — so under trunk-based it is the rule standing
  between a mistake and rewritten shared history. It should not be
  softened to make trunk-based more convenient.
- **§ 5** (branch naming) survives, applying only to § 3's carve-out
  branches. Its "short-lived: cut from `main`, merged back, deleted"
  is exactly the carve-out's shape.
- **§ 2** (the release line is not the dev team's) — untouched.
- **§ 6** (the architect's own commits) — untouched, except that it
  now carries § 2 above explicitly.

## Risks

| Risk | Mitigation |
|---|---|
| A red push blocks the other actor | Local gates before every push; CI verified after. Revert is one commit and always available. |
| Two actors push concurrently and one is rejected | `git pull --ff-only` before push. In a two-actor project this is rare and cheap. |
| A large multi-file change leaves `main` mid-refactor | Land it in coherent commits that each pass the gates, which the audit tranches already did within their branches. |
| A cross-platform break lands on `main` | § 3's carve-out, which exists for exactly this. |
| Losing the tranche-shaped review unit | The review package, not the branch, was always the unit. It names commits; it does not need a branch. |

## Unresolved questions

1. ~~**The carve-out in § 3** — keep, or go fully single-branch and
   accept occasional Windows/macOS breakage on `main`?~~ ✅ **Resolved
   on acceptance 2026-09-04** — kept as written. Flagged here in plain
   sight rather than assumed: the owner accepted without striking it,
   and the recommendation stood inside the accepted document. If only
   §§ 1–2 and 4 were meant and the carve-out should go, say so and § 3
   comes back out.
2. ~~**Whether to adopt RFC 066 Amendment 3 as part of this.**~~ ✅
   **Resolved on acceptance 2026-09-04** — adopted, and binding rather
   than advisory. Recorded as RFC 066 Amendment 3's own status line and
   in Amendment 4.
