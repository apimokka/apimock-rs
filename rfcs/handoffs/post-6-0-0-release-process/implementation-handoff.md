# Implementation Handoff — harden the release process: fail before the release, not during it

**Source.** `.git-exclude/release/6.0.0/PRE-CUT-AUDIT.md` § 4; the 6.0.0
release's own `npm-core-publish` failure.
**Milestone.** Before the next release. **Not blocking anything today.**
**Baseline.** `main` @ `21bce69`.

**Self-contained.** Three items, one theme: **every one of them is a
failure the release process currently discovers *during* a live
release.** That pattern has now cost this project a blocked release
three times and a partial publish once.

---

## 1. `npm-core-publish` races npm's propagation — and will lose again

### What happened on 6.0.0

`npm-core-publish` failed. Three platform packages were live on npm, the
core package was not, and `crates-io-publish` never ran — a **partially
published release**. A rerun 113 seconds later succeeded.

```
npm error code EUSAGE
npm error `npm ci` can only install packages when your package.json and
npm error package-lock.json ... are in sync.
```

### Why, exactly

The job is `needs: npm-platforms-publish` with **no wait, no retry and
no availability check**, then runs:

```yaml
npm install --package-lock-only --ignore-scripts
npm ci --ignore-scripts
npm publish
```

**npm silently omits `optionalDependencies` it cannot resolve.** If
`@apimock-rs/bin-*@X.Y.Z` has not propagated yet, the generated lock is
written *without all three*, and `npm ci` then compares it against a
`package.json` that declares them and rejects it.

### It is a race, and the margin is measured

Gap between the last platform publish completing and the core job
starting:

| Release | Gap | Result |
|---|---|---|
| 5.19.0 | 4s | ✅ |
| 5.19.1 | 3s | ✅ |
| 4.8.2 | 3s | ✅ |
| **6.0.0** | **2s** | ❌ |

The design has always allowed npm about three seconds. The step is
unchanged since RFC 044, so **the eight prior successes were the race
being won, not the race being absent.** Nothing has changed; the next
release runs the same gamble.

### The fix — a pattern this file already contains

`verify-published` already retries `npm pack` **six times over 90s**
because npm propagation is not instant. Apply the same idea one job
earlier. Either:

- **Poll until the three platform versions resolve** before generating
  the lock — e.g. `npm view @apimock-rs/bin-<plat>@$TAG version` for each,
  retrying with a delay, then proceed; or
- **Retry the `npm install --package-lock-only` + `npm ci` pair** on
  failure.

Prefer the first: it waits for the actual precondition rather than
retrying past a symptom, and its failure message can say *which*
package never appeared.

**Match `verify-published`'s existing budget** (six attempts, 15s apart)
rather than inventing a new one — one propagation-wait convention in the
file, not two.

**Do not** relax `npm ci` to `npm install`, and do not commit a
`package-lock.json`. `npm ci`'s strictness is what catches a genuinely
out-of-sync manifest; the defect is that it was asked the question too
early, not that it answered wrongly.

## 2. `RELEASING.md` has no npm recovery procedure

§ "If crates.io publishing fails partway" covers crates.io only. The npm
case has **no written procedure at all** — the next person to hit it
starts from nothing, mid-release.

Add a short section covering:

- **How to tell what actually published.** Check the registry, not the
  job log — `npm view <pkg> version` for all four packages. This mirrors
  the crates.io section's own first instruction, and for the same reason.
- **The recovery used on 6.0.0**: `gh run rerun <run-id> --failed`.
  It re-runs only the failed job and its dependents, so the three
  already-published platform packages are **not** re-attempted — which
  matters, because re-publishing an existing npm version fails.
- **That a partial npm publish is not self-healing**: `crates-io-publish`
  is downstream of `npm-core-publish`, so an npm failure silently means
  crates.io published nothing.

## 3. No release gate packages the crates

`RELEASING.md` § "What each release gate checks" lists exactly two:
`version-consistency-check` and `quality-gate` (`fmt`, `clippy`,
`test`). **Neither runs `cargo package`.**

So a packaging failure — an excluded file a crate needs, a manifest
problem — surfaces during `cargo publish`, after the tag and after the
draft Release. That is the same "discovered during a live release" class
as § 1.

I ran it by hand before the 6.0.0 cut and it passed. It should not need
a human to remember.

**Add `cargo package --workspace` to `ci.yaml`.**

> **This is the part to get right.** Run at the *current* version it
> **fails**, and correctly: packaging `apimock-config` verifies its
> tarball by building against `apimock-routing` **from crates.io at the
> published version**, which predates the workspace's own unreleased
> changes. Only an unpublished version resolves cleanly, via cargo's
> co-packaging temp registry.
>
> So a naive `cargo package --workspace` job would **fail on every
> commit between releases** and be disabled within a week.
>
> Establish the right shape before implementing, and say what you chose:
> run it only when the workspace version is unpublished, only on the
> release path, or something better. **If you conclude there is no shape
> that is both meaningful and non-annoying, say so and stop** — that is
> a legitimate finding, and better than a gate people learn to ignore.

## 4. Not in scope

- Changing what gets published, or the publish order.
- Trusted Publishing configuration — registry-side, owner-only.
- Any crate source change. All three items are CI and documentation.

## 5. Acceptance

- [ ] § 1: the propagation wait is in place, using `verify-published`'s
      existing retry budget; its failure message names the package that
      never resolved
- [ ] § 1: **demonstrate it works when the precondition is already met**
      (a normal run is not slowed meaningfully) and **that it waits**
      when it is not — describe how you established the second, since a
      wait that never waits is untested
- [ ] § 2: `RELEASING.md` gains the npm recovery section
- [ ] § 3: either a packaging gate that does not fire spuriously between
      releases, **or** a written finding explaining why not
- [ ] `cargo test --workspace`, `fmt`, `clippy -D warnings`,
      `mdbook build docs`
- [ ] CI green before merge — including any job you add

## 6. Report back

`.git-exclude/review-request/post-6-0-0-release-process/`, including how
you established § 1's wait actually waits, and § 3's decision with its
reasoning.
