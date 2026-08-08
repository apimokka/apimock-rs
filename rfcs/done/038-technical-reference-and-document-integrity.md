# RFC 038 — Technical reference refresh and document integrity

**Status.** Proposed
**Tracks.** M2. Executes RFC 034's page map for the explanatory and
contributor halves — How it works, Contributing — and clears the
document-integrity defects found across this milestone.
**Touches.** `docs/src/` How it works and Contributing sections, their
`SUMMARY.md` entries, and `CHANGELOG.md` (one duplicate entry). Not
Getting started / Guides / Reference (RFC 035), not `README.md`
(RFC 037).

## Summary

Three things:

1. **How it works** — including the page that did not exist in any form:
   matching order and precedence.
2. **Contributing** — a section serving a persona the documentation
   currently fails completely.
3. **Document integrity** — the defects accumulated across M2's surveys.

## Motivation

### 1. The technical reference describes software that no longer exists

`docs/src/technical-reference/architecture.md` (11 lines) describes the
**pre-5.0.0 single-crate layout**:

> `src/config.rs` · `src/server.rs` · `src/core/server/routing.rs` · `tests/`

None of those paths have existed for fifteen releases. A contributor
following it looks for files that are not there.

`technical-reference/workspace.md` calls `apimock` the *"workspace root"*
crate. It moved to `crates/apimock/` in 5.1.1.

### 2. The contributor persona has no documentation at all

Surveyed 2026-08-03. `docs/` contains **no** local development
procedure, no build-from-source instructions, no guide to running the
tests, and no `cargo install` path — despite a crates.io badge on the
README.

The only contributor-facing technical content anywhere is the six gate
commands RFCs 031 and 033 added to `.github/CONTRIBUTING.md`, and
nothing in `docs/` links to it.

The project's own guidelines name maintainers/contributors as one of
three documentation personas. It is the one served by nothing.

### 3. Nothing tells a reader what a configuration will *do*

The owner's directive for M2 was that a reader should be able to
**predict the effect of a configuration change before making it**.

There is no page stating the order in which mechanisms are consulted,
and no statement of what wins when several could match.
`response-decision-flow.md` is the closest and sits under the abolished
"Advanced Topics".

RFC 034 § D4 assigns this to a new page, **How it works → Matching order
and precedence**, and calls it the single most important new page in the
plan. This RFC owns writing it.

Note that RFC 037's implementation established the real order by reading
`crates/apimock-server/src/server.rs:322-352` — and found the handoff's
own suggested phrasing had it backwards and omitted middleware:

```
middleware_response  → returns early if a script answers
rule_set_response    → rule sets in declaration order
dyn_route_content    → file tree, only if nothing matched
```

That is a warning about this page specifically: it is easy to state
confidently and wrongly.

### 4. Document-integrity defects

Accumulated across this milestone's surveys, none yet fixed:

| Defect | Location |
|---|---|
| Duplicate `## [5.4.0]` entries, different text, both present | `CHANGELOG.md` lines 417 and 714 |
| Dead link to `docs/CONFIGURE.md`, which does not exist | `technical-reference/vision-and-goals.md` |
| Dead link to `./getting-started/rule-based-routing.md` (files are `-1.md`, `-2.md`) | `user-guide/faq.md` — dies with the page under RFC 034 D5, but confirm |

## Scope

Per RFC 034 § D5.

**How it works** — explanation:

| Page | Source |
|---|---|
| **Matching order and precedence** | **new** — § Motivation 3 |
| Response decision flow | `advanced-topics/response-decision-flow.md` |
| Architecture | `technical-reference/architecture.md` — **rewrite** |
| The workspace and its crates | `technical-reference/workspace.md` — **rewrite** |
| Design notes | `design/server/README.md` + `design/response/README.md` — why dotted paths not JSONPath, why read-on-demand |
| Performance | `technical-reference/benchmarks.md` |

**Contributing** — new section:

| Page | Source |
|---|---|
| **Build and test locally** | **new** |
| **The quality gates** | **new** — links `.github/CONTRIBUTING.md`, does not duplicate it |
| **The RFC process** | **new** — links `rfcs/`, does not duplicate RFC 000 |
| Vision and goals | `technical-reference/vision-and-goals.md` (fix the dead link) |

**Document integrity** — the three defects in § Motivation 4.

## Non-goals

