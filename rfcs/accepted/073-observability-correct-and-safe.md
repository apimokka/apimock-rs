# RFC 073 — Observability: correct events, honest limits, no leaks

**Status.** Accepted — owner approved 2026-09-01.
**Tracks.** Correctness / privacy. External audit 2026-09-01, F-08,
S-05, S-06, D-02.
**Touches.** `crates/apimock-server/src/server.rs`,
`crates/apimock-server/src/trace.rs`, verbose logging,
`docs/src/reference/` and the trace module docs.

## Summary

The live match feed reports **every match as a miss**, its documented
back-pressure behaviour does not exist, its transport has no access
control, and verbose logging prints request bodies and query strings
**unredacted** while header redaction is careful.

Grouped because they are one subsystem and one question: *does what
apimock tells you about a request reflect what happened, and does
telling you cost more than you agreed to?*

## Motivation

### F-08 — every event is wrong

`server.rs:409-414` emits `Miss { status: 0 }` for every request,
including matches. The correct rule index is computed **on the adjacent
line** and discarded.

Nothing is emitted at all for middleware responses, fallback-directory
responses, or 404s.

So the feature RFC 006 and RFC 009 built — watch which rule answered —
reports wrong data for every event it emits, and stays silent for most
of the paths a user would be debugging.

### S-06 / D-02 — documented back-pressure that does not exist

`trace.rs:26-34` documents behaviour tokio's `broadcast` channel does
not have:

> *"When the channel is full, `emit` drops the event and increments an
> internal counter"* — `send` fails only when there are **no receivers**.
> *"the gap is reported in the next JSON line via `dropped_count`"* —
> `Lagged(n)` is discarded at `:532-537` and never reaches the counter.

Both false as written. A user reasoning about whether they have seen
every event is reasoning from a description of a mechanism that is not
there.

**And the transport has no access control.** Anything that can reach the
socket receives the feed — which carries request data.

### S-05 — redaction that stops short

Header redaction is genuinely good (RFC 051 built it deliberately, with
allowlist/denylist and a redaction mode). Verbose body logging prints
**bodies and query strings unredacted**.

That is the more sensitive half. A bearer token in a header is redacted;
the same token in `?access_token=` is printed.

## Goals

1. Trace events state what actually happened, for every response path.
2. The documented back-pressure matches the implementation — by fixing
   one or the other, deliberately.
3. The trace transport's access model is stated, and restricted where it
   can be.
4. Redaction covers bodies and query strings, or the docs say plainly
   that it does not.

## Non-goals

- Redesigning the trace protocol or its schema.
- Making trace delivery reliable. Lossy-with-honest-reporting is a fine
  design; silently-lossy-with-wrong-docs is not.

## Design

**F-08** — emit `Outcome::Matched` with the indices already in hand, and
add emit sites for the middleware, fallback and not-found paths.

**S-06 / D-02** — pick one:
- implement the documented counter (track `Lagged(n)` into
  `dropped_count`), or
- correct the documentation to describe `broadcast`'s actual semantics.

**Recommend implementing it.** The documented behaviour is the more
useful one, a consumer cannot otherwise tell a quiet server from a
dropped feed, and `dropped_count` already exists as a field.

**Transport** — a Unix socket should be created with restrictive
permissions. For the TCP transport, state in the docs that it has no
authentication and should be bound to loopback. This is a threat-model
entry as much as a code change.

**S-05** — apply the existing header-redaction machinery to query
strings and bodies. If full body redaction is impractical, a size cap
plus an opt-in is better than the current silent full print, **and the
docs must say which was chosen**.

## Testing and verification

- A matched request emits `Matched` with the **correct rule-set and rule
  index** — assert the index, not just the variant.
- Middleware, fallback and 404 paths each emit.
- With a slow consumer, dropped events are reported (or the docs say
  they are not, and a test pins the documented behaviour).
- A request with `?token=secret` and a secret in the body: neither
  appears in verbose output under default settings.
- Header redaction unchanged — its existing tests must still pass.

## Risks

| Risk | Mitigation |
|---|---|
| Redacting bodies makes debugging harder | It is the user's data and their choice; opt-in with a clear setting, defaulting to redacted |
| Emitting more events changes feed volume | It is currently emitting *wrong* events; more and correct is the point. Note it in the release |
| Unix-socket permissions differ per platform | State the platform behaviour rather than assuming; Windows has no equivalent and the docs should say so |

## Unresolved questions

1. **Is the TCP trace transport worth keeping?** It cannot be
   access-controlled without authentication this project has no reason
   to build. If Unix sockets cover the real use case, deprecating TCP is
   simpler than documenting a permanent caveat. **Establish who uses
   it** before deciding.
