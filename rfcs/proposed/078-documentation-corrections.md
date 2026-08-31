# RFC 078 — Correct four false statements, and give users somewhere to look

**Status.** Proposed — awaiting owner approval.
**Tracks.** Documentation. External audit 2026-09-01, D-01 through
D-07, and the Troubleshooting gap (scored 2/5, the lowest in the
documentation audit).
**Touches.** `docs/src/guides/`, `docs/src/reference/`,
`crates/apimock-server/src/trace.rs` module docs.

## Summary

Four documented statements are false, three user-visible behaviours are
documented nowhere, and there is no troubleshooting page — so a user who
hits any of the three has nothing to search.

## Motivation

This project's documentation is unusually honest — the audit says so,
and the TLS hot-reload guide, which opens by stating the feature is
unreachable, is the example. That is what makes the false statements
worth fixing rather than shrugging at: a reader who has learned to
trust these pages has no reason to doubt them.

### The four false statements

| | Says | Actually |
|---|---|---|
| **D-01** `vary-the-response-for-one-path.md` | `round_robin` "cycles through matches in file order … Deterministic" | True only for a rule set with one match group (RFC 070) |
| **D-02** `trace.rs:26-34` | `emit` drops and counts on a full channel; gaps reported via `dropped_count` | `broadcast::send` fails only with no receivers; `Lagged(n)` is discarded (RFC 073) |
| **D-03** TLS reload guide | An embedder "could reach `ServerHandle` and `reload_tls_certs` itself" | `ServerHandle` is `#[non_exhaustive]`, so out-of-crate construction is a compile error, and nothing returns one. **The stated workaround is impossible** |
| **D-06** `response-headers.md` | `connection: keep-alive` always present | Absent over HTTP/2 — hyper strips it correctly per RFC 9113 §8.2.2 |

D-03 is the sharpest: a reader following it writes code that cannot
compile.

### D-04 / D-05 — the threat model's two gaps

- **CORS is absent entirely** from a page whose stated purpose is to
  enumerate "what apimock allows on purpose and why" (RFC 067).
- **Middleware non-termination** is unmentioned; the page says a failing
  script "cannot crash the process, but it can silently degrade
  routing", which is right about crashing and wrong about a script that
  never returns (RFC 068).

### D-07 — three undocumented behaviours

JSON key reordering and minification (RFC 076), no percent-decoding
(RFC 075), and last-segment-only case-insensitivity (RFC 075). **Each
presents as an inexplicable 404 or a failing snapshot test.**

## Goals

1. No page states something false.
2. The threat model covers CORS and middleware non-termination.
3. A user hitting one of the three behaviours can find out why.

## Non-goals

- Restructuring the documentation.
- Documenting behaviours that RFCs 070/073/075/076 are about to change —
  see the sequencing note below.

## Design

**Sequencing is the whole design question here**, because most of these
statements are false about behaviour other RFCs will change:

| Statement | If its RFC lands | If it does not |
|---|---|---|
| D-01 round_robin | Becomes true; no doc change needed beyond removing the caveat | Must be corrected to describe the single-group constraint |
| D-02 back-pressure | Becomes true if RFC 073 implements the counter | Must be corrected to describe `broadcast` |
| D-07 (a) JSON order | Disappears with RFC 076 | Must be documented as a known transformation |
| D-07 (b) percent-decoding | Disappears with RFC 075 | Must be documented as a limitation |

> **Do not write documentation for behaviour that is about to change.**
> Land this RFC's D-03, D-04, D-05, D-06 and the troubleshooting page
> **now** — none depends on another RFC. Hold D-01, D-02 and D-07 until
> their RFCs are decided, then either delete the item or document the
> limitation.
>
> If those RFCs are rejected or deferred, D-01/D-02/D-07 become
> mandatory and urgent: an undocumented wrong behaviour is worse than a
> documented one.

**The troubleshooting page** should be organised by *symptom*, since
that is what a user has: "my file 404s", "my rule matches everything",
"my snapshot test broke". Cross-link the error-`kind` taxonomy, which is
already closed and documented and is a genuine strength to build on.

## Testing and verification

- Every corrected statement checked against a running server or a
  compile, not against the code by reading.
- **D-03 specifically: attempt the suggested workaround** and confirm it
  does not compile, before writing the correction.
- `mdbook build` clean; the link and stub checks pass.
- Every symptom in the troubleshooting page reproduced, and its stated
  fix applied, before it is written down.

## Risks

| Risk | Mitigation |
|---|---|
| Documentation written for behaviour that then changes | The sequencing table above; that is what it is for |
| Troubleshooting page becomes stale | Each entry names a symptom and a check the reader can run, so a stale entry fails visibly rather than misleading quietly |
