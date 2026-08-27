# Implementation Handoff — pre-6.0.0 documentation, and the docs-build gate

**Source.** `.git-exclude/reviewed/pre-6.0.0-audit/AUDIT-001.md`, items
E-1, B-2, B-3, B-4, F-1.
**Governing rationale.** [RFC 048](../../accepted/048-v6-cli-interface-concept.md)
§ 1 (who U2 is) and § 5 (documentation ranks with the public API).
**Milestone.** 6.0.0. **Blocking for the cut.**
**Baseline.** `main`, after RFC 065 merges. **Do not start § 2–§ 5
before RFC 065 is on `main`** — it changes what `set --json` does, and
these pages must document the new behaviour.
**§ 1 has no such dependency and should land first.**

**Self-contained.** Everything binding is restated here.

---

## The framing that matters

RFC 048 § 5: *"specs and documentation rank with the public API
design… For U2 this is literal — **the documented contract is the only
thing an agent's author can build against.**"*

6.0.0's headline is `get` and `set`. Today the repository front page
never mentions either, there is no guide for either, and a reader
arriving at the docs home is not told the release has an agent user at
all. That is the gap.

This is not a polish pass. Treat it as shipping the interface.

---

## § 1 — E-1: gate the docs build on pull requests (do this first)

`docs.yaml` triggers only on push to `main`; `ci.yaml` never runs
`mdbook`. A change that breaks the docs build is found only *after* it
lands, with the published site already broken.

**Fix:** add an `mdbook build docs` step to `ci.yaml`.

**Do not** add `pull_request` to `docs.yaml`. That workflow builds *and
deploys to GitHub Pages* in one job — triggering it from a PR would
publish unreviewed content. The gate belongs in `ci.yaml`, which already
runs on `pull_request`.

It goes first because everything below is a large documentation change,
and doing that against an ungated build is the wrong order.

- Install `mdbook` (and `mdbook-mermaid` — the docs use mermaid; without
  it the build differs from `docs.yaml`'s).
- Fail the job on a build error.
- Confirm it runs on a pull request, not only on `main`.

## § 2 — B-2: guides for `get` and `set`

`get` and `set` appear only in `cli-reference.md`,
`migrating-to-6-0.md` and `threat-model.md`. **There is no guide for
either**, and the guides index does not list them.

This is an inconsistency, not a policy: guides exist for *"Dry-run a
rule"* (`match-test`) and *"Watch matches live"*. The two commands v6
exists for are the two without one.

Write **two** guides, following the existing task-shaped convention —
one task per page, same voice and structure as the neighbouring guides:

| Page | Covers |
|---|---|
| `docs/src/guides/add-or-change-a-rule.md` | `apimock set rule` — adding a rule, changing one, `--dry-run`, where it writes and how it decides, `--format json` |
| `docs/src/guides/check-what-a-request-returns.md` | `apimock get` — no server, `--why`, `--format json`, and how it differs from `match-test` (which has its own guide) |

Both must:

- Register in `docs/src/SUMMARY.md` **and** `docs/src/guides/README.md`.
- Show `--format json` output, since U2 consumes it. Use the real
  envelope shape (`schema`, `apimock`, `result`/`error`).
- Say what the exit codes mean for the flows shown.
- Cross-link the CLI reference rather than restating it. The reference
  is normative; a guide is a path through it.
- **Use `--json` correctly per RFC 065** — it now writes `respond.json`
  and serves `application/json`.

## § 3 — B-3: `README.md`

**Zero occurrences of `get` or `set`** on the repository front page, for
a release whose headline is those two commands.

Add them where a reader meets the tool — not a changelog line. Keep it
proportionate to the existing README's voice; this is not a rewrite.

## § 4 — B-4: make the agent user visible

`docs/src/README.md` § *"Who is this for?"* lists developers, beginners
and advanced users. Across all 42 doc pages, *"agent"* appears only in
`cli-reference.md` and `threat-model.md`.

RFC 048 § 1 calls U2 *"the user this release is for"*. A reader arriving
at the docs home would never learn that.

Add the agent user to *"Who is this for?"*, in the same register as the
existing entries. Concrete, not aspirational: what an agent does with
apimock — non-interactive commands, machine-readable output, a config it
writes itself — and where to go next (§ 2's guides).

## § 5 — F-1: an example exercising `get` and `set`

Eight examples, each with a matching test file (34 tests). **None
exercises `get` or `set`** — every one is a v5-era server-and-config
scenario.

> **A correction to the audit item, established since it was written.**
> The audit said W7 *"exists only as a test fixture"*, implying RFC 048's
> acceptance criterion was unmet. **It is met.**
> `crates/apimock/tests/set_w7_acceptance.rs` is a real acceptance test
> and `cargo test --workspace` runs it in CI's `test` job. RFC 048's
> *"v6 is finished when W7 is a script that runs in CI"* is satisfied.
>
> **So this item is about readability, not verification.** W7 is
> *checked* but not *readable* — an agent author cannot read it as a
> worked example. Do not rebuild the verification; surface it.

Add `examples/agent-bootstrap/` following the existing example
convention exactly (directory layout, README, and a matching test file
under `crates/apimock/tests/examples/`).

Derive it from `set_w7_acceptance.rs`, which already encodes the
correct ordering and the reasoning behind it — **read its module header
before writing anything.** In particular it explains why the
header-gated rule must be added *before* the unconditional one
(`AddRule` appends; `Strategy::FirstMatch` has no specificity
tie-break), which is exactly the sort of thing an example must get right
or it teaches the wrong lesson.

**Also fix a stale comment in that test's header.** It says `validate`'s
`-c` *"doesn't get RFC 049's bare-relative-path normalisation, a
pre-existing, already-documented gap"*. **RFC 064 fixed that** — a bare
relative `-c` now works on `validate`. The test itself is fine; the
comment describes a gap that no longer exists.

## § 6 — Sweep for the same staleness elsewhere

The comment above is one instance of a pattern this release keeps
hitting: prose describing a limitation that a later RFC closed.

Grep the tree — `docs/`, `README.md`, `examples/`, and **source
comments** — for claims about `get`/`set`/`--json`/`--config` that
RFC 064, its amendment, or RFC 065 have made false. Report what you
find; fix the clear-cut ones.

`docs/src/reference/response-headers.md` was found stale during RFC 065
in exactly this way, so assume there are others.

## § 7 — Not in scope

- Restructuring the docs, the guides index, or the README.
- New features, or any behaviour change. If you find a defect, **report
  it; do not fix it here.**
- `ROADMAP.md` — architect-owned; report anything belonging there.
- Rebuilding W7's verification (§ 5).

## § 8 — Definition of done

- `mdbook build docs` clean, and now **enforced on pull requests**.
- Both guides render, are reachable from `SUMMARY.md` and the guides
  index, and every command shown was **actually run** — no invented
  output.
- The new example has a test, like every other example.
- `cargo test --workspace`, `fmt`, `clippy -D warnings` clean.
- CI green on all 8 jobs (9 once § 1 lands).
- Review-request package in
  `.git-exclude/review-request/pre-6-0-0-documentation/`, including the
  § 6 sweep results.
