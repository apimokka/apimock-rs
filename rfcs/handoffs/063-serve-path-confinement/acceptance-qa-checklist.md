# Acceptance / QA Checklist — RFC 063, confine the serve path

**Governing RFC.** [RFC 063](../../done/063-serve-path-confinement.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

> 🔒 **EMBARGOED.** Local branch only. No push, no PR, no CI dispatch.
> Commit messages and test names must not describe the vulnerability.

## A. The vulnerability is closed

- [ ] `curl --path-as-is 'http://host/../outside.txt'` → **404**, and a
      test asserts it against a **real running server**.
- [ ] `..` mid-path, a bare `..`, and the `%2e%2e` form are all covered.
- [ ] `respond.file_path` resolving outside `respond_dir` → refused.
- [ ] A Rhai middleware returning an outside path → refused.
- [ ] A symlink escaping the base → refused.
- [ ] Refusals are **404**, never 403 or a distinct error, and reveal
      nothing about whether the target exists.

## B. The tests could fail

- [ ] **Each test written first, observed failing, then fixed.** Report
      both halves — the failing output and the passing output.
- [ ] Confirm the tests fail for the right reason (200-with-content),
      not because the fixture is broken.

## C. Nothing normal broke

- [ ] The W7 acceptance script passes end to end.
- [ ] Every existing `dyn_route` test passes.
- [ ] Files legitimately inside `respond_dir` still serve, including via
      extension inference (`/foo` → `foo.json`).
- [ ] A rule set whose `respond_dir` points at a sibling directory still
      works — that is the supported alternative to an opt-out.

## D. Cost

- [ ] Per-request latency impact measured and reported as a number.
- [ ] The base directory is canonicalised **once at load**, not per
      request.

## E. Track B — the 5.19.1 backport

- [ ] Branched from the `5.19.0` tag / `release/5.19`.
- [ ] **Minimal**: the traversal fix and its tests, nothing else.
- [ ] No v6 work pulled in to make it apply — divergences from Track A
      are listed and explained.
- [ ] Suite green on that branch, against its own baseline (425-era, not
      `main`'s).
- [ ] The diff is reviewable by someone comparing it against `5.19.0`.

## F. Gates (both tracks)

- [ ] Full suite green; counts reported per track.
- [ ] `cargo fmt --all --check` clean.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
