# RFC 023 — Body capture in match-trace events

**Status.** Implemented (v5.12.0)
**Tracks.** RFC 006 Unresolved §1 — "Body capture (deferred to
follow-up RFC)."
**Touches.** `apimock-server` (`trace.rs` — `RequestSummary`,
`TraceEmitter`), `apimock-config` (trace config, `RootSettingKey`),
documentation.

## Summary

Match-trace events currently capture request method, URL path, and
selected headers, but not the request body. This RFC adds an opt-in
body capture field to `RequestSummary`, gated by a config flag and
a byte-size cap to prevent accidental memory / bandwidth issues.
Only JSON bodies are captured; non-JSON bodies are omitted (the
content-type is recorded as metadata).

## Motivation

Debugging a rule that matches on `body.json.action` today requires
sending a `curl` separately and eyeballing the output. A GUI that
subscribes to the trace channel could instead show the exact request
body alongside the rule-match outcome — "here is what came in, here
is why it matched (or didn't)."

RFC 006 deferred body capture citing privacy and size concerns. Both
are addressed by the opt-in flag and the byte cap.

## Reference-level explanation

### `RequestSummary` addition

```rust
pub struct RequestSummary {
    pub method: String,
    pub url_path: String,
    pub headers: Vec<(String, String)>,
    // NEW — only present when capture_body = true and body is JSON.
    pub body_json: Option<serde_json::Value>,
}
```

### Config

A new `[trace]` config section in `apimock.toml` (separate from the
transport config):

```toml
[trace]
enabled = false
capture_body = false      # opt-in; default off
max_body_bytes = 8192     # bodies larger than this are omitted
```

New `RootSettingKey` variants:
- `TraceCaptureBody` (`EditValue::Boolean`)
- `TraceMaxBodyBytes` (`EditValue::Integer`)

Both return `SoftReload` from `ReloadHint::for_key`.

### Capture logic in `TraceEmitter::emit`

When `capture_body = true`:
1. If `body_json` on `ParsedRequest` is `Some(v)` and the serialised
   size ≤ `max_body_bytes`: attach `v` to `RequestSummary.body_json`.
2. If serialised size exceeds the cap: set `body_json = None` and
   include `"body_truncated": true` in the event's top-level metadata.
3. If body is not JSON: `body_json = None` (content-type recorded in
   `headers` is sufficient context).

When `capture_body = false` (default): `body_json` is always `None`.

### Privacy note

Documented: "Enable only in development environments. Request bodies
may contain credentials, PII, or secrets. The body is stored in
memory until the trace event is delivered; cap `max_body_bytes`
appropriately."

## Drawbacks

- Adds memory pressure proportional to `max_body_bytes × channel_capacity`
  when enabled.
- Increases trace event size, slowing subscribers on busy servers.
- Both are opt-in and bounded, so the default path is unchanged.

## Unresolved questions

1. **Non-JSON body representation.** Could capture raw bytes (base64)
   or a truncated text snippet. This RFC keeps it simple: JSON only,
   non-JSON omitted. A follow-up RFC can add raw capture.
2. **Redaction rules.** RFC 006 mentioned "redaction rules for body
   capture." Deferred: the opt-in flag + environment guidance is
   the stage-1 answer.
