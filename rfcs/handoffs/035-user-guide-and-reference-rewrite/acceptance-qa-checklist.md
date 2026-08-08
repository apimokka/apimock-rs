# Acceptance / QA Checklist — RFC 035

**Governing RFC.** [RFC 035](../../done/035-user-guide-and-reference-rewrite.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## The operator reference — the reviewer's focus

- [ ] Tables **generated or mechanically checked** against the enums, not
      transcribed
- [ ] Method stated in the review request
- [ ] All 11 `RuleOp` variants present
- [ ] All 13 `HeaderOperator` variants present
- [ ] All 25 `BodyOperator` variants present
- [ ] Check re-runnable by the reviewer

## Correctness

- [ ] `strategy = "first_match"  # only value supported today` — gone
- [ ] "Currently the only strategy" — gone; all five documented, plus the
      RFC 025 per-rule-set override
- [ ] Every subject documented was verified **against code**, not from an
      RFC title or a prior page
- [ ] No page states a feature does not exist when it does
- [ ] Corrections beyond the handoff § 8 minimum are **listed**

## Coverage — each previously returned zero matches

- [ ] `apimock match-test`
- [ ] `structural_contains`, `map_has_key`
- [ ] `round_robin`, `weighted_random`, `uniform_random`
- [ ] `respect_gitignore`, `extra_excludes`, file-tree filtering
- [ ] TLS hot-reload
- [ ] negated operators
- [ ] `apimock validate`, rule `priority` / `weight` — as features, not
      incidental mentions

## Structure — RFC 034 § D5

- [ ] Getting started: 3 pages, linear
- [ ] Guides: 13 pages, each standalone
- [ ] Reference: 6 pages
- [ ] Rule-set schema has **one** authoritative home (D7)
- [ ] No section invented outside the map

## Build and links

- [ ] `mdbook build` succeeds
- [ ] Every `SUMMARY.md` entry resolves
- [ ] Every relative link resolves
- [ ] Site coherent at **every** commit — no half-migrated section merged
- [ ] `SUMMARY.md` coordinated with RFC 038; no entries clobbered

## Non-change scope

- [ ] How it works / Contributing untouched (RFC 038)
- [ ] `README.md` untouched
- [ ] `crates/apimock/examples/` untouched
- [ ] No crate source changed

## Escalations to report

- [ ] Any subject with no home in RFC 034's map
- [ ] Any feature whose behaviour differs from every existing description
- [ ] Any product defect found — report, do not fix

## Review-request package

- [ ] `.git-exclude/review-request/035-user-guide-and-reference-rewrite/`
- [ ] Entry point orients a cold reader; all 10 items from § 9.2
- [ ] States the operator-check method
- [ ] Hand back **one path** — the entry-point file
