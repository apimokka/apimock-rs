# Implementation Handoff — RFC 051, verbose-log header redaction

**Governing RFC.** [RFC 051](../../done/051-verbose-log-header-redaction.md)
**Milestone.** M3 — **P1, security**, targeting **v5.19.0**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

RFC 040 stopped the trace channel emitting credential headers.
`capture_in_log` still prints every request header verbatim to the
console when `log.verbose.header` is on. Apply the same policy to the
same data in the other place it leaves the process.

**Why P1 despite being opt-in.** The trace channel is library-only, so
what it captured stayed in-process. This writes to a running server's
console — CI job logs, terminal scrollback, screen shares, pasted bug
reports. And it gets switched on precisely when someone is debugging a
rejected request, which is the moment an `authorization` header is most
interesting and most likely to end up in an issue.

## 2. The design question is yours, and it is the main one

RFC 051 deliberately does not prescribe **where the shared policy
lives**.

Today `header_redaction` / `header_denylist` / `header_allowlist` and the
private `redact_headers` are on `TraceConfig`
(`crates/apimock-server/src/trace.rs`). Verbose logging is configured by
`log.verbose` in `apimock-config` — a different crate and a different
surface.

**Establish the dependency direction from source before designing.**
`apimock-server` depends on `apimock-config`, not the reverse — RFC 040's
own Touches line got this wrong and claimed `TraceConfig` lived in
`apimock-config`, which was impossible. Do not inherit that mistake from
a document; check it.

Report the choice with its reasoning. What matters is **goal 2: one
definition, not a copy.** A duplicated denylist is the failure mode
here — two lists that agree today and drift in six months, with nobody
noticing which emitter honours which.

## 2b. Before you finish — "additive" is not "non-breaking"

**Added 2026-08-17, after this handoff was issued.**

Whatever you choose in § 2, check whether it adds a field to a public
struct. Every candidate here is `pub`, has public fields, and is **not**
`#[non_exhaustive]`:

```
LogConfig      // apimock-config, exported at lib.rs:38
VerboseConfig  // apimock-config, pub fields
TraceConfig    // apimock-server::trace
```

Adding a field to any of them breaks downstream struct literals. TOML
compatibility is fine — old config files still parse, the new field
takes its default — but the **Rust API break is real**.

This has already happened once unnoticed: RFC 040 added three fields to
`TraceConfig`, and that break is unreleased on `main`.

**Decided 2026-08-17 — and it changes your primary instruction.**

**Try first to land this RFC with *no* new public fields at all.** Share
RFC 040's denylist by reference; do not add a configuration surface for
it. If that works, this security fix ships in an honest minor release
while all the API churn defers to the v6 boundary — which is the outcome
we want, because a delayed security fix and a broken semver promise are
both costs and this avoids paying either.

If it cannot be done without adding a public field, **stop and escalate**
rather than adding one. That is a scope decision now, not an
implementation detail.

Either way, do **not** add `#[non_exhaustive]` yourself. The owner has
approved applying it to all five affected types — `TraceConfig`,
`RequestSummary`, `ParsedRequest`, `LogConfig`, `VerboseConfig` — but as
**one coordinated change**, [RFC 052](../../done/052-non-exhaustive-public-types.md),
not piecemeal inside three RFCs. Recorded as **R-09** in `ROADMAP.md`.

Worth knowing: `apimock-config`'s `view.rs` types *are*
`#[non_exhaustive]`, deliberately and with a comment saying why. So the
idiom is known here and applied in one place and not others — this is
drift, not unawareness, which is also why a single decision across all
three is better than three local judgements.

## 3. Constraints

- **Reuse RFC 040's configuration surface.** Do not invent a parallel
  one. If sharing proves genuinely impractical, that is an escalation,
  not a licence to copy the list.
- **Do not change `log.verbose.header`'s default.** It is off; this RFC
  makes it safe when on, not harder to turn on.
- **Marked, not omitted** — consistent with RFC 040. A reader must be
  able to tell "this header was present and redacted" from "this header
  wasn't sent".
- **Case-insensitive matching.** RFC 040's implementation used
  `eq_ignore_ascii_case`. A second call site is a second chance to get
  this wrong, and a test that only tries lowercase `authorization` will
  pass while the code leaks.

## 4. `log.verbose.body` — establish, then say

RFC 051's Unresolved 2 asks whether the body-logging path has the same
problem. **Almost certainly yes** — a logged form-encoded body carries
credentials as readily as a header does.

Establish it from source. Then say whether it belongs in this change or
its own, with the reasoning. Do not silently fix it and do not silently
skip it.

Note the interaction: [RFC 050](../../done/050-non-json-body-capture-decision.md)
decided the *trace channel* captures body presence only, never content.
If verbose logging prints body content to a console, that is a stronger
version of the same exposure, and the two answers should at least be
consistent with each other.

## 5. Evidence required

- With `log.verbose.header` on and **no other configuration**, a request
  carrying `authorization`, `cookie` and `x-api-key` produces log output
  containing none of the three values — asserted on the **rendered log
  line**, since that is what reaches a terminal.
- A **non-lowercase** spelling (`Authorization`, `COOKIE`) is redacted
  too.
- Redacted headers appear **marked**, not omitted.
- A non-credential header's value **still appears** — the logger has to
  stay useful, and a change that silently redacted everything would pass
  a careless reading of the tests above.
- Whatever § 4 establishes about `log.verbose.body`, reported.
- Full suite green; report the count against the **430** baseline.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 6. Scope boundaries

- **In:** `capture_in_log` in `crates/apimock-server/src/parsed_request.rs`,
  whatever is needed to share RFC 040's policy, documentation.
- **Out:** response headers and bodies; value-scanning heuristics;
  changing when or whether logging happens; matching, dispatch, response
  construction.
- If sharing the policy starts requiring a dependency inversion between
  crates, **stop and escalate** — that is a design change, not an
  implementation detail.

## 7. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package — including the § 2 placement
decision if it turns out worse than it looks, and § 4's finding either
way.
