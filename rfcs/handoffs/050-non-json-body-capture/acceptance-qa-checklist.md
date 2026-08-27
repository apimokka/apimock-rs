# Acceptance / QA Checklist — RFC 050

**Governing RFC.** [RFC 050](../../done/050-non-json-body-capture-decision.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## Content is never captured

- [ ] No bytes, no snippet, no truncated preview — **length and presence
      only**
- [ ] A recognisable string from a non-JSON body **does not appear**
      anywhere in the serialised event
- [ ] No second copy of `content-type` — it is already in
      `RequestSummary.headers`

## Three states, distinguishable

- [ ] **No body** · **JSON body captured** · **non-JSON body present,
      size known, not captured** — asserted on the **serialised** form
- [ ] JSON path unchanged: `body_json` / `body_truncated` behave as
      before, existing tests untouched

## The consumer check — real work, not a formality

- [ ] Every `ParsedRequest` consumer enumerated, and **named** in the
      review request
- [ ] Matcher, middleware and `dyn_route` all confirmed unaffected
- [ ] If the field proved not purely additive anywhere, **escalated**
      rather than reshaping consumers to fit

## Scope held

- [ ] Populated always; no gating on tracing being active
- [ ] Response bodies untouched
- [ ] Matching, dispatch, response construction untouched
- [ ] `log.verbose` logging untouched — that is RFC 051

## Public-API break — R-09 *(added 2026-08-17)*

- [ ] Checked whether the change adds a field to `ParsedRequest` and `RequestSummary` — all `pub`,
      public fields, **not** `#[non_exhaustive]`
- [ ] Any such addition **reported as a breaking change**, not described
      as "additive"
- [ ] `#[non_exhaustive]` **not** added as a tidy-up — owner decision
      pending across RFCs 040/050/051

## Suite and gates

- [ ] Full suite green; count reported against the **430** baseline
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
