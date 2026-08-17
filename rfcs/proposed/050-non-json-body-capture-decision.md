# RFC 050 — Should non-JSON request bodies be captured at all?

**Status.** Proposed — awaiting owner approval. **This is a decision
RFC.** It asks a question and recommends an answer; it does not specify
an implementation, because the implementation is only worth designing if
the answer is yes.
**Tracks.** Security, and RFC 023's Unresolved 1 — reopened by
[RFC 040](./040-trace-capture-and-redaction.md)'s amendment.
**Touches.** Nothing, if the answer is no. If yes:
`crates/apimock-routing/src/parsed_request.rs` and
`crates/apimock-server/src/parsed_request.rs` — the type the whole
matching pipeline is built on.

## Summary

RFC 023 captured JSON request bodies and omitted everything else,
deferring non-JSON representation as *"a follow-up RFC can add raw
capture"*. RFC 040 tried to add it, discovered it cannot be built where
it was scoped, and in the process produced an argument that the feature
may not be wanted.

Answer the question before designing anything.

## Motivation

### Why this is being asked rather than built

RFC 040 specified a truncated UTF-8 snippet for non-JSON bodies. During
implementation it became clear that the data is gone by the time the
trace event is built: `ParsedRequest` carries `body_json: Option<Value>`
and no raw bytes, and the bytes are a local in `parsed_request_from`
that is dropped for anything that is not JSON.

So capturing a non-JSON body means **adding a field to
`apimock_routing::ParsedRequest`** — the type the matcher, middleware and
`dyn_route` all consume — and populating it on every request whether or
not anyone is tracing. That is a materially larger change than RFC 023's
phrase "a follow-up RFC can add raw capture" implies.

### The argument against, which is stronger than it first looks

RFC 040 exists because this channel captures more than it should. Its
finding was that headers — where credentials live — were captured
without gate or cap.

**Non-JSON bodies are the other place credentials live.** A
`application/x-www-form-urlencoded` login body is
`username=alice&password=hunter2`. RFC 040 already rejected base64 raw
capture on the grounds that it takes such content verbatim while looking
opaque enough that nobody inspects it. A truncated UTF-8 snippet is more
readable and therefore *more* exposed, not less.

Name-based redaction — the mechanism RFC 040 chose for headers — does not
transfer. There are no field names to match against without parsing the
body, and parsing it means understanding every content type someone might
post. That is the value-scanning problem RFC 040 declared a non-goal, for
good reasons that have not changed.

### The argument for

Debugging a mock server against a client that posts form data or XML is
genuinely harder when the trace channel shows nothing. `body_json: None`
today means *either* "no body" *or* "body present but not JSON", and
`ParsedRequest`'s own doc comment says the two are indistinguishable.
That ambiguity is a real diagnostic gap, and it is the gap RFC 023 meant
to close eventually.

## The question

**Should the trace channel capture non-JSON request bodies?**

Three answers, in increasing cost:

1. **No.** Close RFC 023 Unresolved 1 as "decided against". Cost: none.
   The diagnostic gap remains.
2. **Presence only.** Distinguish "no body" from "non-JSON body of N
   bytes, content-type X" — without capturing content. Closes most of
   the diagnostic gap, captures no credential material, and needs only a
   small addition to `ParsedRequest` (a length and a content type, not
   the bytes).
3. **Content, redacted somehow.** The original RFC 023 idea. Requires
   solving body redaction, which is the value-scanning problem.

**Recommendation: (2).** It answers the question a developer is actually
asking — *"did my body arrive, and was it what I thought?"* — without
capturing anything that can be a credential. It is also the only one of
the three whose cost is proportionate to the gap it closes.

(3) should not be attempted unless (2) proves insufficient in practice,
and then as its own RFC with the redaction problem faced squarely rather
than assumed away.

## Non-goals

- Response bodies.
- Value-scanning heuristics of any kind (RFC 040's non-goal, restated).
- Changing JSON body capture, which works and is already gated and
  capped.

## If the answer is (2) — what it would need

Sketch only; a real design follows approval.

- `ParsedRequest` gains something like
  `body_meta: Option<BodyMeta { len: usize, content_type: Option<String> }>`,
  populated in `parsed_request_from` where the bytes still exist.
- `RequestSummary` surfaces it, distinguishing the three states: no
  body · JSON body captured · non-JSON body present, described but not
  captured.
- Every consumer of `ParsedRequest` is checked for the new field's
  effect. There should be none — it is additive — but "should be" is not
  "was verified".

**The cost that is easy to underestimate:** populating this on every
request, including when nobody is tracing. It must be cheap or gated,
and which of those is a design question rather than an obvious call.

## Risks

| Risk | Notes |
|---|---|
| (2) is a compromise that satisfies nobody | Possible. If the diagnostic gap is really about *content*, (2) does not close it and the honest outcome is (1) plus a documented limitation |
| Touching `ParsedRequest` ripples | It is additive, but it is the pipeline's central type; the check is real work, not a formality |
| Per-request cost | Named above rather than discovered later |

## Unresolved questions

1. **Which answer?** Recommendation (2); owner's call, because it is a
   product judgement about a diagnostic gap versus an exposure.
2. **If (2): is `body_meta` populated always, or only when tracing is
   active?** Always is simpler and observable; gated is cheaper. Needs
   measuring, not guessing.
3. **Does the GUI want this?** It consumes trace events, and this is the
   kind of thing it might display. Worth asking in the same conversation
   as RFC 040's Q2 and RFC 042's round-trip rather than separately.
