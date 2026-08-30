# Implementation Handoff — `npm-core-publish` must wait for propagation

**Source.** `post-6-0-0-release-process` handoff § 1, which you correctly
declined to implement; the 6.0.0 release's own partial-publish failure.
**Authorisation.** [RFC 066](../../done/066-branching-and-merge-policy.md)
**Amendment 2, adopted 2026-08-30** — § 2's `release-publish.yaml`
clause now excludes *"a wait, retry or poll that changes nothing about
what publishes, in what order, or with what credentials."* **This change
is inside that carve-out and nothing else.**
**Milestone.** Before the next release.
**Baseline.** `main` @ `826e5b7`.

**Self-contained.**

---

## 0. You were right to stop last time. This is the unblock.

Your escalation was correct: my previous handoff asked for an edit
RFC 066 § 2 prohibited, and never mentioned the prohibition. Rather than
authorise one instance, the boundary moved — Amendment 2 now permits a
wait/retry/poll here, and nothing more.

**The carve-out is the scope.** If you find yourself changing what
publishes, in what order, or with what credentials, you have left it —
stop and report, exactly as you did before.

## 1. What is wrong

On 6.0.0, `npm-core-publish` failed and left a **partially published
release**: three platform packages live on npm, the core package
absent, and `crates-io-publish` never ran (it is `needs:
npm-core-publish`).

```
npm error code EUSAGE
npm error `npm ci` can only install packages when your package.json and
npm error package-lock.json ... are in sync.
```

**Mechanism.** The job runs, with no wait of any kind:

```yaml
npm install --package-lock-only --ignore-scripts
npm ci --ignore-scripts
npm publish
```

**npm silently omits `optionalDependencies` it cannot resolve.** The
three `@apimock-rs/bin-*@X.Y.Z` packages had been published seconds
earlier and had not propagated, so the generated lock was written
without all three, and `npm ci` rejected it against a `package.json`
that declares them.

**It is a race, and it is narrow.** Gap between the last platform
publish completing and the core job starting:

| Release | Gap | Result |
|---|---|---|
| 5.19.0 | 4s | ✅ |
| 5.19.1 | 3s | ✅ |
| 4.8.2 | 3s | ✅ |
| **6.0.0** | **2s** | ❌ |

The step is unchanged since RFC 044, so the eight prior successes were
the race being *won*, not absent. A rerun 113s later succeeded.

## 2. What to build

**Poll until all three platform versions resolve, then proceed.**
Before `npm install --package-lock-only`, wait until each of

- `@apimock-rs/bin-linux-x64-gnu`
- `@apimock-rs/bin-darwin-arm64`
- `@apimock-rs/bin-win32-x64-msvc`

is visible at the release version — e.g. `npm view <pkg>@$TAG version`
returning that version.

**Why poll rather than retry the `npm ci` pair:** it waits for the
actual precondition, and its failure message can name *which* package
never appeared. Retrying past a symptom tells you only that something
was wrong.

**Match `verify-published`'s existing budget — six attempts, 15s
apart** — rather than inventing a second convention. That job already
solves npm propagation for the same registry; one convention in the
file, not two.

**On exhaustion: fail, loudly, naming the package.** Do not proceed to
publish. A core package published against a lock missing its platform
binaries is worse than a failed job.

## 3. Do not

- Relax `npm ci` to `npm install`, or commit a `package-lock.json`.
  `npm ci`'s strictness is what catches a genuinely out-of-sync
  manifest. The defect is that it was asked the question too early, not
  that it answered wrongly.
- Change publish order, what is published, or any credential/`id-token`
  configuration. Outside Amendment 2's carve-out.
- Touch `npm-platforms-publish` or `crates-io-publish`.

## 4. Acceptance

- [ ] The wait is in place, using six attempts 15s apart
- [ ] **Demonstrate it actually waits.** A wait that never waits is
      untested. Point it at a version that will never appear and show it
      polls, then fails naming the package — the fast path passing tells
      you nothing about the slow path
- [ ] The fast path is not meaningfully slowed when all three are
      already visible
- [ ] On exhaustion the job **fails** and does not publish
- [ ] `npm ci` remains `npm ci`; no lockfile committed
- [ ] `git diff` on `release-publish.yaml` shows **only** the wait —
      quote the diff in the package so the carve-out is checkable at a
      glance
- [ ] CI green before merge

> **This job cannot be exercised end to end without publishing.** That
> is inherent, not an excuse — say plainly which parts you proved and
> which you could not, and how you convinced yourself of the rest.
> Extracting the polling into a script under
> `.github/workflows/scripts/` that can be run directly against the real
> registry would make most of it testable; your call whether that is
> proportionate.

## 5. Report back

`.git-exclude/review-request/post-6-0-0-npm-propagation-wait/`,
including the `release-publish.yaml` diff and how you established § 4's
"it actually waits".
