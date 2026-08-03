# Implementation Handoff — RFC 033, Supply-chain gates

**Governing RFC.** [RFC 033](../../done/033-supply-chain-gates.md)
**Milestone.** M1 (Pipeline trust) → v5.15.0 — the last item.
**Status.** Inherited from RFC 033 (Proposed; scope settled by owner
decision D-04, 2026-08-02)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

**Prerequisite.** [RFC 031](../../done/031-ci-quality-gates.md)'s
`ci.yaml` must exist — this RFC extends it. RFC 031's implementation is
approved; its live-run evidence may still be in progress. Coordinate so
you are not editing `ci.yaml` while that evidence is being captured.

---

## 1. Purpose

Add two blocking CI checks that catch problems arriving from outside the
codebase: a known-vulnerability audit of the dependency graph, and a
lockfile-freshness check.

## 2. Scope, as settled by D-04

**Two checks. Not three.**

| Check | Command |
|---|---|
| Vulnerability audit | `cargo audit` |
| Lockfile freshness | `cargo update --workspace --locked` |

**`cargo-deny` is out of scope entirely.** Do not add `deny.toml`, do
not add a licence allow-list, do not add bans/duplicates or sources
policy. The owner dropped it on evidence: a survey of all 270 crates in
the graph found no GPL/AGPL and no licence conflict, so the check
guarded a risk currently measuring zero. RFC 033's decision-record
section retains the survey; do not re-litigate it in the
implementation.

If you believe a licence policy is needed, that is a design request, not
a code change.

## 3. Change scope

- `.github/workflows/ci.yaml` — add two jobs
- `.github/CONTRIBUTING.md` — document the two commands alongside RFC
  031's four

## 4. Explicit non-change scope

Do **not**:

- Add `deny.toml` or invoke `cargo-deny` (§ 2).
- Upgrade, add, or remove any dependency, **except** as strictly
  required to clear an advisory that actually fires. Tidying
  `[workspace.dependencies]` pins is not in scope.
- Touch crate source.
- Modify RFC 031's four existing jobs, or RFC 032's
  `version-consistency-check`.
- Change `docs.yaml`.

## 5. Required implementation

### 5.1 Vulnerability audit

`cargo audit` against the RustSec advisory database, over the committed
`Cargo.lock`.

**Also runs on a weekly `schedule` trigger**, not only on push and pull
request. This is the point of the check: advisories are published
against code that has not changed, so a check that only runs on commits
will not see them. A scheduled run turning CI red with no new commit is
correct behaviour, not a bug.

**Advisory handling**, in order of preference:

1. Upgrade the dependency.
2. If no fixed version exists, assess whether the vulnerable path is
   reachable from apimock's usage.
3. If unreachable, record an ignore entry carrying **the advisory ID,
   the reachability argument, and a review date**.

A bare ignore with no rationale is not acceptable — that is how an audit
gate decays into decoration. If you reach step 3, say so explicitly in
the review request rather than letting it sit in a config file.

### 5.2 Lockfile freshness

`cargo update --workspace --locked` fails when the lockfile does not
match the manifests. This closes RFC 031's Unresolved question 2.

Note it complements RFC 032's `version-consistency-check`, which
compares *versions* across manifest and npm files. This one checks that
`Cargo.lock` is *resolvable and current* against `Cargo.toml`. Different
failure, different check; neither subsumes the other.

### 5.3 Wiring

Both jobs live in `ci.yaml` alongside RFC 031's four, run in parallel
with them, and inherit the same properties: `permissions: contents:
read`, no `continue-on-error`, caching keyed on `Cargo.lock`.

The weekly schedule applies to the audit job. Decide whether the
lockfile job also runs on schedule and say why in the review request —
it cannot change without a commit, so there is a reasonable argument
either way.

## 6. Required tests

Same environment constraint RFC 031 hit: observing a gate fail needs a
real Actions run.

1. **Both checks pass** on the test branch.
2. **Each observed failing independently:**
   - stale lockfile → lockfile check fails, audit unaffected
   - a known-advisory dependency version → audit fails, if one can be
     introduced cheaply. If not, say so rather than contriving
     something; do not fake an advisory.
3. **The weekly schedule trigger fires** and reports.

Use the disposable-branch procedure from
`.git-exclude/reviewed/031-ci-quality-gates/DECISION-001-live-run-evidence.md`.
**Its guardrails apply unchanged: no push to `main`, no tag, no
Release, no pull request.** If you widen the push trigger for testing,
that line must not survive onto `main`.

## 7. Acceptance criteria

1. `cargo audit` and the lockfile check run on push, pull request, and a
   weekly schedule.
2. Both pass on `main`.
3. Each observed failing independently.
4. **No `deny.toml` exists; `cargo-deny` is not invoked.**
5. Any advisory ignore entry carries an ID, a reachability argument, and
   a review date.
6. No dependency changed except to clear a firing advisory.
7. RFC 031's and RFC 032's existing jobs unmodified.

## 8. Prohibited shortcuts

- Adding `cargo-deny` "since we're here" (§ 2).
- `continue-on-error` on either job.
- Silencing an advisory with a bare ignore.
- Upgrading dependencies opportunistically under cover of this RFC.
- Leaving a test-branch trigger widening on `main`.

## 9. Known risks

| Risk | Mitigation |
|---|---|
| A live advisory fires immediately and blocks M1 | Report it — do not ignore it to get green. If it is genuinely unreachable, follow § 5.1 step 3 and say so |
| CI turns red with no commit, from the schedule | Correct behaviour; make sure CONTRIBUTING says so, so it is not mistaken for a broken gate |
| `cargo audit` needs installing on the runner | Implementation detail; state which install method you chose |

## 10. Required evidence

- Green run with both checks.
- Runs showing each failing independently.
- Confirmation the schedule trigger is configured (and fired, if
  observable within the window).
- `git grep -l deny.toml` returning nothing.
- Confirmation the shipped `ci.yaml` has no test-branch trigger.

## 11. Required review-request format

Package at `.git-exclude/review-request/033-supply-chain-gates/` with an
entry-point file a reviewer can open cold. Per § 9.2 of the workflow
document. **Hand back one path — the entry-point file itself.**

Reviewer's focus: whether any advisory was ignored, and on what
reasoning. That is the one place this RFC can quietly become worthless.
