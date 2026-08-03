# Implementation Handoff — RFC 031, CI quality gates

**Governing RFC.** [RFC 031](../../done/031-ci-quality-gates.md)
**Milestone.** M1 (Pipeline trust) → v5.15.0
**Status.** Inherited from RFC 031 (Proposed, approved for implementation
2026-08-02)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

**Prerequisite met.** [RFC 030](../../done/030-warning-clean-baseline.md)
is approved and the workspace is warning-clean. This handoff was
deliberately held until that was true rather than predicted.

---

## 1. Purpose

Install blocking format, lint, test, and MSRV checks on every push to
`main` and every pull request, so that the state RFC 030 just achieved
cannot silently decay again.

## 2. Background

Verified on 2026-08-02, post-RFC-030:

```
cargo clippy --workspace --all-targets --all-features -- -D warnings  → exit 0
cargo build --workspace --all-targets                                 → zero warnings
cargo fmt --check                                                     → clean
cargo test --workspace                                                → 371 passed, 0 failed
```

Nothing currently protects that. The only Rust-side CI is a release-time
`cargo build --release --locked`, triggered by `release: created`. The
cost of that gap is measured, not hypothetical: clippy drifted 21 → 26 in
`apimock-routing` between sessions, the workspace-wide total reached 130
unnoticed, `cargo fmt` compliance lapsed across v5.8.0–v5.14.0, and the
recorded test gate ran 212 of 371 tests.

## 3. Change scope

- `.github/workflows/ci.yaml` — new
- `.github/workflows/release-executable.yaml` — add a quality gate (§ 6.3)
- `.github/CONTRIBUTING.md` — document the four commands

## 4. Explicit non-change scope

Do **not**:

- Touch any crate source. If a gate fails, the gate is not the fix —
  escalate.
- Relax a gate to make it pass. `-D warnings` stays; `--workspace` stays.
- Change `.github/workflows/docs.yaml`.
- Change the release workflow's build matrix, published targets, archive
  layout, `permissions`, or `secrets` wiring.
- Modify or remove RFC 032's `version-consistency-check` job — it is
  approved and shipping in the same release. Your job runs alongside it.
- Add `cargo audit` / `cargo deny` / lockfile-freshness checks — that is
  RFC 033, and it is currently blocked on owner decision D-04.

## 5. Applicable requirements

RFC 031 in full. The four mandatory gates:

| Check | Command |
|---|---|
| Format | `cargo fmt --all --check` |
| Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Test | `cargo test --workspace` |
| MSRV | `cargo check --workspace` on the pinned `rust-version` |

## 6. Required implementation

### 6.1 `.github/workflows/ci.yaml`

Triggers: `push` to `main`, `pull_request`, `workflow_dispatch`.

Required properties:

- **`permissions: contents: read`.** CI needs no write scope. Do not copy
  the `contents: write, id-token: write` block from the existing
  workflows — that posture belongs to release and docs, not to CI.
- **`concurrency`** keyed on the ref, with `cancel-in-progress: true`.
- **Four jobs running in parallel**, not chained. A format failure must
  not hide a test failure; a contributor deserves the full picture from
  one run.
- **Dependency caching** on `~/.cargo` and `target/`, keyed on
  `Cargo.lock` — the release workflow's existing `actions/cache` block is
  the reference.

**Toolchain components — a real trap.**
`.github/workflows/scripts/install-rust.sh` runs `rustup set profile
minimal`, which does **not** include `clippy` or `rustfmt`. If you reuse
that script you must add the components explicitly:

```sh
rustup component add clippy rustfmt
```

Reusing the existing script keeps toolchain setup consistent with the
release workflow; using a marketplace action instead is acceptable if you
prefer, but say which you chose and why in the review request.

### 6.2 MSRV job

`Cargo.toml` pins `rust-version = "1.91.0"`. Nothing has ever verified
it — the release workflow installs `stable`, and local development is on
1.97.1.

The job installs exactly `1.91.0` and runs `cargo check --workspace`.
**It does not run tests** — dev-dependencies may legitimately require a
newer toolchain than the library itself.

> **Escalation trigger.** If `cargo check --workspace` fails on 1.91.0,
> **stop and escalate.** Do not raise the pin to make the job pass, and
> do not delete the job. A failing MSRV job means the project's stated
> minimum is already untrue, and correcting a public compatibility claim
> is a design decision, not a CI fix. Report what fails and on which
> crate.

