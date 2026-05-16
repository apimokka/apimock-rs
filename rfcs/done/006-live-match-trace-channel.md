# RFC 006 — Live match-trace channel from server to GUI

**Status.** Implemented (v5.8.0) — in-process channel fully functional; socket transport stub deferred
**Tracks.** Stage-2 / stage-3 GUI observability — emitting structured
runtime events from the server so the GUI can show "what just
matched" without users curling and tailing logs.
**Touches.** `apimock-server` (new event emission path),
`apimock-routing` (event payload shape — read-only views of the rule
that matched), `apimock-config` (toggling the channel on/off),
documentation, GUI architect brief §12 (resolves gap (f)).

## Summary

Today the only way to see whether a request matched a given rule is to
inspect server logs, often line-buffered and unstructured. This RFC
proposes a structured trace channel from the server to subscribers
(typically the GUI in subprocess mode), emitting one event per
incoming request with: the request shape, the rule that matched (if
any), the respond chosen, and timing. The channel is opt-in,
out-of-band from the HTTP listening port, and has explicit retention
and back-pressure semantics.

This is the largest RFC in the stage-2 set — it adds a new
cross-process surface — and the most architectural in flavour.

## Motivation

When a mock server doesn't behave as expected, the debugging loop
today is:

1. User reads TOML, forms a hypothesis.
2. User issues a `curl` against the server.
3. User reads server log to see whether the rule matched.
4. User re-reads TOML, refines hypothesis.

Steps 2–3 are awkward: the log format isn't structured, the rule
identifier in the log doesn't easily round-trip back to the GUI's
view, and timing information is scattered. With a structured
match-trace channel, the GUI can show a live "request log" panel:

```
12:04:31  GET  /api/v1/users           → rule 4f2…/c81 (started_with /api)  200  12ms
12:04:33  POST /api/v1/orders/123      → rule 7a9…/e02 (equal /api/v1/orders/{id})  201  8ms
12:04:35  GET  /api/v2/health          → (no match — fallback)  404  2ms
```

This unlocks "click the request to highlight the rule that matched"
UX and makes mock-server behaviour debuggable interactively.

## Guide-level explanation

The server gains a new feature: an event stream. When enabled, each
incoming request produces one event after the response is sent:

```json
{
  "event_id": "01HQA…",
  "received_at": "2026-05-15T12:04:31.123Z",
  "duration_ms": 12,
  "request": {
    "method": "GET",
    "url_path": "/api/v1/users",
    "headers": [["accept", "*/*"], ["user-agent", "curl/8.4.0"]]
  },
  "outcome": {
    "type": "matched",
    "rule_id": "4f2…",
    "rule_set_id": "c81…",
    "respond": { "type": "file", "path": "fixtures/users.json", "status": 200 }
  }
}
```

For unmatched requests, `outcome.type` is `"fallback"` or `"miss"`
depending on whether the fallback respond dir produced a response.

The transport is configurable; the recommended default is a
Unix-domain socket (or named pipe on Windows) so the GUI subprocess
can subscribe locally.

## Reference-level explanation

### Architecture

```
┌──────────────────┐   HTTP   ┌──────────────────┐
│  HTTP client     │ ───────▶ │  apimock-server  │
└──────────────────┘          │   (process A)    │
                              │                  │
                              │  match decision  │
                              │       │          │
                              │       ▼          │
                              │  trace emitter   │
                              │       │          │
                              └───────┼──────────┘
                                      │ event channel (UDS / pipe)
                                      ▼
                              ┌──────────────────┐
                              │  GUI (process B) │
                              │  subscribes,     │
                              │  renders live    │
                              └──────────────────┘
```

The trace emitter is **non-blocking** with respect to the HTTP
request: events are pushed to a bounded in-process queue, and the
emitter task drains the queue and writes to the transport. If the
queue fills (subscriber too slow), the oldest events are dropped
and a `dropped_count` field appears on the next emitted event.

### Event schema

```rust
pub struct MatchTraceEvent {
    pub event_id: Ulid,
    pub received_at: chrono::DateTime<Utc>,
    pub duration_ms: u32,
    pub request: RequestSummary,
    pub outcome: Outcome,
    pub dropped_count: Option<u32>,    // events lost since last delivery
}

pub struct RequestSummary {
    pub method: String,
    pub url_path: String,
    pub headers: Vec<(String, String)>,
    // body NOT included — see "Drawbacks"
}

pub enum Outcome {
    Matched { rule_id: NodeId, rule_set_id: NodeId, respond: RespondSummary },
    Fallback { file_path: String, status: u16 },
    Miss     { status: u16 },
    Error    { kind: String, message: String },
}
```

Event encoding: newline-delimited JSON over the transport. One event
per line. The shape is versioned via a top-level `schema_version`
field omitted from the example above for readability; first version
is `1`.

### Transport

| Option              | Pros                              | Cons                                  |
|---------------------|-----------------------------------|---------------------------------------|
| UDS / Named pipe    | Local-only, simple, secure        | OS-specific code paths               |
| TCP loopback        | Universally portable               | Authentication considerations         |
| Unix signals + file | Trivial                            | Doesn't scale beyond toy cases        |
| HTTP SSE            | Browser-friendly                   | Couples to HTTP stack again           |

