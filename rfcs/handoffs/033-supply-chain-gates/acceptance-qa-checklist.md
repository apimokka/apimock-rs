# Acceptance / QA Checklist — RFC 033

**Governing RFC.** [RFC 033](../../done/033-supply-chain-gates.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Paste actual run output and links into the review-request package.

---

## Scope discipline — D-04

- [ ] **No `deny.toml` anywhere** — `git grep -l deny.toml` returns
      nothing
- [ ] `cargo-deny` is not invoked in any workflow
- [ ] No licence allow-list, no bans/duplicates policy, no sources
      policy
- [ ] Exactly **two** new jobs, not three

## The two checks

- [ ] `cargo audit` job present
- [ ] Lockfile job runs `cargo update --workspace --locked`
- [ ] Both blocking — no `continue-on-error`
- [ ] Both inherit `permissions: contents: read`
- [ ] Caching keyed on `Cargo.lock`, consistent with RFC 031's jobs
- [ ] Audit job also runs on a weekly `schedule` trigger
- [ ] Stated in the review request whether the lockfile job runs on
      schedule too, and why

## Green run

- [ ] Both checks pass on the test branch
- [ ] Both pass on `main` at the merge commit
- [ ] Run links captured

## Deliberate-failure runs

- [ ] Stale lockfile → **lockfile check fails**, audit unaffected
- [ ] Known-advisory dependency → **audit fails** — or an explicit
      statement that one could not be introduced cheaply. Do **not**
      fabricate an advisory
- [ ] Each failure observed independently
- [ ] All deliberate breakage reverted; tree ends green
- [ ] Run links captured for each

## Advisory handling — the reviewer's focus

- [ ] If **no advisory fired**: state that plainly
- [ ] If one fired and was **fixed by upgrade**: name the crate, the
      advisory ID, and the version moved to
- [ ] If one fired and was **ignored**: the entry carries an advisory
      ID, a written reachability argument, and a review date — and it
      is called out in the review request body, not left in a config
      file
- [ ] No bare ignore entries

## Non-change scope

- [ ] No crate source touched
- [ ] No dependency added, removed, or upgraded except to clear a
      firing advisory
- [ ] `[workspace.dependencies]` pins untouched for tidiness
- [ ] RFC 031's four jobs unmodified
- [ ] RFC 032's `version-consistency-check` unmodified
- [ ] `docs.yaml` unmodified

## Guardrails — Decision 001, still in force

- [ ] `main` not pushed
- [ ] No tag created
- [ ] No GitHub Release created, draft or published
- [ ] No pull request opened
- [ ] Disposable branch deleted after use
- [ ] Shipped `ci.yaml` contains **no** test-branch trigger

## Documentation

- [ ] `.github/CONTRIBUTING.md` lists the two commands alongside RFC
      031's four
- [ ] It notes that a scheduled audit run can turn CI red with no new
      commit, and that this is correct behaviour

## Review-request package

- [ ] Created at `.git-exclude/review-request/033-supply-chain-gates/`
- [ ] Entry-point file orients a reviewer with no prior context
- [ ] Contains all 10 items from § 9.2 of the workflow document
- [ ] States which install method was used for `cargo audit`
- [ ] Hand back **one path** — the entry-point file itself
