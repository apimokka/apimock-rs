# RFC 009 — Trace channel socket transport and integration tests

**Status.** Implemented (v5.9.0) — UDS + TCP transport implemented; UDS cleanup on Windows deferred
**Tracks.** RFC 006 completion — implementing the out-of-process
transport layer that was explicitly stubbed in v5.8.0, and adding
the integration-test coverage that RFC 006's Unresolved §5 flagged
as mandatory before the channel can be declared stable.
**Touches.** `apimock-server` (`trace.rs` — transport implementation,
`server.rs` — emit call sites), `apimock-config` (new `[trace]`
TOML section), `crates/apimock/tests/` (integration tests).

## Summary

RFC 006 shipped the in-process `TraceEmitter` broadcast channel in
v5.8.0 but deliberately stubbed `TraceTransport::accept_loop` with
`unimplemented!()`. This RFC completes the transport layer: a
Unix-domain socket (UDS) on Unix/macOS and TCP loopback as a
portable fallback, plus the integration tests that prove end-to-end
event delivery from request receipt to subscriber read.

## Motivation

The stub is acceptable for an in-process GUI (same Tokio runtime),
but the RFC 006 design explicitly targets an **out-of-process GUI**
subscribing over a local socket. Until the transport exists:

- GUI authors cannot consume trace events.
- The `TraceTransport` type is dead code that `cargo check` does not
  exercise; regressions are invisible.
- RFC 006 Unresolved §5 calls out the lack of a slow-subscriber
  test and a real integration test as known quality gaps.

## Guide-level explanation

After this RFC, a GUI subscriber connects to the configured socket
and reads newline-delimited JSON events:

```
# server config
[trace]
enabled = true
transport = "uds"
path = "/tmp/apimock-trace.sock"
queue_size = 1024
drop_policy = "drop_oldest"
```

```sh
# subscriber (e.g. shell script)
nc -U /tmp/apimock-trace.sock
```

Each line is one `MatchTraceEvent` serialised as JSON:

```json
{"event_id":0,"received_at_ms":1747400000123,"duration_ms":4,
 "request":{"method":"GET","url_path":"/api/users","headers":[]},
 "outcome":{"type":"matched","rule_set_index":0,"rule_index":1},
 "dropped_count":0,"schema_version":1}
```

## Reference-level explanation

### Transport variants

```
[trace]
transport = "uds"              # Unix only; default when available
path = "/tmp/apimock.sock"     # UDS path

transport = "tcp"              # portable fallback
addr = "127.0.0.1:0"          # port 0 = OS-assigned ephemeral

transport = "disabled"         # no out-of-process forwarding (default)
```

`TraceTransportConfig` (already defined in v5.8.0) drives the
selection. The server logs the bound address at startup so users
can find the ephemeral TCP port.

### accept_loop implementation

```rust
pub async fn accept_loop(
    config: TraceTransportConfig,
    emitter: TraceEmitter,
) {
    match config {
        TraceTransportConfig::Uds { path } => {
            uds_accept_loop(path, emitter).await
        }
        TraceTransportConfig::Tcp { addr } => {
            tcp_accept_loop(addr, emitter).await
        }
        TraceTransportConfig::Disabled => {
            // no-op; future: return immediately
        }
    }
}
```

Each connection gets its own `broadcast::Receiver` clone. The
writer task drains the receiver and writes JSON lines. On
`RecvError::Lagged`, the gap is logged but the connection stays
open — the `dropped_count` field on the next event covers it.

Subscriber cap: **4 concurrent connections**. A fifth connection
attempt is immediately closed with a `{"error":"max_subscribers_reached"}` line.

### JSON serialisation

`MatchTraceEvent` gains `#[derive(Serialize)]`. The `Outcome` enum
uses `#[serde(tag = "type", rename_all = "snake_case")]`:

```json
{"type":"matched","rule_set_index":0,"rule_index":1}
{"type":"fallback","file_path":"users.json","status":200}
{"type":"miss","status":404}
{"type":"error","kind":"io","message":"..."}
```

A top-level `schema_version: 1` field is always emitted. Future
breaking changes bump this number.

### Config integration (`apimock-config`)

New TOML section added to the `Config` model:

```rust
pub struct TraceConfig {
    pub enabled: bool,
    pub transport: TraceTransportKind,  // "uds" | "tcp" | "disabled"
    pub path: Option<String>,           // UDS path
    pub addr: Option<String>,           // TCP addr
    pub queue_size: usize,              // default: 1024
    pub drop_policy: DropPolicy,        // "drop_oldest" | "drop_newest"
}
```

`RootSettingKey` gains `TraceEnabled` / `TracePath` / `TraceAddr`
variants, all returning `SoftReload` (the emitter task can be
toggled without rebinding the HTTP listener).

### emit call sites (`apimock-server`)

`rule_set_response` and `dyn_route_response` in `server.rs` gain
a `TraceEmitter` parameter. Each returns the outcome alongside the
response so the caller can emit one event after the response is
sent (to include `duration_ms`).

The emit is **fire-and-forget**: if the channel is full the event
is dropped without blocking the HTTP response path.

### Integration tests

New test module `crates/apimock/tests/server/trace.rs`:

1. **`trace_matched_event_delivered`** — Start server with UDS
   trace enabled. Issue a request that matches a known rule. Read
   one JSON line from the socket. Assert `outcome.type == "matched"`,
   `rule_set_index`, `rule_index`, `request.method`, `request.url_path`.

2. **`trace_miss_event_on_no_match`** — Same setup; request URL
   that has no rule. Assert `outcome.type == "miss"`.

3. **`trace_drop_policy_slow_subscriber`** — Fill the queue by
   issuing `queue_size + 10` requests without reading. Then read
   events and assert `dropped_count > 0` on at least one.

4. **`trace_disabled_produces_no_events`** — Server with
   `trace.enabled = false`. Issue requests. Assert socket path
   does not exist / TCP port is not bound.

5. **`trace_max_subscribers_rejected`** — Open 5 connections.
   Assert the 5th receives the `max_subscribers_reached` error
   JSON and is then closed.

## Drawbacks

1. **UDS is Unix-only.** Windows users must use TCP loopback.
   The conditional compilation (`#[cfg(unix)]`) adds complexity.
2. **Serialising every event is CPU cost.** `serde_json::to_string`
   per request. Benchmarks should confirm this is negligible at
   typical mock-server throughput (< 1k req/s).
3. **Socket lifecycle.** Stale UDS paths from a previous run
   prevent the server from binding. The server must unlink the
   path at startup if it exists and is a socket.

## Rationale and alternatives

**Alternative A: HTTP SSE endpoint.** Browser-friendly but
re-entangles the trace plane with the HTTP serving plane.
Rejected per RFC 006 reasoning.

**Alternative B (this RFC): UDS + TCP loopback.** Matches the
transport contract RFC 006 specified; reuses the existing
`TraceTransportConfig` shape.

## Unresolved questions

1. **UDS path cleanup on Windows.** Named pipes on Windows behave
   differently from POSIX sockets; the implementation may use
   `tokio::net::windows::named_pipe` behind `#[cfg(windows)]`.
2. **Backfill on late connect.** RFC 006 deferred a retention
   buffer. Still deferred here — a late subscriber sees only
   events emitted after connection.
3. **`TraceConfig` placement.** Does it live at the root
   `apimock.toml` level or under `[service]`? Root level preferred
   (matches `[listener]`, `[log]` precedent).