- Getting started, Guides, Reference — **RFC 035**.
- `README.md` — RFC 037.
- Re-deciding structure — RFC 034's map is settled.
- Duplicating `CONTRIBUTING.md` or RFC 000. The Contributing section
  **links** them; two copies of a procedure drift.
- Fixing product defects found while writing. Report them.

## Reference-level explanation

### Matching order and precedence

The page RFC 034 D4 specifies. It owns:

- the order mechanisms are consulted — middleware, then rule sets in
  declaration order, then the fallback file tree;
- what wins when more than one could match;
- what `prefix`, `guard`, and per-rule-set `strategy` do to that order.

**Verify against `server.rs` before writing a word.** § Motivation 3
explains why: a confident wrong statement here is the most damaging
single error the documentation could contain, because this is the page
readers will use to predict behaviour.

Two honesty requirements:

- **`guard` currently does nothing.** It is a zero-field struct carrying
  a `// todo:` comment (RFC 036 Escalation 004). Do not describe it as
  affecting matching. Its disposition is an open owner decision — until
  then, say what is true.
- **`[default].delay_response_milliseconds` is inert** (RFC 036
  Escalation 002, now RFC 045). Do not document it as working.

### Architecture and workspace pages

Rewrites, not edits. The current text describes a different program.

The four-crate structure, the one-way dependency direction
(`server → config → routing`), and the façade's purpose are all recorded
in the v5.14.0 handoff bundle's `external-design.md` — a useful starting
point, but **verify against the manifests**, since that bundle is itself
a year-old snapshot in places.

### Contributing

The persona needs: how to build, how to run the tests, what the six gates
are and how to run them locally, and how the RFC process works.

`.github/CONTRIBUTING.md` already carries the gate commands. **Link
it.** The docs page explains *what the gates are for* and *when they
run*; CONTRIBUTING stays the copy-pasteable list. One source per fact.

### Landing complete sections

`docs.yaml` deploys on every push to `main`. Land a complete section at a
time; the site must be coherent at every commit. Coordinate `SUMMARY.md`
with RFC 035 — whoever lands second rebases.

## Testing and verification

1. `mdbook build` succeeds; every `SUMMARY.md` entry resolves.
2. Every relative link resolves; the two dead links in § Motivation 4
   are gone.
3. `CHANGELOG.md` has exactly one `## [5.4.0]` entry, and the retained
   text is the accurate one.
4. Every path named in the architecture page exists.
5. The matching-order page's claims trace to `server.rs`, cited by line.
6. No page describes `guard` or `[default].delay_response_milliseconds`
   as functional.
7. The site is coherent at every commit.

## Drawbacks

1. **The matching-order page is easy to get wrong**, and wrong here is
   worse than absent — a reader would use it to predict behaviour and be
   confidently misled. Hence the cite-by-line requirement.
2. **A Contributing section invites contributions** the project may not
   want at volume. `CONTRIBUTING.md` is already explicit that pull
   requests are reviewed but not guaranteed; the docs section should
   match that tone rather than over-invite.
3. **Deleting one of two `[5.4.0]` entries loses text.** Both describe
   the same release differently; keeping the accurate one and dropping
   the other is right, but it is a deletion from a historical record and
   should be called out in the review request.

## Rationale and alternatives

**Alternative A (chosen): rewrite How it works, add Contributing, fix
integrity defects.**

**Alternative B: fix the two stale pages, skip Contributing.** Cheaper.
Rejected — the contributor persona would remain unserved, which was the
gap RFC 034 § D1 called out as the one the current tree fails
completely.

**Alternative C: put contributor docs only in `CONTRIBUTING.md`.**
Defensible, and it is where the gate commands already live. Rejected
because build/test/RFC-process guidance is documentation, not a
contribution policy, and `CONTRIBUTING.md` is not discoverable from the
docs site. The split above — policy in `CONTRIBUTING.md`, explanation in
the docs, links between — keeps one source per fact.

## Unresolved questions

1. **Does the matching-order page include a diagram?** The docs already
   have mermaid configured (`docs/book.toml`). A flow diagram would suit
   this page; it is also another thing to keep true. Implementer's call.
2. **Which `[5.4.0]` entry is the accurate one?** Both describe a
   refactor-only release; they differ in detail and date (2026-04-27 vs
   2026-04-28). Determine from git history rather than picking.
3. **Does `benchmarks.md` survive as-is?** It documents two benchmark
   suites. RFC 037 removed an unverifiable k6 claim from the README; if
   `benchmarks.md` makes claims that cannot be reproduced, the same
   standard applies here.
