# RFC 040 — Trace channel: header redaction

**Status.** Proposed — approved by the project owner 2026-08-17.
**Amended 2026-08-17, during implementation:** goal 3 (non-JSON body
capture) is **removed from this RFC** — it could not be built inside the
scope this RFC set. See § Amendment.
**Tracks.** Security, and RFC 023's two deferred questions.
**Touches.** `crates/apimock-server/src/trace.rs` (including
`TraceConfig`, which lives there — *corrected 2026-08-17; this line
originally said `apimock-config`, which was wrong and impossible, since
`apimock-server` depends on `apimock-config` and not the reverse*), the
trace-event construction site in `server.rs`, documentation.
**No change to matching or response construction.**
**Related.** Resolves RFC 023 Unresolved 2; Unresolved 1 is **reopened**,
see § Amendment. Constrains
[RFC 048](./048-v6-cli-interface-concept.md) § 9's threat **T4**.

## Summary

The trace channel captures request bodies only when asked, capped by
size. It captures **every request header unconditionally**. Decide what
gets redacted, and do the redaction where it cannot be bypassed later.

~~Also settle how non-JSON bodies are represented, which RFC 023 left
open.~~ **Removed 2026-08-17 — see § Amendment.**

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
3. ~~Non-JSON bodies have a defined representation (RFC 023
   Unresolved 1).~~ **Removed — see § Amendment.**
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

### Non-JSON bodies — removed

See § Amendment. This RFC no longer covers them.

## Amendment — 2026-08-17 — goal 3 could not be built here

Goal 3 asked for a truncated UTF-8 snippet of a non-JSON body, captured
under the same size cap and the same redaction posture. **It cannot be
implemented within this RFC's stated Touches list**, and the reason is a
fact about the code that this RFC failed to check before specifying the
work.

`ParsedRequest` (`crates/apimock-routing/src/parsed_request.rs`) carries
`body_json: Option<Value>` and no raw bytes. The bytes are a local in
`parsed_request_from` (`crates/apimock-server/src/parsed_request.rs`):
collected, offered to `serde_json::from_slice`, and — for anything that
is not JSON — dropped when the function returns.

So a snippet must be produced **before** the bytes cease to exist, which
is upstream of where `RequestSummary` is built. That means either a new
field on `apimock_routing::ParsedRequest` or threading a snippet out of
`parsed_request_from` — and `ParsedRequest` is the type the whole
matching pipeline is built on, not a contained type like
`RequestSummary`.

This RFC's load-bearing principle is *redact at capture*. Capture, for
this channel, happens downstream of the point where a non-JSON body
still exists. The two do not meet, and this document did not notice.

**Consequences.**

- RFC 040 is now header redaction only, and is complete as such.
- **RFC 023's Unresolved 1 returns to open.**
- The follow-up is
  [RFC 050](./050-non-json-body-capture-decision.md), deliberately framed
  as *whether* non-JSON bodies should be captured rather than how. This
  RFC's own argument is that the channel captures more than it should;
  capturing non-JSON bodies makes it capture more. That question was not
  visible until someone tried to build it.

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
2. ~~**Does the GUI display trace headers today?**~~ ✅ **Answered
   2026-08-17: yes, it does.** So GUI users debugging an auth failure
   will now see `[redacted]` where a credential value used to be.

   **Implementation cost to the GUI: none.** The event's shape is
   unchanged — same field, same type, same position — so no GUI code
   has to change. What changes is what a user reads.

   **But the escape hatch is unreachable from the GUI**, and that is the
   real cost. `header_denylist` lives on `TraceConfig`, which has no
   config-file surface at all (the same pre-existing state
   `capture_body` has had since RFC 023), so a GUI user cannot opt a
   header back in even deliberately. If that matters, giving
   `TraceConfig` a configuration surface is its own piece of work —
   larger than this RFC and not assumed by it.
3. **Should the redaction policy be shared with v6's `get`, or applied
   independently?** Sharing it is the point of redacting at capture, but
   `get` answers from configuration rather than from a live request, so
   the surfaces may not be identical. Revisit when `get` is designed —
   noted so it is not forgotten.
