# Acceptance / QA Checklist — RFC 045

**Governing RFC.** [RFC 045](../../proposed/045-configuration-accepted-but-ignored.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## Established from source, not inherited

- [ ] The RFC's line-cited claims (`status_code_response.rs:6-25`,
      `respond_response.rs:78-81`, `respond_response.rs:45`,
      `rule_set.rs:312`) **checked**, not assumed
- [ ] Any contradiction **reported** rather than designed around

## Defect 1 — `respond.headers` on every shape

- [ ] `file_path` — headers honoured, **behaviour unchanged**
- [ ] `text` alone — headers honoured, explicit `content-type` no longer
      overwritten
- [ ] `text` + `status` — headers honoured
- [ ] `status` alone — headers honoured
- [ ] Precedence stated once and applied everywhere: explicit beats
      inferred
- [ ] `file_path`'s extension-inferred `content-type` **not** regressed —
      demonstrated by the existing suite, not by argument

## Defect 2 — the rule-set default delay

- [ ] `[default].delay_response_milliseconds` actually delays
- [ ] Per-rule `respond.delay_response_milliseconds` still overrides it
- [ ] Before/after measurement reported (symptom was 2000 ms configured,
      ~4 ms observed)
- [ ] **CHANGELOG-worthy behaviour change flagged** in the review request

## Goal 4 — validation of inert configuration

- [ ] Option 3's practicality **investigated and reported either way**
- [ ] If impractical: the reason given, and the fallback named explicitly
- [ ] No decision made by omission
- [ ] Scope held — a decision and a principle, not an exhaustive audit

## Tests

- [ ] One test per `respond` shape, header survival asserted
- [ ] Explicit `content-type` beats default
- [ ] Rule-set default delay works; per-rule overrides
- [ ] Full suite green; **new count reported** against the 409 baseline

## Scope held

- [ ] `respond` schema unchanged; no new fields
- [ ] Examples **not** simplified
- [ ] Example READMEs/comments checked for prose these fixes falsify —
      files checked are named (expected: none)
- [ ] `guard` untouched (owner decision, still open)
- [ ] Trace channel untouched

## Gates

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
