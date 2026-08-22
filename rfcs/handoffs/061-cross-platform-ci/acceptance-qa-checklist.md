# Acceptance / QA Checklist — RFC 061, cross-platform CI

**Governing RFC.** [RFC 061](../../accepted/061-cross-platform-ci.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

## A. The matrix

- [ ] `test` job runs on `ubuntu-latest`, `windows-latest`, `macos-latest`.
- [ ] `fail-fast: false` is set, and **demonstrated** — one platform
      failing did not cancel the others. Show the run.
- [ ] `fmt`, `clippy`, `msrv`, `audit`, `lockfile` still Ubuntu-only.
- [ ] Trigger is every push, as today. Not release-only.

## B. Results

- [ ] Per-platform test results pasted, not summarised.
- [ ] **The W7 acceptance script** passes on all three, or its failures
      are listed per platform.
- [ ] Ubuntu results identical to today's baseline — this change must
      not alter what already worked.
- [ ] The jobs genuinely ran the suite (test counts match), rather than
      skipping and reporting green.

## C. Findings

- [ ] Every failure listed with platform, test name, and a judgement:
      product defect or test-environment artefact.
- [ ] Product defects **reported, not fixed here** — except one-line
      fixture issues, which are called out individually.
- [ ] Any test made platform-conditional is named, with the reason.
- [ ] **No platform was disabled to obtain green.**

## D. Cost

- [ ] `test`-job wall-clock reported as a number, before and after.
- [ ] If the cost looks unacceptable, a narrower trigger is *proposed*
      rather than applied unilaterally.

## E. Gates

- [ ] Ubuntu: full suite green, count against `main`'s baseline.
- [ ] `cargo fmt --all --check` and `clippy … -D warnings` clean.
- [ ] No production code changed (or, if a fixture fix was unavoidable,
      it is called out).
