# RFC 077 — Work that should not be per-request

**Status.** Proposed — awaiting owner approval.
**Tracks.** Performance. External audit 2026-09-01, P-05, P-06, P-07,
P-09.
**Touches.** `crates/apimock-server/src/response/file_response.rs`,
`dyn_route.rs`, `parsed_request.rs`, `http_method.rs`, `util/http.rs`.

## Summary

Four independent per-request costs, none structural, all small to fix:

| | Cost |
|---|---|
| **P-05** | Binary files read **twice** and copied twice |
| **P-06** | `dyn_route_content` lists the **whole directory** on every request |
| **P-07** | `HttpMethod::is_match` allocates **two `String`s** per comparison |
| **P-09** | Content-type checked 3× and URL normalisation allocates 3 `String`s, per request |

Grouped because they share a shape and a fix strategy, not because they
interact. **RFC 071 is the one that matters for scaling**; this is the
tail behind it.

## Motivation

None of these is a correctness problem and none will be visible at small
scale. They are worth doing because they are cheap, and because P-06 in
particular scales with something a user controls — the number of files
in the served directory, which the zero-config workflow actively
encourages growing.

The audit's own framing is worth keeping: this is the cluster you fix
*after* `Arc<AppState>` (RFC 071), because that one is ~20× larger than
the matching this project benchmarks and these are not.

## Goals

1. A binary file is read once and copied once.
2. Directory resolution does not enumerate the directory.
3. Method comparison and content-type checking allocate nothing per
   request.

## Non-goals

- Caching file contents in memory. That is a design with invalidation
  questions (and interacts with the absent hot-reload); not this.
- Restructuring the response pipeline.

## Design

**P-05** — read once. The current shape reads as text, and on failure
re-reads as binary (`file_response.rs:113-129`); that fallback is what
RFC 065's review established as load-bearing for content-type
detection, so keep the behaviour and lose the second read — read bytes
once, then attempt UTF-8 interpretation on what is already in hand.

**P-06** — `stat` the candidate path directly instead of listing the
directory to find it. The listing exists for index resolution; a direct
existence check covers the common case, with the listing kept only where
it is genuinely needed.

**P-07** — `eq_ignore_ascii_case` instead of allocating.

**P-09** — hoist the content-type check; reuse one normalised path.

## Testing and verification

- Behaviour is **identical** — this RFC changes no semantics. The
  existing suite is the primary check.
- A directory with many files: request latency is flat relative to a
  directory with few. **This is the assertion that would have caught
  P-06**, and none exists today.
- The benchmark suite gains a case per fix, so a regression is visible.
- Binary and text file serving, index resolution, and extension
  inference all unchanged.

## Risks

| Risk | Mitigation |
|---|---|
| The read-once change alters content-type detection | It must not. Pin current detection with tests **before** touching it |
| `stat` fast path misses an index case | Keep the listing for the cases that need it; the fast path is an addition, not a replacement |
| Micro-optimisation churn for no measured gain | Measure each. If one shows nothing, **say so and drop it** — an unmeasured optimisation is just churn |
