# Acceptance / QA Checklist — RFC 037

**Governing RFC.** [RFC 037](../../done/037-readme-rethink.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Paste actual output into the review-request package.

---

## The four defects

- [ ] Both `4.7.0` references gone (`README.md:97`, `:100`);
      `--init --yes` described by what it does
- [ ] `"as validated with k6 load testing"` gone
- [ ] Surrounding performance claims **kept** — no preloading,
      per-request non-blocking reads, flat memory
- [ ] No load-test evidence was generated to justify keeping it
- [ ] § 5 Features / Design Notes exists
- [ ] Acknowledgements derived from `[workspace.dependencies]`, not
      patched with the eight named crates
- [ ] `rustls` and `tokio-rustls` present

## Structure

- [ ] Six sections, in order: Hero · Overview · Why/When · Quick start ·
      Features / Design Notes · More detail
- [ ] § 5 is **design notes**, not a feature enumeration
- [ ] Where that line was drawn is stated and justified in the review
      request
- [ ] README is **not longer** than before — before/after line counts
      captured

## Install paths

- [ ] `cargo install apimock` documented alongside `npx apimock`
- [ ] `apimock validate` and `apimock match-test` mentioned, with a link
      to the docs

## Links — the ordering hazard

- [ ] Every link resolves **at merge time**
- [ ] No repository-relative links to anything outside the README
      (they 404 on crates.io)
- [ ] `docs/src/assets/logo.png` converted to an absolute URL
- [ ] Where a docs page from RFC 035/038 does not exist yet, the
      **section index** is linked instead
- [ ] Link check re-run as the **last** action before submitting

## Crate packaging

- [ ] `cargo package -p apimock` succeeds
- [ ] Packaged README inspected — no broken relative links in the
      crates.io rendering

## Non-change scope

- [ ] Nothing under `docs/src` changed
- [ ] `crates/apimock/examples/` untouched
- [ ] No crate source changed
- [ ] Badges, logo image, and licence policy unchanged
- [ ] **Only `README.md` differs**

## Regression

- [ ] `cargo fmt --all --check` clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0
- [ ] `cargo test --workspace` — **409 passed, 0 failed**

## Escalations to report

- [ ] Any claim you could not verify against code or an artifact —
      report rather than keeping or silently deleting
- [ ] Any docs link with no valid target even at section-index level
- [ ] Any conclusion that the k6 claim should be retained

## Review-request package

- [ ] Created at `.git-exclude/review-request/037-readme-rethink/`
- [ ] Entry-point file orients a reviewer with no prior context
- [ ] Contains all 10 items from § 9.2 of the workflow document
- [ ] States where the design-notes / feature-list line was drawn
- [ ] Hand back **one path** — the entry-point file itself
