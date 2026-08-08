# RFC 034 — Documentation information architecture

**Status.** Proposed — **design decided 2026-08-04**, see § Decisions.
**Tracks.** M2 (Documentation and examples). The docs have accreted
page-by-page across 14 minor releases without anyone deciding what the
whole is supposed to be. Before rewriting the prose, decide the shape.
**Touches.** Nothing on disk. This RFC is **decisions only** — no files
under `docs/src` are created or moved by it. RFCs 035, 037, and 038 land
the actual pages.

## Summary

Decide the documentation's information architecture: who it is for, what
question each section answers, what belongs where, and what the
navigation looks like. The decisions are in § Decisions below; RFCs 035,
037, and 038 execute them.

## Amendment 2026-08-04 — no skeleton, and why

The original version required a `SUMMARY.md` skeleton with placeholder
pages as output. **That requirement is withdrawn**, because it cannot be
landed safely:

`.github/workflows/docs.yaml` triggers on **push to `main`** and deploys
to GitHub Pages. It is not gated by the release cycle. So any commit
touching `docs/src` publishes immediately — a skeleton of placeholder
pages would go live as a half-built public documentation site, which is
worse for readers than the current stale-but-populated one.

Consequences:

- This RFC produces **decisions only**. Nothing merges to `docs/src`.
- RFCs 035, 037, 038 each land **complete sections**, so `main` is
  coherent at every commit.
- Those three no longer gate each other once the map below exists — they
  can run in parallel.

This also changes what "M2" means for the release cycle; see
`ROADMAP.md` § M2.

## Motivation

The immediate trigger is a quality directive from the project owner:
documentation should be *easy to read*, should *help a reader understand
the tool*, and should let them **predict the effect of a configuration
change before making it**. That last property is the demanding one, and
nothing in the current docs is organised around it.

The current tree is 38 content pages (~1,240 non-blank lines) in four
top-level sections — User Guide, Advanced Topics, Technical Reference,
plus a Home page. Five of the 38 are near-empty stubs of 2–5 lines. It has grown by accretion, and the symptoms are structural rather
than merely stale:

- **The same subject is split across sections with no stated rule.**
  Rule-set configuration appears in `user-guide/getting-started/`
  (`rule-based-routing-1.md`, `rule-based-routing-2.md`), in
  `user-guide/configuration-reference.md`, *and* in
  `advanced-topics/rule-set-config-structure/` (4 pages). A reader
  looking up `when.request.headers` has three plausible destinations and
  no way to know which is authoritative.
- **"Advanced Topics" is a residual category**, not a persona or a task.
  It currently holds middleware, the listener, TLS, the response-decision
  flow, and the rule-set structure — which is most of the product's
  surface, not an advanced subset.
- **The reference page is a wall.** `configuration-reference.md` is 150
  lines covering the root file, the rule-set schema, operators, glob
  syntax, body paths, prefix, and defaults, in one scroll.
- **Nothing answers "what will this do?"** There is no page that traces
  a request through the system, no worked before/after, and no statement
  of precedence when several mechanisms could match. The
  `response-decision-flow.md` page is the closest and is filed under
  Advanced Topics.

