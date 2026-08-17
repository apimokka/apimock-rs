# Acceptance / QA Checklist — RFC 040

**Governing RFC.** [RFC 040](../../proposed/040-trace-capture-and-redaction.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## Established from source, not inherited

- [ ] `RequestSummary` (`trace.rs:80`) and its `Serialize` derive
      confirmed
- [ ] The construction site (`server.rs:365`) confirmed to filter only
      non-UTF-8, selecting nothing by name
- [ ] **Whether a second `RequestSummary` construction site exists on a
      live request path** — established, and covered if so
- [ ] Any contradiction with the RFC **reported**

## Redaction happens at capture

- [ ] Redaction applied where `RequestSummary` is built
- [ ] **No** redaction logic in a serialisation or display path
- [ ] The policy lives in one place

## What gets redacted

- [ ] Denylist by default (Q1, decided)
- [ ] Allowlist mode available by configuration
- [ ] Denylist itself configurable
- [ ] **Matching is case-insensitive** — proven with a non-lowercase
      spelling, not only `authorization`
- [ ] Redacted headers **present and marked**, never silently absent

## The default must be safe

- [ ] With **no trace configuration at all**, credential headers are
      redacted — demonstrated, not asserted

## Evidence on the serialised form

- [ ] `authorization`, `cookie`, `x-api-key` values absent from the
      **serialised** event, not merely from the struct
- [ ] Non-lowercase spelling covered

## Non-JSON bodies

- [ ] Truncated UTF-8 snippet, not base64 raw capture
- [ ] Same size cap as JSON capture
- [ ] Same redaction posture
- [ ] *Not captured* distinguishable from *captured and truncated*

## Compatibility

- [ ] Existing trace tests pass unchanged
- [ ] Event **shape** unchanged, so a header-rendering consumer keeps
      working
- [ ] Review request flags that **the GUI team needs telling** values
      will be redacted (Q2)

## Scope held

- [ ] Response bodies untouched
- [ ] No value-scanning heuristics — name-based only
- [ ] Emission timing and dispatch path unchanged
- [ ] Nothing built for v6's `get` (Q3 deferred)

## Suite and gates

- [ ] Full suite green; count reported against the **425** baseline
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
