# Implementation Handoff — RFC 040, trace redaction and non-JSON body capture

**Governing RFC.** [RFC 040](../../done/040-trace-capture-and-redaction.md)
**Milestone.** M3 — P1, targeting **v5.19.0**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

The trace channel gates *body* capture behind a flag and a size cap, and
captures **every request header unconditionally**. Fix that, and settle
how non-JSON bodies are represented.

This is security work on a channel that is library-only *today* and
will be surfaced by v6's `get` tomorrow. The point of doing it now is
that redaction designed after an interface exists has to be retrofitted
into something callers already depend on.

## 2. Establish from source first

RFC 040's claims are line-cited, so they are checkable — **check them**:

- `RequestSummary` at `crates/apimock-server/src/trace.rs:80`, and that
  it derives `Serialize`.
- The construction site at `crates/apimock-server/src/server.rs:365` —
  specifically that the only filter is `v.to_str().ok()`, i.e. drops
  non-UTF-8 values and selects nothing by name.
- Whether that is the **only** place a `RequestSummary` is built on a
  live request path. The RFC assumes it is. If there is a second site,
  redaction must cover it too, and that is exactly the kind of thing the
  RFC could have missed.

If any of it contradicts the RFC, **report the contradiction** rather
than designing around it.

## 3. Decided

**Q1 — denylist or allowlist by default? → DENYLIST**, per the RFC's
recommendation, accepted with the RFC on 2026-08-17. Allowlist mode
available by configuration; the denylist itself configurable. The
default — meaning *no configuration at all* — must be safe.

**Q2 — does the GUI display trace headers? → NOT BLOCKING, but
coordinate.** Still unanswered; it needs the GUI side and we cannot see
it from here. It does not block you, because RFC 040 requires *marking
rather than omitting*: a redacted header keeps its name and gets a
placeholder value. The event's **shape** is therefore unchanged, so a
consumer that renders a header list keeps working — only values differ.

Do not design anything that depends on knowing the answer. Flag in your
review request that the GUI team needs telling that values will be
redacted, so it reaches them as a notification rather than a surprise.

**Q3 — share the redaction policy with v6's `get`? → DEFERRED**, as the
RFC says. Do not build for it. Do leave the redaction decision in one
place, which is what § 4 requires anyway.

## 4. The load-bearing requirement

**Redact at capture, not at display.** The redaction happens where
`RequestSummary` is built, so that no formatter — v6's `get`, the GUI,
a log dump, anything added later — can leak what was never captured, and
none of them has to remember to re-apply a rule.

If you find yourself adding redaction to a serialisation or display
path, stop: that is the design this RFC exists to avoid, and it is how
the current gap arose.

## 5. Two traps worth naming

**Header names are case-insensitive.** `Authorization`, `authorization`
and `AUTHORIZATION` are the same header. A denylist compared
case-sensitively is a leak that will pass a naive test. Match
case-insensitively and prove it with a test that uses a non-lowercase
spelling.

**The RFC's example denylist includes `set-cookie`, which is a
*response* header.** My slip; the RFC's own non-goals say requests only.
Harmless if it stays in the list — it simply never matches — but do not
let it imply response coverage that isn't there.

## 6. Non-JSON bodies

Per the RFC: a **truncated UTF-8 snippet**, subject to the same size cap
and the same redaction posture, with a flag distinguishing *not
captured* from *captured and truncated* — mirroring the existing
`body_truncated`. Not base64 raw capture: it takes form-encoded
credentials verbatim while looking opaque enough that nobody inspects
it.

## 7. Evidence required

- A request carrying `authorization`, `cookie` and `x-api-key` produces
  an event where **none of the three values appears in the serialised
  output**. Assert on the serialised form, not the struct — serialisation
  is what reaches a consumer.
- The same, with a non-lowercase header name (§ 5).
- Redacted headers are **present and marked**, not absent.
- **The default configuration is safe** — demonstrate with no trace
  configuration whatsoever, since that is the common case.
- Non-JSON body captured within the cap, truncation flagged distinctly
  from not-captured, and redaction applied.
- Existing trace tests pass unchanged.
- Full suite green; report the count against the **425** baseline.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 8. Scope boundaries

- **In:** `trace.rs`, the `RequestSummary` construction site(s),
  `TraceConfig`, documentation.
- **Out:** response bodies and server-side secrets; value-scanning
  heuristics (explicit non-goal — name-based only); when trace events
  are emitted; the dispatch path; anything in v6's `get`.
- If redaction starts reaching into matching or response construction,
  stop and escalate.

## 9. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package — including a § 2 contradiction,
and including a second `RequestSummary` construction site if you find
one, since that would widen the change beyond what the RFC scoped.
