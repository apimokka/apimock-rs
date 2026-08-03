# Acceptance / QA Checklist — RFC 031

**Governing RFC.** [RFC 031](../../proposed/031-ci-quality-gates.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Paste actual run output and links into the review-request package, not a
summary of them.

---

## Workflow structure

- [ ] `.github/workflows/ci.yaml` exists
- [ ] Triggers: `push` to `main`, `pull_request`, `workflow_dispatch`
- [ ] `permissions: contents: read` — **not** `write`, and no
      `id-token: write`
- [ ] `concurrency` keyed on the ref, `cancel-in-progress: true`
- [ ] Four jobs, running **in parallel** (no `needs:` chain between them)
- [ ] Cargo/target caching keyed on `Cargo.lock`
- [ ] `clippy` and `rustfmt` components explicitly installed (the
      existing `install-rust.sh` uses a **minimal** profile and omits
      them)
- [ ] No `continue-on-error` on any gate job

## The four gates

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace` — reports **371 passed, 0 failed**
- [ ] MSRV: `cargo check --workspace` on `1.91.0`, pinned from
      `[workspace.package].rust-version`, tests **not** run

## Green run

- [ ] All four checks pass on the merge commit
- [ ] Run link/log captured

## Deliberate-failure run — mandatory

The reviewer's primary focus. A gate seen only passing has not been
tested.

- [ ] One formatting violation → **format check fails**, others unaffected
- [ ] One clippy finding → **lint check fails**, others unaffected
- [ ] One failing test → **test check fails**, others unaffected
- [ ] Each failure **blocks the merge**
- [ ] All deliberate breakage reverted; tree ends green
- [ ] Run links/logs captured for each

## Release workflow

- [ ] `build` declares `needs: [version-consistency-check, quality-gate]`
- [ ] RFC 032's `version-consistency-check` job is **byte-for-byte
      unmodified**
- [ ] A red quality gate prevents the release from proceeding
- [ ] Build matrix, published targets, archive layout, `permissions`, and
      `secrets` wiring all unchanged

## Non-change scope

- [ ] No crate source modified
- [ ] `.github/workflows/docs.yaml` unmodified
- [ ] No `cargo audit` / `cargo deny` / lockfile check added (RFC 033,
      blocked on D-04)
- [ ] No gate command weakened — `-D warnings` and `--workspace` intact

## Documentation

- [ ] `.github/CONTRIBUTING.md` lists the four commands
- [ ] It states that `--lib` is fast local feedback but **not** the gate
      (212 of 371 tests)
- [ ] No workflow or document still presents `--lib` as the gate

## Escalations to report

- [ ] **MSRV job fails on 1.91.0** — report which crate and what error.
      Do **not** raise the pin, do **not** delete the job.
- [ ] Integration-test flakiness on CI runners — report it; do not add
      retries or skips
- [ ] Anything requiring a crate-source change

## Review-request package

- [ ] Created at `.git-exclude/review-request/031-ci-quality-gates/`
- [ ] Entry-point file orients a reviewer with no prior context
- [ ] Contains all 10 items from § 9.2 of the workflow document
- [ ] Deliberate-failure evidence is prominent, not buried
- [ ] States whether `install-rust.sh` or a marketplace action was used
      for toolchain setup, and why
- [ ] Hand back **one path** — the entry-point file itself
