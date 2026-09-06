# Watch matches live

**Not currently reachable from the `apimock` CLI, or from anywhere
else outside custom Rust code.** This page documents that state
honestly rather than a workflow you can actually follow today.

## What exists

`TraceEmitter` (`crates/apimock-server/src/trace.rs`) is a
`tokio::sync::broadcast`-based channel the server emits one event to
per request, describing what actually answered it — a matched rule
(with its rule-set and rule index), a middleware response, a served
fallback file, or a genuine miss (RFC 073; before it, every event
wrongly reported the same "miss" regardless) — including whether a body
was captured (subject to a `max_body_bytes` cap). A `TraceTransport`
type can also expose the channel over a Unix-domain socket or TCP, for
an external process to subscribe to.

**Request headers are redacted before an event is built (RFC 040).**
By default, well-known credential-bearing headers — `authorization`,
`cookie`, `set-cookie`, `proxy-authorization`, `x-api-key` — are
replaced with the placeholder `[redacted]`; the header name still
appears, so a consumer can tell "redacted" from "the request never
sent this header". Matching is case-insensitive. This is a denylist by
default; an allowlist mode exists (`TraceConfig::header_redaction =
HeaderRedactionMode::Allowlist`), which redacts every header except
the ones named in `TraceConfig::header_allowlist`. Both lists are
plain `Vec<String>` fields on `TraceConfig` — configurable only at
this Rust level, for the same reason as everything else on this page:
there is no config-file or CLI surface yet.

**A request body's presence and length are always reported; its
content never is unless it was JSON and capture was on (RFC 050).**
`RequestSummary.body_len` is `Some(n)` for *any* `n`-byte body that
arrived — JSON included — so a body's presence no longer depends on
whether it happened to be captured. Together with `body_json`, that's
three distinguishable states: no body (both absent); a body present and
JSON-captured (`body_len` **and** `body_json` both present, capture
still gated by `capture_body`/`max_body_bytes` exactly as before); a
body present but not captured (`body_len` present, `body_json` absent —
because it wasn't JSON, or capture was off, or it was over the cap;
`body_truncated` distinguishes that last cause further). The field
started narrower — populated only for non-JSON bodies — until review
found that left the common case (a JSON body under `capture_body`'s
default of `false`) still indistinguishable from no body at all, which
was the exact ambiguity this RFC exists to close. Content capture is
deliberately the ceiling regardless: see RFC 050's Motivation for why a
truncated snippet was rejected, not merely deferred.

**Verbose console logging shares this same redaction policy (RFC 051,
extended by RFC 073).** `capture_in_log`
(`crates/apimock-server/src/parsed_request.rs`, gated by
`log.verbose.header`, default off) used to print every request header
verbatim to the console — the same credential values RFC 040 stopped
the trace channel from emitting, just through a different door. It now
calls `TraceConfig::is_redacted_key` — the exact function
`redact_headers` uses — so there is one definition of "which names are
credentials," not two lists that can drift.

**`log.verbose.body` is redacted too, as of RFC 073** — it used to
print a query string and a JSON body's fields verbatim, with no
redaction at all, even while header redaction (above) already existed.
The same denylist/allowlist now applies to a query parameter's value
and a JSON body's object keys (recursively, so a secret nested under a
non-secret-named parent is still caught) via
`TraceConfig::redact_query_string`/`redact_json_value` — one policy,
wherever a name-value pair can leave the process, not three separate
ones. The trace channel's own `capture_body` capture is redacted the
same way, not only the console path — a captured body reaching an
out-of-process UDS/TCP subscriber is at least as serious a leak surface
as a local terminal, so it got the same fix.

## Why you can't reach it

- **No config surface.** `apimock.toml` has no field that sets
  `TraceConfig` or a transport. The server always constructs the
  tracer with a fixed default (`capture_body: false`,
  `max_body_bytes: 8192`) — nothing in the config file changes that.
- **The socket/TCP transport is never started.** `TraceTransport`'s
  accept loop is fully implemented but is not called anywhere in this
  repository — confirmed by searching every source file. There's no
  flag or setting that turns it on.
- **The `Workspace` edit API's trace fields are a stub.** The
  GUI-facing config-editing surface has `EditValue` variants shaped
  like they'd toggle trace settings, but their handlers only log a
  message — they don't write anything back to the config. The comment
  in the source ("stored in config for persistence") doesn't match
  what the code actually does.
- **Nothing in the shipped binary subscribes to the channel either.**
  `main`/`args.rs` never call `TraceEmitter::subscribe()` or start
  `TraceTransport::accept_loop` — the running `apimock` process never
  has a subscriber, shipped-binary code included. (`subscribe()` is
  called from test code outside `trace.rs`'s own module too, as of RFC
  073's tranche — `crates/apimock/tests/server/trace.rs` — but a test
  proving the mechanism works is not the same as the CLI exposing it.)

## If you need this now

The channel itself works and is unit-tested — an embedder constructing
a `Server` directly via the `apimock-server` crate (the way
[`bench_load.rs`](https://github.com/apimokka/apimock-rs/blob/main/crates/apimock/examples/bench_load.rs)
constructs one in-process for its own purposes, though that example
doesn't touch tracing) could call `TraceEmitter::subscribe()` on it
directly. That's a from-source integration; the shipped CLI doesn't
expose a way to do this today.