Fixing the wrong facts (RFC 035's job) without fixing the structure would
produce accurate documentation that is still hard to navigate — and would
have to be reorganised later, rewriting the same pages twice.

## Goals

1. Name the reader personas and what each needs.
2. Assign every subject exactly one authoritative home, with a stated
   rule for where a new subject goes.
3. Design navigation such that a reader with a question reaches the
   answer without guessing between sections.
4. Ensure the structure has a natural place for "what effect will this
   configuration have?" — the owner's stated requirement.
5. Produce a page map and a migration plan from the current 38 pages.

## Non-goals

- Writing user-facing prose. That is RFC 035 (user guide and reference),
  RFC 037 (README), RFC 038 (technical reference).
- Correcting factual errors. RFC 035 owns those; this RFC may *record*
  them against the page map but must not fix them.
- Changing mdbook itself, its theme, or the docs CI workflow.
- Examples under `crates/apimock/examples/` — RFC 036.

## Decisions

Decided 2026-08-04 by the architect. RFCs 035, 037, and 038 execute
these; they are not open for re-derivation during implementation.

### D1 — Personas

Three, each earning its place by changing a structural decision.

| Persona | Arrives asking | Structural consequence |
|---|---|---|
| **First-time user** — frontend/QA dev, first hour | "How do I get a mock API running?" | Needs one linear path, read in order, no choices |
| **Working user** — has a config, returns weekly | "How do I make it do X?" | Needs task-indexed lookup; every page standalone |
| **Contributor** — wants to build or change apimock | "How do I build, test, and change this?" | Needs a section that **does not currently exist** |

The contributor persona is the one the current tree fails completely
(see § 1 of Required output, retained below).

### D2 — Section model, adopting Diátaxis

**Diátaxis is adopted by name**, extended with a contributor section.
Unresolved question 3 is resolved: naming the framework gives future
contributors a rule for placing a new page, which is what the current
tree lacks.

| Section | Charter — the question it answers | Diátaxis |
|---|---|---|
| **Getting started** | How do I get a mock API running? | tutorial |
| **Guides** | How do I make it do X? | how-to |
| **Reference** | What exactly does this setting do? | reference |
| **How it works** | What will this configuration do, and why? | explanation |
| **Contributing** | How do I build, test, and change apimock? | — |

"Advanced Topics" is **abolished**. Its charter was "things that did not
fit", which fails the test that every section's charter be a question.
Its contents redistribute per D5.

### D3 — Placement rule for new subjects

Written down so the next accretion has an answer:

1. Does it teach the first path a new user walks? → **Getting started**
2. Is it a task a user wants to accomplish? → **Guides**
3. Is it "what does this key/flag do"? → **Reference**
4. Is it "why does it behave this way"? → **How it works**
5. Is it about changing apimock itself? → **Contributing**

A subject that seems to fit two sections goes in **Reference** with a
link from the other. A subject that fits none is a signal the model is
wrong — raise it rather than inventing a sixth section.

### D4 — The predictability requirement has a home

The owner's requirement — a reader should be able to **predict the
effect of a configuration change before making it** — lives in
**How it works → Matching order and precedence**, a new page that owns:

- the order mechanisms are consulted: rule sets in declaration order →
  strategy within a set → fallback file tree;
- what wins when more than one could match;
- what `prefix`, `guard`, and per-rule-set `strategy` do to that order.

This page did not exist in any form. It is the single most important new
page in the plan and RFC 038 owns writing it.

### D5 — Page map and migration

Every current page has a disposition. **38 content pages** (excluding
`SUMMARY.md`).

**Getting started** — linear, read in order:

| Page | From |
|---|---|
| Install and first response | `user-guide/getting-started/README.md` + `file-based-routing.md` |
| Your first config file | `root-configuration.md` + `toml-configuration.md` (merge) |
| Your first rule | `rule-based-routing-1.md` + `-2.md` (merge) |

**Guides** — task-indexed, each standalone:

| Page | From |
|---|---|
| Serve JSON files from a folder | `file-based-routing.md` (split) |
| Match on URL path and method | `examples/operators.md`, `combining-conditions-1.md` |
| Match on headers | `combining-conditions-2.md` |
| Match on the request body | `rule-set-config-structure/rules/when.md` |
| Return errors and status codes | `design/response/README.md` (extract) |
| **Vary the response for one path** | **new** — strategies, `priority`, `weight` |
| Simulate slow or flaky backends | `rules/respond.md` (extract) |
| Serve over HTTPS | `advanced-topics/listener/https-support.md` |
| **Reload TLS certificates without restart** | **new** |
| Script with Rhai middleware | `middleware-basics-*.md` + `middleware-map-*.md` (merge) |
| **Filter the served file tree** | **new** — `respect_gitignore`, `extra_excludes` |
| **Validate config in CI** | **new** — `apimock validate` |
| **Dry-run a rule** | **new** — `apimock match-test` |
| **Watch matches live** | **new** — trace channel, `capture_body` |

**Reference** — exhaustive, lookup:

| Page | From |
|---|---|
| `apimock.toml` root settings | `configuration-reference.md` (split 1) |
| Rule-set schema | `configuration-reference.md` (split 2) + `rule-set-config-structure/*` (4 pages, merge) |
| **Operator reference** | `examples/operators.md` + **new** — all 11 `RuleOp`, 13 `HeaderOperator`, 25 `BodyOperator` |
| Body path syntax | `configuration-reference.md` (split 3) |
| **CLI reference** | **new** — flags, `--init`, `validate`, `match-test` |
| Response headers (CORS, OPTIONS) | `design/response/headers/*` (3 pages, merge) |

**How it works** — explanation:

| Page | From |
|---|---|
| **Matching order and precedence** | **new** — D4 |
| Response decision flow | `advanced-topics/response-decision-flow.md` |
| Architecture | `technical-reference/architecture.md` — **rewrite**, currently describes the pre-5.0.0 single-crate layout |
| The workspace and its crates | `technical-reference/workspace.md` — **rewrite**, calls `apimock` the workspace-root crate |
| Design notes | `design/server/README.md` + `design/response/README.md` — why dotted paths not JSONPath, why read-on-demand |
| Performance | `technical-reference/benchmarks.md` |

**Contributing** — new section:

| Page | From |
|---|---|
| **Build and test locally** | **new** |
| **The quality gates** | **new** — links `.github/CONTRIBUTING.md`, does not duplicate it |
| **The RFC process** | **new** — links `rfcs/`, does not duplicate RFC 000 |
| Vision and goals | `technical-reference/vision-and-goals.md` (fix the dead `docs/CONFIGURE.md` link) |

**Deleted** — with reasons:

| Page | Reason |
|---|---|
| `advanced-topics/README.md` (2 lines) | Section abolished; stub |
| `advanced-topics/rule-set-config-structure/README.md` (4 lines) | Stub; contents merged into Reference |
| `technical-reference/design/README.md` (2 lines) | Stub |
| `user-guide/examples/README.md` (3 lines) | Stub; Guides is self-indexing |
| `user-guide/conclusion.md` (5 lines) | A closing page serves no persona |
| `user-guide/faq.md` | Contents redistribute to the page that owns each question; an FAQ is where content goes when nobody decided where it belongs — the exact pattern this RFC exists to end. **Its broken `rule-based-routing.md` link dies with it.** |
| `user-guide/README.md`, `technical-reference/README.md` | Replaced by new section indexes |

Net: 38 pages → roughly 33, with **11 genuinely new** and the
undocumented feature set finally placed.

### D6 — Redirects

**Not provided.** Unresolved question 1 resolved. mdbook supports a
redirect table, but the current site's pages are linked from the README
and little else; the cost of maintaining a redirect map for a tree this
size exceeds the benefit. RFC 037 updates the README's links in the same
milestone.

### D7 — The rule-set schema gets a Reference page, not a guide

Unresolved question 2 resolved. It is the most-consulted subject and is
currently split three ways (`getting-started`, `configuration-reference`,
`advanced-topics/rule-set-config-structure/`). One authoritative
Reference page; Guides link to it rather than restating it.

---

## Required output, retained for context

The following was the original brief. It is superseded by § Decisions
above, and retained because its § 1 and § 3 carry survey evidence the
executing RFCs still need.

## Verification

This RFC lands no files, so there is nothing to build. It is verified by
inspection against its own acceptance criteria, and by the executing
RFCs finding the map sufficient — if 035, 037, or 038 cannot place a
subject using D3, that is a defect in this RFC to be raised, not worked
around.

## Acceptance criteria

1. ✅ Personas defined, each tied to a structural consequence — D1.
2. ✅ Section model where every charter is a question — D2.
3. ✅ Page map covering every documented subject and every
   shipped-but-undocumented subject — D5.
4. ✅ Every subject has one authoritative home; placement rule written
   down — D3, D7.
5. ✅ A home exists for matching order and precedence — D4.
6. ✅ A migration disposition for all 38 current pages, deletions
   justified — D5.
7. ✅ No user-facing prose written; no factual correction made.
8. ✅ No files created or moved under `docs/src`.

## Drawbacks

1. **It produces no visible improvement.** A reader sees nothing until
   RFC 035 or 038 lands. The alternative — rewriting prose into a
   structure nobody agreed on — is how the current tree happened.
2. **Deleting the FAQ is contentious.** An FAQ is popular with readers
   and cheap to append to. It is deleted here precisely because "cheap
   to append to" is what makes it the destination for content nobody
   placed — the pattern this RFC exists to end. Each question moves to
   the page that owns its subject. If that proves wrong in practice,
   reinstating it is a small change; the map is not load-bearing on its
   absence.
3. **Restructuring breaks external links**, and D6 declines to provide
   redirects. Accepted for a tree this size, with RFC 037 updating the
   README's links in the same milestone.

## Rationale and alternatives

**Alternative A (chosen): design the structure first, then write.**

**Alternative B: rewrite page-by-page, letting structure emerge.**
Rejected — that is the process that produced the current tree, and it
has no mechanism for noticing that "Advanced Topics" has become a
dumping ground.

**Alternative C: fix the facts only (the original M2 scope), restructure
later.** Rejected by the owner on 2026-08-02. It also means writing the
same pages twice, since a later restructure would move and re-cut them.

**Alternative D: adopt an off-the-shelf framework such as Diátaxis**
(tutorial / how-to / reference / explanation). Genuinely worth
considering — it maps well onto the personas the project already names,
and it would have caught the "Advanced Topics" problem by construction.
Not mandated here because the choice of framework is exactly what this
RFC exists to decide; it should be evaluated as a candidate, not assumed.

## Unresolved questions

All three original questions are resolved by § Decisions:

1. ~~Should redirects be provided for moved pages?~~ → **D6: no.**
2. ~~Does the rule-set schema get a reference page, a guide page, or
   both?~~ → **D7: one Reference page**, linked from Guides.
3. ~~Is Diátaxis adopted by name?~~ → **D2: yes**, extended with a
   Contributing section.

None remain. The executing RFCs should raise a design request rather
than improvise if the map proves insufficient.
