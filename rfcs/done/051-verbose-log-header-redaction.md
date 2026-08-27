# RFC 051 — Redact credential headers in verbose request logging

**Status.** Implemented (v6.0.0). Accepted — approved by the project owner 2026-08-17.
Implemented and merged to `main`; awaiting the 6.0.0 release.

**Tracks.** Security. The second place request headers are emitted
verbatim — and, on reachability, the more exposed of the two.
**Touches.** `crates/apimock-server/src/parsed_request.rs`
(`capture_in_log`), and whatever is needed to share
[RFC 040](./040-trace-capture-and-redaction.md)'s redaction. **No change
to matching, dispatch, or response construction.**

## Summary

RFC 040 stopped the trace channel emitting credential headers. The
console logger still does. Apply the same policy to the same data, in
the other place it leaves the process.

## Motivation

### The finding

`capture_in_log` (`crates/apimock-server/src/parsed_request.rs`), gated
by `log.verbose.header`:

```rust
.map(|(name, value)| format!("\n{}: {}", name, value.to_str().unwrap_or("<non-utf8>")))
```

Every request header, verbatim, to the console. `authorization`,
`cookie`, `x-api-key` included. Found by the dev team while implementing
RFC 040, reported rather than fixed because it is a different
configuration surface (`log.verbose`, not `TraceConfig`) and was not in
that RFC's scope. Correctly so.

### Why this is not simply "the same bug again"

It is off by default — `VerboseConfig { header: bool, body: bool }`
defaults to `false` — so it is opt-in in a way the trace channel's header
capture was not. That makes it less bad.

**On reachability it is worse.** The trace channel is library-only: what
it captures stays inside the process unless someone writes a subscriber.
This writes to the **console of a running server**, which is where CI job
logs, terminal scrollback, screen shares and pasted bug reports come
from. It leaves the machine by default once written.

And consider when it gets switched on. Nobody enables verbose header
logging idly; they enable it because a request is being rejected and they
want to see what was sent. That is *precisely* the moment an
`authorization` header is interesting — and precisely the moment its
value lands in a log someone may later paste into an issue.

### Why now rather than later

RFC 040 built the mechanism: a configurable denylist, case-insensitive
matching, marking rather than omitting. Reusing it is a small change.
Leaving the two paths inconsistent means the project has a documented
redaction policy that one emitter honours and another ignores — which is
worse than having neither, because it invites the assumption that
redaction is handled.

## Goals

1. Verbose header logging redacts by the same policy as the trace
   channel.
2. One policy, defined once, applied in both places — not two lists that
   drift.
3. Redacted headers are marked, not omitted, consistent with RFC 040.
4. `log.verbose.body` is examined for the same problem and dispositioned
   — see Unresolved 2.

## Non-goals

- Changing the default of `log.verbose.header`. It is already off; this
  RFC makes it safe when on, rather than harder to turn on.
- Response headers or bodies.
- Value-scanning heuristics. Name-based only, as RFC 040 settled.
- A new configuration surface. This should reuse RFC 040's, not invent a
  parallel one.

## Proposed design

Direction, not prescription — the implementer decides having read both
call sites.

The redaction policy currently lives on `TraceConfig`
(`crates/apimock-server/src/trace.rs`) as `header_redaction`,
`header_denylist`, `header_allowlist`, with `redact_headers` private to
it. Verbose logging is configured by `log.verbose` in `apimock-config`,
a different surface entirely.

So there is a real question of **where the shared policy should live**,
and it is the main design decision here. Options include lifting the
policy into a small type both can hold, or having the logger borrow the
tracer's configuration. What matters is goal 2: one definition, not a
copy.

Whatever is chosen, the **default with no configuration must be safe**,
as it is for the trace channel.

## Testing and verification

- With `log.verbose.header` on and no other configuration, a request
  carrying `authorization`, `cookie` and `x-api-key` produces log output
  containing none of the three values — asserted on the **rendered log
  line**, since that is what reaches a terminal.
- A non-lowercase spelling is redacted too. RFC 040's implementation
  matched case-insensitively; a second call site is a second chance to
  get that wrong.
- Redacted headers appear marked, not omitted.
- A non-credential header's value still appears — the logger must remain
  useful.
- Full suite green; report the count against the **430** baseline.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## Risks

| Risk | Mitigation |
|---|---|
| Redaction makes verbose logging useless for the auth debugging it is turned on for | Marking rather than omitting; the denylist is configurable, so a developer can opt a specific header back in deliberately |
| The policy is copied rather than shared, and the two drift | Goal 2 is explicit; a copied list should fail review |
| Sharing the policy drags `TraceConfig` into `apimock-config`, or the reverse | Establish the dependency direction from source before designing — `apimock-server` depends on `apimock-config`, not the reverse, and RFC 040 already got this wrong once |

## Unresolved questions

1. **Where does the shared policy live?** The main design question; see
   § Proposed design. Establish from source rather than assuming, and
   report the choice with its reasoning.
2. **Does `log.verbose.body` have the same problem?** Almost certainly —
   a logged request body carries form-encoded credentials as readily as
   a header does. Not assumed here: establish it, and if so say whether
   it belongs in this RFC or its own. Note that
   [RFC 050](./050-non-json-body-capture-decision.md) is deciding
   whether non-JSON bodies should be *captured* at all, and the answer
   there may bear on this.
