# RFC 040 — Trace channel: redaction, and non-JSON body capture

**Status.** Proposed — awaiting owner approval.
**Tracks.** Security, and RFC 023's two deferred questions.
**Touches.** `crates/apimock-server/src/trace.rs`, the trace-event
construction site in `server.rs`, `TraceConfig` in `apimock-config`,
documentation. **No change to matching or response construction.**
**Related.** Resolves RFC 023 Unresolved 1 and 2. Constrains
[RFC 048](./048-v6-cli-interface-concept.md) § 9's threat **T4**.

## Summary

The trace channel captures request bodies only when asked, capped by
size. It captures **every request header unconditionally**. Decide what
gets redacted, and do the redaction where it cannot be bypassed later.

Also settle how non-JSON bodies are represented, which RFC 023 left open.

## Motivation

### The asymmetry, established from source

`RequestSummary` (`crates/apimock-server/src/trace.rs:80`) carries:

- `body_json: Option<serde_json::Value>` — present *only* when
  `TraceConfig::capture_body` is true, and only within a size cap. Opt-in
  and bounded.
- `headers: Vec<(String, String)>` — documented as *"Selected request
  headers (display-only)"*.

**Nothing selects.** At the construction site
(`crates/apimock-server/src/server.rs:365`):

```rust
headers: parsed_request
    .component_parts
    .headers
    .iter()
    .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_owned())))
    .collect(),
```

The only filter drops values that are not valid UTF-8. `authorization`,
`cookie`, `x-api-key` — every one is captured, and `RequestSummary`
derives `Serialize`, so every one is serialisable by any consumer.

So the deliberate, gated, size-capped treatment was given to bodies, and
headers — which is where credentials actually live — got none of it. The
doc comment saying "selected" is the kind of sentence that stops anyone
looking closer.

### Why this is worth doing before v6, not after

Today the trace channel is library-only: no CLI surface, no config
surface, so captured data stays in the process that captured it. That
bounds the exposure, and it is why this has been survivable.

RFC 048's v6 introduces `get` and the W3 "why did this match?" output,
which surfaces matched request detail to **stdout** — into CI logs, into
an agent's transcript. That is threat **T4**. Once that path exists,
whatever the trace channel captures becomes something callers can print.

Designing redaction afterwards means retrofitting it into an interface
people already depend on. The owner's guidance applies directly here:
an interface can be reconsidered later, but it costs, so design with a
wide perspective and security in mind up front.

## Goals

1. Credential-bearing headers are not captured by default.
2. Redaction happens **at capture**, not at each point of display.
3. Non-JSON bodies have a defined representation (RFC 023 Unresolved 1).
4. What is redacted is visible to the consumer — a redacted field is
   marked, not silently absent, so nobody mistakes redaction for "the
   request didn't have one".
5. The GUI's existing trace consumption keeps working.

## Non-goals

- Redacting response bodies or server-side secrets. Requests only.
- A general secret-detection heuristic over values. Name-based only —
  scanning values for things that look like tokens is a different, much
  larger problem with a worse false-positive story.
- Changing when trace events are emitted, or the dispatch path.
- Building v6's `get`. This RFC constrains it; it does not implement it.

## Proposed design

### Redact at capture — the load-bearing decision

Redaction belongs in the construction of `RequestSummary`, not in
whatever formats it. A future output path — v6's `get`, a GUI panel, a
log dump — **cannot leak what was never captured**, and does not have to
remember to re-apply a rule.

The alternative, redacting per display site, fails the moment someone
adds a display site. That is precisely how the current situation arose:
the body path was thought about, the header path was not.

### Which headers

Two coherent policies, and this is the decision I want taken
deliberately rather than by default:

**A. Denylist.** Redact a built-in set of well-known credential headers
(`authorization`, `cookie`, `set-cookie`, `proxy-authorization`,
`x-api-key`, …), configurable. Familiar, preserves most debugging value,
and **fails open**: a header named something we did not anticipate is
captured in full.

**B. Allowlist.** Capture only headers named in configuration, redact
everything else. **Fails closed**, and matches the project's stated
preference for the safer option where the two conflict — but it changes
what existing trace consumers see, and makes the channel less useful out
of the box for the ordinary debugging case.

**Recommendation: A as the default, with B available by configuration**,
and the denylist itself configurable. Rationale: the trace channel's
purpose is debugging, an allowlist default would make it near-useless on
first use, and the realistic threat is a *known* credential header
reaching a log rather than an exotic one. But this is a judgement about
product risk, not a technical fact, so it is the owner's to confirm or
overturn.

Whichever is chosen, **the default must be safe with no configuration at
all** — the common case is someone who never opens the trace settings.

### Marking, not omitting

A redacted header appears with its name and a fixed placeholder rather
than vanishing. A consumer that sees no `authorization` header cannot
otherwise tell whether the request lacked one or the value was removed,
and that difference matters when debugging an auth failure — the exact
situation in which someone reaches for the trace channel.

### Non-JSON bodies (RFC 023 Unresolved 1)

RFC 023 captured JSON only and omitted everything else. Options it
named: raw bytes as base64, or a truncated text snippet.

**Recommendation: a truncated UTF-8 snippet, subject to the same size
cap and the same redaction posture**, with a flag distinguishing "not
captured" from "captured and truncated" — mirroring the existing
`body_truncated`. Base64 raw capture is more faithful and considerably
more dangerous: it captures form-encoded credentials verbatim while
looking opaque enough that nobody inspects it.

If raw capture is wanted later, it should be its own opt-in, argued
separately.

## Testing and verification

- A request carrying `authorization`, `cookie` and `x-api-key` produces
  a trace event where none of the three values appears — asserted on the
  **serialised** event, not the struct, since serialisation is what
  reaches a consumer.
- Redacted headers are present-and-marked, not absent.
- The default configuration — i.e. none — is safe.
- Non-JSON bodies: captured within the cap, truncation flagged, and
  subject to the same redaction.
- Existing trace tests pass unchanged, and the GUI's consumption path is
  confirmed unbroken.
- Full suite green; report the count against the **425** baseline.

## Risks

| Risk | Mitigation |
|---|---|
| Redaction makes the channel useless for debugging auth problems | Marking rather than omitting; the denylist is configurable, so a developer can opt a header back in deliberately |
| A denylist default fails open on an unanticipated header name | Stated openly as the cost of recommendation A; allowlist mode exists for anyone who wants closed-by-default |
| Existing consumers depend on seeing every header | The GUI is the known consumer and must be checked; anything else is out of our sight, which is itself an argument for doing this while the channel is still library-only |
| Scope drifts into value-scanning heuristics | Explicit non-goal |

## Unresolved questions

1. **Denylist or allowlist by default?** Recommendation A above; owner's
   call, because it is a product-risk judgement.
2. **Does the GUI display trace headers today, and would redaction
   change what its users see?** Establish from the GUI side — this is
   the one consumer we know about and cannot inspect from here.
3. **Should the redaction policy be shared with v6's `get`, or applied
   independently?** Sharing it is the point of redacting at capture, but
   `get` answers from configuration rather than from a live request, so
   the surfaces may not be identical. Revisit when `get` is designed —
   noted so it is not forgotten.
