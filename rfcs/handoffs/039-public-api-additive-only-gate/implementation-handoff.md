# Implementation Handoff — RFC 039, the additive-only public-API gate

**Governing RFC.** [RFC 039](../../accepted/039-public-api-additive-only-gate.md)
— approved 2026-08-20, deliberately held until after 6.0.0.
**Milestone.** 6.x. **Not blocking anything; time-sensitive for one
specific reason — see § 1.**
**Baseline.** `main` @ `7249fdc`.

**Self-contained.**

---

## 1. Why now, and why the timing genuinely matters

RFC 039 § "When this turns on" is explicit:

> *"After 6.0.0 ships, as the first gate of the 6.x line. … Baseline
> the API **at the 6.0.0 tag** and let the first 6.x change be the
> first thing the gate sees."*

6.0.0 shipped on 2026-08-28. **There are still zero source changes
since the tag** — verified today: `git diff --name-only 6.0.0..main`
matches no `crates/**/*.rs`.

So the baseline you generate now *is* the 6.0.0 public API, provably,
with no reconstruction or judgement involved. Every commit that touches
a public type before this lands makes that harder to say. Nothing
breaks if it slips — the baseline just stops being self-evidently the
released surface.

**Generate the baseline from `main` as it stands.** If `main` has moved
into `crates/**` by the time you start, **stop and report** rather than
baselining a surface that is no longer the released one; that changes
what the first diff means.

## 2. What to build

### Tool and shape

**`cargo-public-api`**, with the baseline **checked into the
repository** — one file per crate, `crates/<name>/public-api.txt`.

The checked-in file is the whole point, and RFC 039 says why:

> *"A tool that only diffs against the previous release tells CI; **a
> file in the diff tells the reviewer**, in the pull request, next to
> the change that caused it."*

So: **nothing is auto-updated.** A commit that changes the API contains
the baseline change, which makes `git log` on that file the API's
changelog.

### The job

1. Build the current public API for each of the four crates.
2. Diff against the checked-in baseline.
3. Identical → pass.
4. Differs → **fail, printing the diff**, with a message naming the two
   valid responses: *update the baseline in this commit (declaring the
   change), or undo the change.*

### Toolchain — the part most likely to be got wrong later

`cargo-public-api` needs **nightly** (it builds rustdoc JSON). The
workspace pins `rust-version = "1.91.0"` and CI has an `msrv` job.

**These do not conflict**, and RFC 039 asks for the reason to be written
into the job's own comment so nobody later "fixes" the inconsistency:

> The API job proves nothing about what compiles for users. It is an
> inspection tool, on its own toolchain, in its own job. **`msrv`
> remains the authority on what the crates support.**

**Pin the nightly to a dated version** and bump it deliberately. An
unpinned nightly makes this the flakiest job in CI, and a gate that
fails for unrelated reasons is a gate people bypass — which this repo
has now seen twice (the `docs` job's rate limit; the `package` job's
first, unconditional shape).

## 3. Acceptance — prove the gate fires

RFC 039 § Testing is unusually specific, because a gate that is only
ever green is indistinguishable from one that does nothing:

- [ ] **A removed public method makes the job fail**, with a readable
      diff. Prove it fires; do not just show CI green.
- [ ] **An additive change also fails** until the baseline is updated —
      *additive-only means declared, not unchecked.* This is the one
      most likely to be missed, because "additive is safe" is the
      intuition the RFC is deliberately overriding.
- [ ] **An internal-only change produces no diff** — rename a private
      fn, move a module between files. This is Goal 3, zero false
      positives, and it is what makes the gate survivable.
- [ ] All four crates covered: `apimock`, `apimock-config`,
      `apimock-routing`, `apimock-server`.
- [ ] **Report the job's runtime.** If it materially slows CI, say so
      rather than absorbing it — RFC 031 made that trade explicitly and
      the same applies.
- [ ] The nightly is pinned to a dated version, and the job comment
      explains the `msrv` relationship.
- [ ] CI green on all jobs before merge (11 once this lands).

Revert every probe before committing, and confirm with `git diff`.

## 4. Not in scope

- Changing any public API. If generating the baseline reveals something
  that looks wrong — an accidentally-`pub` item, a leaked internal type
  — **report it, do not fix it.** That is a finding, and possibly a
  valuable one; it is not this task.
- `release-publish.yaml` or anything on the publish path.
- Version bumps, CHANGELOG, README.

## 5. Report back

`.git-exclude/review-request/039-public-api-additive-only-gate/`,
including:

- The three acceptance demonstrations (removed / additive / internal),
  with the actual diff output for the first two.
- The job's runtime.
- **Anything the baseline revealed about the public surface** — § 4.
  A first look at four crates' full public API in one file is likely to
  show something nobody has looked at directly before.