The recommended default is UDS on Unix, named pipe on Windows. TCP
loopback as a configurable fallback. HTTP SSE is the most ambitious
option but is left out of scope because it brings the routing layer's
HTTP machinery into the GUI control plane, contradicting the layered
design.

### Configuration

A new `[trace]` section in the workspace config:

```toml
[trace]
enabled = false                    # default: off
transport = "uds"                  # "uds" | "named_pipe" | "tcp"
path = "/tmp/apimock-trace.sock"   # for uds / named_pipe
addr = "127.0.0.1:0"               # for tcp; port 0 = ephemeral
queue_size = 1024
drop_policy = "drop_oldest"        # "drop_oldest" | "drop_newest" | "block"
```

`UpdateRootSetting` gains `Trace*` variants per RFC 003's pattern.

### Subscriber API

The GUI subscribes by opening the configured transport. The server
maintains a list of active subscribers and fan-outs each event.
Subscribers are tracked by connection; on disconnect their slot is
freed.

Number of subscribers: capped at a small constant (e.g. 4). Mock
servers don't typically need a fleet of observers, and the cap keeps
fan-out cost predictable.

### Reload semantics

`Trace.enabled` toggles can take effect via `SoftReload` — the
emitter task starts/stops without rebinding the HTTP listener.
`Trace.transport` and `Trace.path` changes require `HardRestart`
(re-binding the trace channel). This follows RFC 003's pattern.

### Self-trace

The trace channel is **not** instrumented for its own events. Edits
made through the workspace API are observed via `ApplyResult` /
`SaveResult`; the trace channel concerns runtime HTTP traffic only.

## Drawbacks

1. **New cross-process surface.** Introduces a new IPC path with its
   own versioning, error handling, and security considerations.
   Substantially raises the implementation and maintenance bar of
   apimock-server.
2. **Body capture decision is hard.** Including request bodies in
   trace events would be useful for debugging but raises privacy
   concerns (secrets in headers/bodies), payload size concerns
   (megabyte uploads), and PII concerns. This RFC excludes bodies
   from v1; a follow-up RFC can add opt-in body capture with
   redaction rules.
3. **Resource usage.** With `enabled = false`, cost is essentially
   zero. With `enabled = true`, every request pays a queue-push cost.
   At very high request rates, the queue may become the bottleneck.
   Benchmarking required before declaring stable.
4. **Adds a soft dependency on a local socket / pipe.** Some
   container environments restrict UDS or named pipes; users in those
   environments must fall back to TCP loopback or disable the channel.

## Rationale and alternatives

**Alternative A: structured stdout JSON logs only.** Cheapest path —
emit JSON events to stdout, let the GUI subprocess tail. Loses
ability to have multiple subscribers; ties trace timing to log
flushing; doesn't scale to remote subscribers (future).

**Alternative B (this RFC): explicit trace channel via local IPC.**
Best of both: structured, multiplexed, separately configurable.

**Alternative C: built-in HTTP endpoint that serves a WebSocket /
SSE stream of events.** Maximally portable (any browser-based GUI
can consume) but couples to the HTTP stack and adds attack surface
(must authenticate, must defend against subscriber DoS).

**Alternative D: write events to a file, GUI tails the file.**
Simple, but file rotation is its own problem; rotation under live
load is error-prone.

We pick B. A is the "lite" variant worth shipping first if B proves
too ambitious for one release. C is the right answer if the GUI
becomes a remote / web client (out of scope today).

## Prior art

- Mountebank exposes a `/imposters/<port>/requests` endpoint that
  records all incoming requests; it's HTTP-pull rather than
  push-streaming but the data shape is similar.
- Charles Proxy and Fiddler use long-lived IPC channels between
  their capture engines and UI. Mature reference implementations.
- WireMock stores requests in memory and serves them via REST API.
  Halfway between Mountebank and a push channel.

## Unresolved questions

1. **Body capture (deferred to follow-up RFC).** Privacy and size
   make this nontrivial. v1 omits.
2. **Authentication on TCP transport.** If TCP loopback is used, who
   can connect? Probably "localhost is trusted" is acceptable for a
   mock server; explicit warning in docs.
3. **Schema versioning policy.** v1 ships with `schema_version: 1`;
   when do we bump? Probably any field rename or semantic change
   bumps the version; field additions are backwards-compatible and
   keep the version.
4. **Retention buffer.** Should the server keep the last N events in
   memory so a subscriber that connects late can backfill? Useful
   but adds memory pressure. Probably defer to a follow-up.
5. **Test plan.** Trace correctness is hard to assert from
   functional tests alone. Integration tests that boot the server,
   subscribe to the channel, issue requests, and verify the events
   are the minimum. A simulated "slow subscriber" test for the
   drop-policy behaviour is necessary.

## Future possibilities

- Body capture (with redaction rules) as a follow-up RFC.
- A second event class: "config change observed" — emits an event
  whenever `apply()` results in a routing-table change. Useful for
  GUI synchronisation but blurs the trace channel's purpose; better
  as a separate channel.
- Backfill / replay: serve the last N events to late subscribers.
- Remote subscriptions via authenticated TCP / TLS — a deliberate
  stage-3 feature.
- "Filtered trace": subscribers register an interest filter
  (e.g. only events matching a given rule_id) to reduce traffic at
  scale.
