# Acceptance / QA Checklist — RFC 036

**Governing RFC.** [RFC 036](../../proposed/036-example-configs.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Paste actual command output into the review-request package.

---

## Content rules

- [ ] Every example runs as-is, from its own directory, without editing
      paths
- [ ] **Zero commented-out feature demonstrations** anywhere in the set
- [ ] No `"hej ab"` / `"hejhej cd"` placeholder responses remain
- [ ] Resource paths, JSON bodies, and header names resemble a real API
- [ ] Body paths use the dotted mini-syntax; no `$.`-prefixed
      pseudo-JSONPath
- [ ] At least one **working** middleware example (current count: zero)
- [ ] Each set has a README: purpose, run command, `curl` + expected
      response

## Verification mechanism — the reviewer's focus

- [ ] Mechanism chosen and **stated in the review request**, with why
- [ ] It runs every example set
- [ ] It asserts every documented request/response pair
- [ ] It runs green — output captured
- [ ] **It fails when an example is deliberately broken** — output
      captured. A check seen only passing has not been tested
- [ ] Not manual-only

## `--init` and release archives

- [ ] `apimock --init` produces a working config on a clean directory,
      verified by running it
- [ ] `--init` output is still **minimal** — a starting point, not a
      feature tour
- [ ] If `config/default/` filenames changed,
      `.github/workflows/release-executable.yaml` copy steps updated in
      the same change
- [ ] `apimock validate` passes on every example config

## Non-change scope

- [ ] `crates/apimock/examples/config/tests/` untouched
- [ ] `crates/apimock/examples/bench_load.rs` untouched
- [ ] **No crate source changed**
- [ ] No `--init --template` flag added
- [ ] No prose written into `docs/`
- [ ] No product behaviour changed

## Regression

- [ ] `cargo test --workspace` — **371 passed, 0 failed**
- [ ] `cargo fmt --all --check` clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0
- [ ] If the verification mechanism adds tests, the count change is
      stated and explained

## Escalations to report

- [ ] Any feature that **could not be exemplified** — missing, broken, or
      awkward to configure. This is a product finding, not a failure of
      the task
- [ ] Any defect an example revealed. Report; do not fix crate source
- [ ] Any conclusion that `--init --template` is needed

## Review-request package

- [ ] Created at `.git-exclude/review-request/036-example-configs/`
- [ ] Entry-point file orients a reviewer with no prior context
- [ ] Contains all 10 items from § 9.2 of the workflow document
- [ ] States the verification mechanism and the reasoning
- [ ] States which candidate tasks were adopted, which were not, and why
- [ ] Hand back **one path** — the entry-point file itself
