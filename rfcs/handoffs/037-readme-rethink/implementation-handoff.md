# Implementation Handoff — RFC 037, README rethink

**Governing RFC.** [RFC 037](../../proposed/037-readme-rethink.md)
**Milestone.** M2 → **release-bearing**: with RFC 036, defines v5.16.0
**Status.** Inherited from RFC 037 (Proposed)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. Purpose

Rewrite `README.md` so every claim is true, the structure matches the
project's own rule, and it hands off to the docs site instead of
competing with it.

## 2. Why this one is release-bearing

`crates/apimock/Cargo.toml:4` declares `readme = "../../README.md"`.
This file **ships inside the published crate** and renders as the
crates.io landing page. Unlike the other M2 documentation RFCs, it
changes a release artifact.

Practical consequence: **repository-relative links break on crates.io.**
The current `![logo](docs/src/assets/logo.png)` already has this
problem — it resolves on GitHub and 404s on crates.io. Use absolute
URLs for anything outside the README.

## 3. Change scope

`README.md` — nothing else. Not `docs/`, not `CONTRIBUTING.md`, not
crate source.

## 4. Explicit non-change scope

Do **not**:

- Touch anything under `docs/src` — RFCs 035 and 038.
- Touch `crates/apimock/examples/` — RFC 036, already implemented.
- Change badges, the logo image itself, or the licence policy.
- Re-run or fabricate load-test evidence (§ 6.2).
- Change crate source.

## 5. Applicable requirements

RFC 037 in full, plus the project's six-section README structure:
Hero · Overview · Why/When · Quick start · **Features / Design Notes** ·
More detail.

## 6. Required implementation

### 6.1 Remove both `4.7.0` references

`README.md:97` and `:100`. Describe `--init --yes` by **what it does**,
not by which release introduced it. A reader does not care that 4.7.0
established the defaults; they care what the defaults are.

### 6.2 Remove the k6 claim

`README.md:52` — *"as validated with k6 load testing"*.

No k6 script, log, or result exists anywhere in the repository. It has
been carried as RISK-004 since the v5.14.0 handoff and never
substantiated.

**Keep the surrounding claims** — no preloading, per-request non-blocking
reads, flat memory. Those are true and verifiable from the code. Only the
attribution to a load test that cannot be produced goes.

**Do not attempt to generate the evidence.** That is explicitly out of
scope. If you think the claim is worth keeping, that is a design request,
not a licence to run benchmarks.

### 6.3 Add § 5 — Features / Design Notes

The section the structure rule specifies and the README lacks. Its
absence is why a reader cannot tell what apimock is *like to work with*.

Per the rule: *"Features can be moved to the full docs, leaving only
Design Notes here."* So this is not a feature list. Candidates:

- the read-on-demand response model and what it buys;
- file-then-rules dispatch order;
- the dotted-path body syntax, **explicitly not JSONPath** — this has
  caused a real bug before (ROADMAP § History);
- that `apimock validate` and `apimock match-test` exist, with a link.

Where the design-notes/feature-list line falls is a judgement call.
**Propose it and justify it** in the review request — RFC 037's
Unresolved question 2 leaves it deliberately open.

### 6.4 Complete Acknowledgements

Missing: `rustls`, `tokio-rustls`, `csv`, `regex`, `globset`, `ignore`,
`uuid`, `indexmap`. `rustls` carries the whole TLS surface.

Cross-check against `[workspace.dependencies]` rather than adding only
these eight — the list should be derived, not patched.

### 6.5 Add `cargo install apimock` to Quick start

A crates.io badge and a docs.rs badge sit at the top of a README whose
Quick start covers only npm. crates.io is the channel that has stayed
current.

### 6.6 Link targets — check immediately before merging

RFC 034 restructures the docs site and **RFC 034 D6 declines to provide
redirects**. Every docs link must point at a page that exists *at the
moment this merges*.

RFCs 035 and 038 are landing sections in parallel. If a target page is
not yet there, **link the section index instead of a page that 404s**.

This is the one ordering hazard in an otherwise independent task. Re-run
the link check as the last thing you do.

## 7. Required tests

1. Every link resolves — including as rendered on crates.io, i.e. **no
   repository-relative links** to anything outside the README.
2. No version reference to any release other than the current one.
3. Every claim traceable to code or a checked artifact.
4. Six sections present, in order.
5. Every shipped dependency appears in Acknowledgements.
6. `cargo package -p apimock` succeeds and the packaged README renders
   without broken relative links.

## 8. Acceptance criteria

1. No `4.7.0` reference remains.
2. No k6 claim remains; surrounding performance claims retained.
3. § 5 Features / Design Notes exists, and is design notes rather than a
   feature enumeration.
4. Acknowledgements derived from `[workspace.dependencies]`.
5. `cargo install apimock` documented alongside `npx apimock`.
6. All links resolve, including from a crates.io rendering.
7. The README is **not longer** than it was — feature detail moved out,
   not added.
8. Nothing outside `README.md` changed.

## 9. Prohibited shortcuts

- Keeping the k6 claim because it reads well.
- Generating load-test evidence to justify keeping it (§ 6.2).
- Growing § 5 into the feature list the docs are supposed to own.
- Linking a docs page that does not exist yet (§ 6.6).
- Leaving repository-relative links that break on crates.io.

## 10. Known risks

| Risk | Mitigation |
|---|---|
| A docs link 404s because 035/038 have not landed that page | § 6.6 — link section indexes; re-check last |
| § 5 grows into a feature list | The rule is explicit; justify the line in the review request |
| Removing the k6 claim weakens the pitch | Accepted and deliberate — see RFC 037 Drawbacks |

## 11. Required evidence

- Link-check output over the final README, including which links are
  absolute and why.
- `cargo package -p apimock` output.
- Before/after line counts, demonstrating it did not grow.
- The derived Acknowledgements list next to `[workspace.dependencies]`.
- A statement of where you drew the design-notes/feature-list line.

## 12. Required review-request format

Package at `.git-exclude/review-request/037-readme-rethink/` with an
entry-point file a reviewer can open cold. Per § 9.2 of the workflow
document. **Hand back one path — the entry-point file itself.**

Reviewer's focus: whether every remaining claim is checkable, and where
the § 5 line was drawn.
