# Acceptance / QA Checklist — RFC 051

**Governing RFC.** [RFC 051](../../done/051-verbose-log-header-redaction.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## One policy, not two

- [ ] RFC 040's redaction policy **shared**, not copied
- [ ] Placement decision **reported with its reasoning**
- [ ] Dependency direction **established from source**, not inherited
      from a document (RFC 040's Touches line got it wrong)
- [ ] No parallel configuration surface invented

## Redaction works where it now matters

- [ ] With `log.verbose.header` on and **no other configuration**,
      `authorization` / `cookie` / `x-api-key` values absent from the
      **rendered log line**
- [ ] **Non-lowercase** spellings redacted too
- [ ] Redacted headers **marked**, not omitted
- [ ] A non-credential header's value **still appears** — the logger
      stays useful

## `log.verbose.body`

- [ ] Established from source whether it has the same problem
- [ ] Reported either way, with whether it belongs here or in its own
      change
- [ ] Consistency with RFC 050's "presence only, never content"
      considered

## Scope held

- [ ] `log.verbose.header`'s default unchanged (still off)
- [ ] Response headers and bodies untouched
- [ ] No value-scanning heuristics
- [ ] Logging timing and dispatch unchanged
- [ ] A required dependency inversion would have been **escalated**

## Public-API break — R-09 *(added 2026-08-17)*

- [ ] Checked whether the change adds a field to `LogConfig`, `VerboseConfig` or `TraceConfig` — all `pub`,
      public fields, **not** `#[non_exhaustive]`
- [ ] Any such addition **reported as a breaking change**, not described
      as "additive"
- [ ] `#[non_exhaustive]` **not** added as a tidy-up — owner decision
      pending across RFCs 040/050/051

## Suite and gates

- [ ] Full suite green; count reported against the **430** baseline
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
