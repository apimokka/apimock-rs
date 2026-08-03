# RFC 034 — Documentation information architecture

**Status.** Proposed
**Tracks.** M2 (Documentation and examples). The docs have accreted
page-by-page across 14 minor releases without anyone deciding what the
whole is supposed to be. Before rewriting the prose, decide the shape.
**Touches.** `docs/src/SUMMARY.md` and the layout of `docs/src/`. This
RFC produces **decisions and a skeleton, not finished prose** — the prose
is RFC 035, 037, and 038.

## Summary

Decide the documentation's information architecture: who it is for, what
question each section answers, what belongs where, and what the
navigation looks like. Produce a page map and a migration plan from the
current tree. Write no user-facing prose.

Gates RFCs 035, 037, and 038, all of which write into the structure this
RFC settles.

## Motivation

The immediate trigger is a quality directive from the project owner:
documentation should be *easy to read*, should *help a reader understand
the tool*, and should let them **predict the effect of a configuration
change before making it**. That last property is the demanding one, and
nothing in the current docs is organised around it.

The current tree is 1,760 lines across 30 pages in four top-level
sections (User Guide, Advanced Topics, Technical Reference, plus a Home
page). It has grown by accretion, and the symptoms are structural rather
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
5. Produce a page map and a migration plan from the current 30 pages.

## Non-goals

- Writing user-facing prose. That is RFC 035 (user guide and reference),
  RFC 037 (README), RFC 038 (technical reference).
- Correcting factual errors. RFC 035 owns those; this RFC may *record*
  them against the page map but must not fix them.
- Changing mdbook itself, its theme, or the docs CI workflow.
- Examples under `crates/apimock/examples/` — RFC 036.

## Required output

### 1. Persona definitions

The project's own guidelines already name three documentation personas —
new users, intermediate users, and maintainers/contributors. This RFC
must state, for each, the questions they arrive with and the order they
arrive in. Personas that do not change a structural decision should be
dropped rather than documented for completeness.

### 2. Section model

A replacement for the current four-section split, with a one-sentence
charter per section stating **the question it answers**. Any section
whose charter is "things that didn't fit" fails this criterion —
"Advanced Topics" is the current example.

### 3. Page map

Every subject currently documented, plus every subject that ships but is
undocumented, mapped to exactly one destination page. The undocumented
set is known and must appear: `apimock validate`, `apimock match-test`,
the trace channel, TLS hot-reload, `[file_tree_view]` filtering, rule
`priority` / `weight`, the four non-default strategies, and roughly 44 of
the 49 operator variants.

The map states, per page: destination path, charter, source material (an
existing page, or new), and the persona it serves.

### 4. Predictability requirement

The structure must include a home for material that lets a reader
anticipate behaviour before running the server. At minimum, somewhere in
the map there must be a page that owns:

- the order in which matching mechanisms are consulted (rule sets in
  declaration order → strategy within a set → fallback file tree), and
- what wins when more than one could match.

This RFC decides *where* that lives, not what it says.

### 5. Migration plan

For each of the current 30 pages: keep in place, move, merge, split, or
delete. Deletions need a stated reason. mdbook renders relative links,
so the plan must include a link-rewrite pass — the same discipline
[RFC 000](../done/000-rfc-lifecycle-policy.md) applies to RFC
cross-references.

### 6. Skeleton

A rewritten `docs/src/SUMMARY.md` plus placeholder pages for the agreed
map, each carrying its charter as a comment. This is the artifact RFC
035, 037, and 038 write into.

## Required tests

Structural, not behavioural:

- `mdbook build` succeeds against the skeleton.
- Every entry in `SUMMARY.md` resolves to a file that exists.
- Every relative link in the skeleton resolves.
- No page from the current tree is silently dropped — each appears in
  the migration plan with a disposition.

## Acceptance criteria

1. Personas defined, each tied to a structural consequence.
2. A section model where every section's charter is a question, not a
   residual category.
3. A page map covering every currently-documented subject **and** every
   shipped-but-undocumented subject listed in § 3.
4. Every subject has exactly one authoritative home; the rule for
   placing new subjects is written down.
5. A home exists for matching order and precedence (§ 4).
6. A migration disposition for all 30 current pages, deletions justified.
7. `SUMMARY.md` skeleton builds under `mdbook build` with all links
   resolving.
8. No user-facing prose written; no factual correction made.

## Drawbacks

1. **It produces no visible improvement.** A reader sees nothing until
   RFC 035 lands. The alternative — rewriting prose into a structure
   nobody agreed on — is how the current tree happened.
2. **It creates a bottleneck.** Three RFCs wait on this one. Mitigated by
   RFC 036 running in parallel, and by this RFC's output being decisions
   and a skeleton rather than a large body of work.
3. **Restructuring breaks external links.** Anyone who has bookmarked or
   linked to a page under the published docs site will get a 404. Worth
   accepting for a docs tree this size, but it should be a conscious
   choice — see Unresolved questions.

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

1. **Should redirects be provided for moved pages?** mdbook supports a
   redirect table. Cheap insurance if the published site has inbound
   links; unnecessary if it does not. Needs a look at whether anything
   links to specific pages.
2. **Does the rule-set TOML schema get a reference page, a guide page,
   or both?** It is the single most-consulted subject and the one
   currently split three ways. The page map must resolve it explicitly
   rather than leaving it to RFC 035.
3. **Is Diátaxis (or similar) adopted by name?** See Alternative D.
