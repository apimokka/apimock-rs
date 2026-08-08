# RFC 035 — User guide and reference rewrite

**Status.** Proposed
**Tracks.** M2. Executes RFC 034's page map for the reader-facing half of
the documentation: Getting started, Guides, and Reference.
**Touches.** `docs/src/` — the Getting started, Guides, and Reference
sections, plus the `SUMMARY.md` entries for them. Not the How it works or
Contributing sections (RFC 038), not `README.md` (RFC 037).

## Summary

Write the documentation a reader uses to get running and to look things
up: three Getting started pages, thirteen Guides, six Reference pages —
per [RFC 034](./034-documentation-information-architecture.md) § D5.

This is the RFC that makes the documentation *true*. The correctness
catch-up that M2 originally consisted of is absorbed here.

## Motivation

`docs/src` documents roughly the v5.8.0 feature set. Two years of shipped
work is invisible, and some of what is written is actively wrong.

### What is wrong, not merely missing

`user-guide/configuration-reference.md:23,37` states:

```toml
strategy = "first_match"    # only value supported today
```

> `service.strategy` | Currently the only strategy — the first rule that
> matches wins.

Five strategies ship (`first_match`, `uniform_random`, `weighted_random`,
`priority`, `round_robin`), plus RFC 025's per-rule-set override. The
same page documents **5 operators against 49 in code**.

A reader consulting the reference concludes that shipped features do not
exist. That is worse than an omission — they will not go looking.

### What is missing entirely

Re-surveyed 2026-08-04. Zero matches across `docs/src`:

| Subject | Shipped in |
|---|---|
| `apimock match-test` | RFC 015 |
| `structural_contains`, `map_has_key` | RFCs 028, 022 |
| `round_robin`, `weighted_random`, `uniform_random` | RFCs 007, 011 |
| `respect_gitignore`, `extra_excludes`, file-tree filtering | RFCs 005, 012, 019 |
| TLS hot-reload | RFC 020 |
| negated operators (`not_starts_with`, `ends_with`, …) | RFC 021 |

`apimock validate` and rule `priority` / `weight` appear incidentally but
are not documented as features.

### Why now

v5.16.0 waits on this RFC. `README.md` ships frozen in the published
crate and points at the docs root; publishing it while the docs still
contradict the product would defeat RFC 037's purpose (`ROADMAP.md` § M2).

## Scope

Per RFC 034 § D5, this RFC owns three sections.

**Getting started** — linear, read in order, no choices:

| Page | Source |
|---|---|
| Install and first response | `user-guide/getting-started/README.md` + `file-based-routing.md` |
| Your first config file | `root-configuration.md` + `toml-configuration.md` (merge) |
| Your first rule | `rule-based-routing-1.md` + `-2.md` (merge) |

**Guides** — task-indexed, each standalone: the thirteen pages in
RFC 034 § D5, of which seven are new — varying a response by strategy,
TLS hot-reload, file-tree filtering, validating in CI, dry-running a
rule, watching matches live, and serving JSON from a folder as its own
task.

**Reference** — exhaustive, lookup:

| Page | Note |
|---|---|
| `apimock.toml` root settings | split from `configuration-reference.md` |
| Rule-set schema | merged from `configuration-reference.md` + `rule-set-config-structure/*` (4 pages) — RFC 034 D7 makes this the single authoritative home |
| **Operator reference** | all 11 `RuleOp`, 13 `HeaderOperator`, 25 `BodyOperator` |
| Body path syntax | dotted mini-syntax, **explicitly not JSONPath** |
| **CLI reference** | flags, `--init`, `validate`, `match-test` |
| Response headers | merged from `design/response/headers/*` (3 pages) |

## Non-goals

- How it works, Contributing, `architecture.md`, `workspace.md`,
  `benchmarks.md`, `vision-and-goals.md` — **RFC 038**.
- `README.md` — RFC 037.
- Examples — RFC 036, implemented.
- Re-deciding structure. RFC 034's map is settled; a subject that will
  not fit is a design request against RFC 034, not a licence to invent a
  section.
- Fixing product defects found while writing. Report them — RFC 036
  found four that way.

## Reference-level explanation

### Every documented behaviour must be verified against code

This is the RFC's central constraint and the reason the current
documentation is wrong.

Do not describe a feature from its RFC, its name, or a prior doc page.
Read the implementation, and where practical run it. The operator tables
in particular must be generated from the enums — `RuleOp`,
`HeaderOperator`, `BodyOperator` — not transcribed, because a
hand-transcribed table of 49 variants will be wrong on the day it ships.

RFC 036's example sets are runnable and verified; they are a legitimate
source for guide content and worth reusing rather than re-deriving.

### Landing complete sections

`docs.yaml` deploys on every push to `main`. Each merge must leave the
site coherent — so land a **complete section** at a time, not a
half-migrated tree. Pages being replaced stay until their replacement is
ready, then go in the same commit.

### `SUMMARY.md`

This RFC adds its three sections' entries. RFC 038 adds the other two.
Coordinate so neither clobbers the other; whoever lands second rebases.

### Known-wrong content

The corrections in § Motivation are the minimum. Anything else found
while reading code gets corrected too, and **listed in the review
request** — a silent correction is indistinguishable from a silent
error.

## Testing and verification

1. `mdbook build` succeeds; every `SUMMARY.md` entry resolves.
2. Every relative link resolves.
3. **Every operator in the three enums appears in the operator
   reference** — checked mechanically against the source, not by eye.
4. Every subject in § Motivation's missing table has a home.
5. No page states a feature does not exist when it does.
6. The site is coherent at every commit.

## Drawbacks

1. **It is a large body of writing** — roughly 22 pages, half of them
   new. Mitigated by RFC 034 having settled the structure and RFC 036
   having produced verified examples to draw on.
2. **Verifying 49 operators against code is tedious**, and the tedium is
   exactly where errors enter. Hence the requirement to generate rather
   than transcribe.
3. **It blocks v5.16.0.** Deliberate — see § Motivation.

## Rationale and alternatives

**Alternative A (chosen): write the three sections completely, against
code.**

**Alternative B: correct the known-wrong statements only, defer the
rest.** This was M2's original scope, revised by the owner on
2026-08-02. It would leave two years of features undocumented while
implying the docs are now trustworthy — the worst of both.

**Alternative C: generate the reference from code.** Attractive for the
operator tables specifically, and worth doing if it is cheap. Not
mandated: a generator is a maintenance surface of its own, and the
requirement here is that the tables *match* the enums, not how that is
achieved.

## Unresolved questions

1. **Is the operator reference one page or three** (url_path / header /
   body)? 49 variants on one page may be a wall; three pages fragment a
   lookup. Implementer's call, stated in the review request.
2. **Do the Guides link the RFC 036 example sets directly?** They are
   verified and runnable, so a guide could point at one rather than
   restate it. Attractive, but couples docs to example paths — decide
   explicitly.
3. **How much of `faq.md` survives?** RFC 034 D5 deletes the page and
   redistributes its questions. Some may have no home in this RFC's
   sections and belong to RFC 038; hand those over rather than dropping
   them.
