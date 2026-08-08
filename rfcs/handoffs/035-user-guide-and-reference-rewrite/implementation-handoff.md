# Implementation Handoff — RFC 035, User guide and reference rewrite

**Governing RFC.** [RFC 035](../../done/035-user-guide-and-reference-rewrite.md)
**Structure decided by.** [RFC 034 § D5](../../done/034-documentation-information-architecture.md)
**Milestone.** M2 — **blocks v5.16.0**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

**Runs in parallel with RFC 038.** Different sections; one shared file
(`SUMMARY.md`) — see § 7.

---

## 1. Purpose

Write Getting started, Guides, and Reference: the documentation a reader
uses to get running and to look things up. This is the RFC that makes
the documentation *true*.

## 2. Why this blocks the release

v5.16.0 waits on this. `README.md` ships **frozen** in the published
crate and points at the docs root; publishing it while the docs still
state that four shipped strategies do not exist would defeat RFC 037's
purpose. See `ROADMAP.md` § M2.

## 3. Scope

The three sections in RFC 034 § D5: Getting started (3 pages), Guides
(13), Reference (6). Their `SUMMARY.md` entries.

## 4. Explicit non-change scope

Do **not**:

- Touch How it works, Contributing, `architecture.md`, `workspace.md`,
  `benchmarks.md`, `vision-and-goals.md` — **RFC 038**.
- Touch `README.md` — RFC 037.
- Touch `crates/apimock/examples/` — RFC 036, implemented.
- Change crate source. **If you find a defect, report it** — RFC 036
  found four that way and fixed none.
- Re-decide structure. RFC 034's map is settled. A subject that will not
  fit is a **design request against RFC 034**, not a licence to invent a
  section.

## 5. The central constraint — verify against code

The current documentation is wrong *because someone wrote it from a
feature's name instead of its implementation*. Do not repeat that.

**Do not describe a feature from its RFC, its title, or a prior doc
page.** Read the implementation. Where practical, run it.

**Generate the operator tables; do not transcribe them.** There are 49
variants across `RuleOp` (11), `HeaderOperator` (13), and `BodyOperator`
(25). A hand-copied table of 49 rows is wrong on the day it ships. Derive
from the enums by whatever means you like — a script, a test, a macro —
but the acceptance bar is that **every variant in the source appears in
the reference**, checked mechanically.

RFC 036's example sets are runnable and verified. They are a legitimate
source for guide content — reuse rather than re-derive.

## 6. Land complete sections

`.github/workflows/docs.yaml` deploys the site on **every push to
`main`**. There is no staging.

So each merge must leave the site coherent. Land a **complete section**
at a time. A page being replaced stays until its replacement is ready,
then both move in the same commit. Never merge a section half-migrated.

## 7. `SUMMARY.md` — the one file shared with RFC 038

You add three sections' entries; RFC 038 adds two. You are working in
parallel on the same file.

Coordinate. Whoever lands second rebases. Do not restructure entries you
do not own.

## 8. Known-wrong content — the minimum

These are confirmed and must be corrected. Anything else you find gets
corrected too, and **listed in the review request** — a silent
correction is indistinguishable from a silent error.

| Wrong | Where |
|---|---|
| `strategy = "first_match"  # only value supported today` | `user-guide/configuration-reference.md:23` |
| "Currently the only strategy" | same file, line 37 |
| 5 operators documented, 49 exist | same file |

Undocumented entirely — every one returns zero matches across
`docs/src`: `apimock match-test`, `structural_contains`, `map_has_key`,
`round_robin`, `weighted_random`, `uniform_random`, `respect_gitignore`,
`extra_excludes`, file-tree filtering, TLS hot-reload, and the negated
operators.

## 9. Required tests

1. `mdbook build` succeeds; every `SUMMARY.md` entry resolves.
2. Every relative link resolves.
3. **Mechanical check: every variant of all three operator enums appears
   in the reference.** Not by eye.
4. Every subject in § 8's undocumented list has a home.
5. No page states a feature does not exist when it does.
6. The site is coherent at every commit — verifiable by building at each.

## 10. Prohibited shortcuts

- Writing a feature's description from its RFC rather than its code.
- Transcribing the operator tables by hand.
- Merging a half-migrated section.
- Fixing a product defect you find. Report it.
- Inventing a section because a subject will not fit RFC 034's map.

## 11. Escalation triggers

- **A subject has no home** in RFC 034's map → design request against
  RFC 034.
- **A feature's behaviour differs from every existing description** →
  report before documenting; it may be a defect rather than a doc gap.
- **A product defect** — RFC 036's precedent applies exactly.

## 12. Required evidence

- `mdbook build` output.
- Link-check output over the built site.
- The mechanical operator check, with its method and output.
- A list of every correction made beyond § 8's minimum.
- Confirmation the site built coherently at each commit.

## 13. Required review-request format

Package at `.git-exclude/review-request/035-user-guide-and-reference-rewrite/`
with an entry-point file a reviewer can open cold. Per § 9.2 of the
workflow document. **Hand back one path — the entry-point file itself.**

Reviewer's focus: whether any documented behaviour was written from a
name rather than from code, and the operator check's method.
