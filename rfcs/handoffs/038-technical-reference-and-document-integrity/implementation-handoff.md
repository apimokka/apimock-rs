# Implementation Handoff — RFC 038, Technical reference and document integrity

**Governing RFC.** [RFC 038](../../done/038-technical-reference-and-document-integrity.md)
**Structure decided by.** [RFC 034 § D5](../../done/034-documentation-information-architecture.md)
**Milestone.** M2 — **blocks v5.16.0**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

**Runs in parallel with RFC 035.** Different sections; one shared file
(`SUMMARY.md`) — see § 8.

---

## 1. Purpose

Three things: **How it works** (including the page that does not exist in
any form — matching order and precedence), **Contributing** (a section
serving a persona the docs currently fail completely), and the
**document-integrity defects** accumulated across this milestone.

## 2. Scope

Per RFC 034 § D5: How it works (6 pages), Contributing (4), plus three
integrity fixes including one in `CHANGELOG.md`.

## 3. Explicit non-change scope

Do **not**:

- Touch Getting started, Guides, or Reference — **RFC 035**.
- Touch `README.md` — RFC 037.
- Change crate source. **Report defects, do not fix them.**
- Duplicate `.github/CONTRIBUTING.md` or RFC 000 — § 6.
- Re-decide structure. A subject that will not fit is a design request
  against RFC 034.

## 4. The matching-order page — establish it, do not accept it

RFC 034 § D4 calls this the single most important new page in the plan.
It is also the easiest to get confidently wrong.

**Read `crates/apimock-server/src/server.rs` and establish the dispatch
order yourself. Cite it by line.**

RFC 038 § Motivation 3 records what a previous verification found. Treat
that as a prior reading to check, **not as the answer**. Code changes,
and the specific reason this instruction exists is that my own RFC 037
handoff stated this order from memory — it was inverted and omitted a
stage entirely. The dev team caught it only because that handoff told
them to trace it rather than copy it.

The page owns: the order mechanisms are consulted; what wins when more
than one could match; and what `prefix`, `guard`, and per-rule-set
`strategy` do to that order.

### Two honesty requirements

- **`guard` currently does nothing.** Zero-field struct with a
  `// todo:` comment (RFC 036 Escalation 004). Do not describe it as
  affecting matching. Its disposition is an open owner decision — until
  then, document what is true.
- **`[default].delay_response_milliseconds` is inert** (RFC 036
  Escalation 002 → RFC 045). Do not document it as working.

## 5. Architecture and workspace — rewrites, not edits

`technical-reference/architecture.md` describes the pre-5.0.0
single-crate layout: `src/config.rs`, `src/server.rs`,
`src/core/server/routing.rs`. **None of those paths have existed for
fifteen releases.** `workspace.md` calls `apimock` the workspace-root
crate; it moved to `crates/apimock/` in 5.1.1.

Both are rewrites. The v5.14.0 handoff bundle's `external-design.md`
describes the four-crate structure and the one-way dependency direction
— a useful starting point, but **verify against the manifests**. That
bundle is itself a snapshot and has been wrong before.

## 6. Contributing — link, do not duplicate

The persona needs: how to build, how to run the tests, what the six gates
are, and how the RFC process works.

`.github/CONTRIBUTING.md` already carries the gate commands. **Link
it.** The docs page explains *what the gates are for* and *when they
run*; CONTRIBUTING stays the copy-pasteable list. One source per fact —
two copies of a procedure drift, and this milestone has watched that
happen twice.

Same for RFC 000 and the RFC process: link, summarise, do not restate.

Match `CONTRIBUTING.md`'s existing tone. It is explicit that pull
requests are reviewed but acceptance is not guaranteed; the docs section
should not over-invite.

## 7. Document integrity

| Defect | Location |
|---|---|
| Duplicate `## [5.4.0]` entries, different text | `CHANGELOG.md` lines 417 and 714 |
| Dead link to `docs/CONFIGURE.md` | `technical-reference/vision-and-goals.md` |
| Dead link to `./getting-started/rule-based-routing.md` | `user-guide/faq.md` — dies with the page under RFC 034 D5; confirm it does |

For the CHANGELOG: **determine which entry is accurate from git
history**, do not pick. Deleting one is a deletion from a historical
record — call it out in the review request.

## 8. Land complete sections, and share `SUMMARY.md`

`docs.yaml` deploys on **every push to `main`**. No staging. Each merge
must leave the site coherent — land a complete section at a time.

You add two sections' `SUMMARY.md` entries; RFC 035 adds three, in
parallel. Whoever lands second rebases. Do not restructure entries you do
not own.

## 9. Required tests

1. `mdbook build` succeeds; every `SUMMARY.md` entry resolves.
2. Every relative link resolves; both dead links in § 7 are gone.
3. `CHANGELOG.md` has exactly one `## [5.4.0]`, and it is the accurate
   one.
4. **Every path named in the architecture page exists.**
5. The matching-order page's claims cite `server.rs` by line.
6. No page describes `guard` or `[default].delay_response_milliseconds`
   as functional.
7. The site is coherent at every commit.

## 10. Prohibited shortcuts

- Writing the matching order from RFC 038's text rather than from
  `server.rs`.
- Copying `CONTRIBUTING.md` or RFC 000 into the docs.
- Describing `guard` or the default delay as working.
- Picking a `[5.4.0]` entry without checking git history.
- Fixing a product defect you find. Report it.

## 11. Escalation triggers

- **The dispatch order differs from RFC 038 § Motivation 3.** Report
  it — that would mean either the code changed or the prior reading was
  wrong, and both matter.
- **`benchmarks.md` makes a claim you cannot reproduce.** RFC 037
  removed an unverifiable k6 claim from the README; the same standard
  applies. Report rather than deciding alone.
- Any subject with no home in RFC 034's map.
- Any product defect.

## 12. Required evidence

- `mdbook build` output; link-check over the built site.
- The `server.rs` line citations behind the matching-order page.
- Git-history evidence for the `[5.4.0]` choice, and a note of what was
  deleted.
- Confirmation every path in the architecture page exists.
- Confirmation the site built coherently at each commit.

## 13. Required review-request format

Package at
`.git-exclude/review-request/038-technical-reference-and-document-integrity/`
with an entry-point file a reviewer can open cold. Per § 9.2 of the
workflow document. **Hand back one path — the entry-point file itself.**

Reviewer's focus: the matching-order page's citations, and whether
anything in Contributing duplicates rather than links.