### 6.3 Release-workflow gating — decided, do not re-derive

RFC 031 says the release workflow "gains a dependency on the CI checks".
GitHub Actions cannot express a `needs:` relationship *across* workflows,
so that sentence needs concretising. It is decided here:

**Add a `quality-gate` job inside `release-executable.yaml`** that runs
fmt, clippy, and test, and make `build` depend on both it and RFC 032's
existing job:

```yaml
build:
  needs: [version-consistency-check, quality-gate]
```

Rationale, so it is not re-litigated:

- `workflow_run` triggers fire *after* the fact and cannot block a
  release that is already running.
- Branch protection governs pull requests; a release is triggered by
  `release: created` on a tag and is not covered by it.
- Re-running the gates at release time costs a few minutes and buys a
  guarantee that the artifact was built from a green tree — which is the
  property RFC 031 is actually after.

Duplication between `ci.yaml` and the release workflow's `quality-gate`
is accepted, deliberately. If you can factor the shared steps into a
reusable workflow (`workflow_call`) without obscuring either caller, that
is welcome — but correctness first; do not contort the release workflow
to avoid repeating four commands.

### 6.4 CONTRIBUTING

A short "Before you open a PR" section listing the four commands, so a
contributor can reproduce every gate locally. Note alongside it that
`cargo test --workspace --lib` is a fine fast-feedback command but is
**not** the gate — it runs 212 of 371 tests.

## 7. Required tests

The deliverable is CI configuration, so correctness is demonstrated by
runs, not unit tests.

1. **A fully green run** on a branch — all four checks pass.
2. **A deliberately broken run** — introduce one formatting violation,
   one clippy finding, and one failing test, and show that **each check
   fails independently and the merge is blocked**. This is mandatory. A
   gate observed only in its passing state has not been tested; that is
   the entire lesson of this milestone.
3. **The release workflow refuses to start** with a red gate.
4. Revert all deliberate breakage; the tree ends green.

## 8. Acceptance criteria

1. `ci.yaml` exists, triggering on push to `main`, pull request, and
   manual dispatch.
2. All four checks pass on the merge commit.
3. Each check has been observed failing independently and blocking.
4. `permissions` is `contents: read`.
5. The mandatory test command is `cargo test --workspace`; no workflow or
   document still presents `--lib` as the gate.
6. The MSRV job pins to `[workspace.package].rust-version`.
7. `release-executable.yaml`'s `build` depends on both
   `version-consistency-check` and `quality-gate`.
8. RFC 032's job is unmodified.
9. CONTRIBUTING documents the four commands.

## 9. Prohibited shortcuts

- Weakening any gate to get green. If clippy fails, something regressed —
  find it.
- `continue-on-error: true` on a gate job. That is an advisory gate
  wearing a blocking gate's clothes, and RFC 031 rejected advisory gates
  explicitly.
- Skipping the deliberate-failure runs because the green run passed.
- Bumping `rust-version` to make the MSRV job pass (§ 6.2).
- Adding RFC 033's checks "while you're in there".

## 10. Known risks

| Risk | Mitigation |
|---|---|
| MSRV job fails on 1.91.0 | Escalate per § 6.2 — do not adjust the pin |
| Integration tests bind real sockets and may be flaky on CI runners | If flakiness appears, report it; do not paper over it with retries or `--skip` |
| Release-time gate adds minutes to every release | Accepted; see § 6.3 |
| `install-rust.sh`'s minimal profile omits clippy/rustfmt | § 6.1 |

## 11. Required evidence

- Link/log of the green CI run, all four checks.
- Link/log of the deliberately-broken run, showing each check failing
  **independently**.
- Evidence the release workflow is blocked by a red gate.
- The final `ci.yaml`, and the `release-executable.yaml` diff showing
  `needs:` on both jobs.
- Confirmation `cargo test --workspace` still reports 371.

## 12. Required review-request format

Package at `.git-exclude/review-request/031-ci-quality-gates/` with an
entry-point file that a reviewer can open cold. Per § 9.2 of the workflow
document: implementation summary, addressed requirements, changed files,
implementation decisions, deviations, executed tests and results,
build/static-analysis results, unresolved issues, known limitations,
requested review focus.

Hand back **one path** — the entry-point file itself.

Reviewer's focus will be **the deliberate-failure evidence** — whether
each of the four gates was genuinely observed blocking, one at a time.
That is the only thing that distinguishes this RFC from a configuration
file that happens to be green.
