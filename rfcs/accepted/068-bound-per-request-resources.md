# RFC 068 — Bound what one request can consume

**Status.** Accepted — owner approved 2026-09-01.
**Tracks.** Availability. External audit 2026-09-01, S-02 and S-03
(two of the three High findings on that axis).
**Touches.** `crates/apimock-server/src/parsed_request.rs`,
`crates/apimock-server/src/middleware/middleware_handler.rs`,
`crates/apimock-config` (two settings),
`docs/src/reference/threat-model.md`.

## Summary

Two unbounded resources, both reachable by a single request:

1. **Request bodies are buffered whole, with no limit.** The audit
   measured one 256 MiB request taking the process from 9 MiB RSS to
   **462 MiB**.
2. **Rhai middleware runs with no operation limit, synchronously on a
   tokio worker.** A script that does not terminate wedges that worker
   permanently; a few wedge the server.

Both are grouped here because they are the same shape — *external input,
unbounded, no back-stop* — and because fixing either alone leaves the
process trivially stallable.

## Motivation

### S-02 — bodies

`parsed_request.rs:40-46` collects the body without a cap. There is no
connection limit either, so concurrency does not bound it. Read-only,
no auth needed, and the process is a development tool likely running on
the developer's own laptop alongside their editor and browser.

### S-03 — middleware

`Engine::new()` (`middleware_handler.rs:63`) sets no limits — not
`set_max_operations`, not `set_max_call_levels`, not the string/array
size caps. Evaluation at `:129` is a direct synchronous call on the
async runtime.

**No attacker is required.** A `while true` in a script an operator is
actively developing is the ordinary case, and today it does not fail —
it silently removes a worker thread from the runtime. That is worse than
a crash, which would at least be visible.

**It is also inconsistent with the crate's own discipline**: file reads
already go through `spawn_blocking` precisely so they cannot block the
runtime. Script evaluation is the more dangerous of the two and does not.

`threat-model.md` says a failing script "cannot crash the process, but
it can silently degrade routing" (D-05). Correct about crashing;
non-termination is not degraded routing, it is a stalled server.

## Goals

1. A request body larger than a configured limit is refused with **413**,
   not buffered.
2. A non-terminating script fails that request, and leaves the server
   serving.
3. Both limits configurable, with defaults that are generous for
   development and finite.
4. The threat model says what the limits are and what happens at them.

## Non-goals

- Streaming request bodies. Matching needs the whole body; a limit is
  the right answer, not incremental parsing.
- Sandboxing Rhai's *capabilities*. Middleware executing operator-authored
  code is a documented, deliberate allowance. This is about termination
  and thread ownership, not privilege.
- Connection-count limits. Worth considering separately; not needed to
  close either finding.

## Design

### Bodies

Wrap the incoming body in `http_body_util::Limited` at the single point
it is collected. On overflow, return **413 Payload Too Large** —
`bad_request_response`'s sibling, and note that `bad_request_response`
already exists and is never called (F-09), so the error path largely
exists.

New setting `[service].max_request_body_bytes`. **Recommend a default of
32 MiB**: far above any realistic mock request, far below the 462 MiB
the audit reached.

### Middleware

Two changes, both required; either alone is insufficient:

1. **`engine.set_max_operations(...)`** plus the call-depth and
   string/array caps. Bounds a runaway script by work done.
2. **Move `eval_ast_with_scope` into `spawn_blocking`.** Bounds the
   damage of anything the operation limit does not catch, and matches
   what file I/O already does. Rhai's `sync` feature is already enabled,
   so the engine is `Send`.

New setting `[service].middleware_max_operations`, defaulting to a value
generous enough that no reasonable script hits it.

> **Do not implement only the operation limit.** It is the easier half
> and it is the one that looks sufficient. An operation limit does not
> bound a script blocked on something that is not an operation, and it
> is a value an operator can raise. `spawn_blocking` is what makes the
> failure mode "one slow request" instead of "one fewer worker forever".

## Testing and verification

- A body one byte over the limit → **413**, and RSS does not grow by the
  body's size. **Assert on memory, not just the status** — the status
  can be right while the body was still buffered first.
- A body at the limit succeeds.
- A `while true` script: the request fails, **and the server answers a
  subsequent request on a different connection**. That second assertion
  is the finding; the first alone would pass with the defect present.
- Concurrency: N such scripts running simultaneously must not reduce
  throughput to zero.
- Existing middleware tests unchanged.

## Risks

| Risk | Mitigation |
|---|---|
| A legitimate large upload starts failing | Configurable, and 32 MiB is far above realistic mock traffic. Named in the release note |
| An operation limit trips a heavy but valid script | Configurable, and the default is set generously. The limit exists to bound pathology, not to police scripts |
| `spawn_blocking` changes middleware timing | It does — evaluation moves off the async worker. That is the point, and middleware is not on a latency-critical path in a mock server |

## Unresolved questions

1. **Should the body limit apply to responses read from disk?** A large
   `respond.file_path` is operator-authored, so it is not the same
   threat — but it has the same memory shape, and the audit notes
   (P-05) that response files are fully buffered too. **Recommend
   leaving it out of scope here** and treating it as a performance item,
   since conflating an operator-authored size with a client-controlled
   one would muddle both.
