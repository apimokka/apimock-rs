# Acceptance / QA Checklist — RFC 038

**Governing RFC.** [RFC 038](../../done/038-technical-reference-and-document-integrity.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## Matching order and precedence — the reviewer's focus

- [ ] Dispatch order **established from `crates/apimock-server/src/server.rs`**,
      not copied from RFC 038's text
- [ ] Claims **cited by line**
- [ ] Covers: consultation order, what wins on multiple matches, and the
      effect of `prefix`, `guard`, per-rule-set `strategy`
- [ ] Any difference from RFC 038 § Motivation 3 is **reported**

## Honesty requirements

- [ ] `guard` **not** described as affecting matching (zero-field stub,
      `// todo:`)
- [ ] `[default].delay_response_milliseconds` **not** described as
      working (inert — RFC 045)

## Rewrites

- [ ] `architecture.md` rewritten; **every path it names exists**
- [ ] `workspace.md` rewritten; no longer calls `apimock` the
      workspace-root crate
- [ ] Both verified against the manifests, not just against the v5.14.0
      handoff bundle

## Contributing

- [ ] Build, test, gates, and RFC process covered
- [ ] `.github/CONTRIBUTING.md` **linked, not duplicated**
- [ ] RFC 000 **linked, not restated**
- [ ] Tone matches CONTRIBUTING's — does not over-invite

## Document integrity

- [ ] `CHANGELOG.md` has exactly one `## [5.4.0]`
- [ ] Which entry was accurate determined from **git history**
- [ ] The deletion is called out in the review request
- [ ] `docs/CONFIGURE.md` dead link gone
- [ ] `faq.md`'s dead link gone (confirm the page's deletion covers it)

## Build and links

- [ ] `mdbook build` succeeds
- [ ] Every `SUMMARY.md` entry resolves
- [ ] Every relative link resolves
- [ ] Site coherent at **every** commit
- [ ] `SUMMARY.md` coordinated with RFC 035

## Non-change scope

- [ ] Getting started / Guides / Reference untouched (RFC 035)
- [ ] `README.md` untouched
- [ ] No crate source changed

## Escalations to report

- [ ] Dispatch order differing from RFC 038 § Motivation 3
- [ ] `benchmarks.md` claims that cannot be reproduced
- [ ] Any subject with no home in RFC 034's map
- [ ] Any product defect found

## Review-request package

- [ ] `.git-exclude/review-request/038-technical-reference-and-document-integrity/`
- [ ] Entry point orients a cold reader; all 10 items from § 9.2
- [ ] Hand back **one path** — the entry-point file
