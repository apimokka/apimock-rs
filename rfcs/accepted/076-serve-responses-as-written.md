# RFC 076 — Serve JSON as it was written

**Status.** Accepted — owner approved 2026-09-01.
**Tracks.** Correctness / performance. External audit 2026-09-01, F-04
and P-04.
**Touches.** `crates/apimock-server/src/response/json_response.rs`,
`docs/src/reference/`.

## Summary

A `.json` response file is parsed and re-serialised on every request.
The bytes a client receives are **minified**, and object keys are
**reordered alphabetically**, relative to the file on disk.

One change fixes both a correctness problem and a per-request cost.

## Motivation

`json_response.rs:41-43` does `json5::from_str::<Value>` then
`.to_string()`. Without `serde_json`'s `preserve_order`, `Value`'s map
is ordered, so keys come back sorted; `to_string()` minifies.

**Why this matters beyond aesthetics.** The project's stated use case
includes *stable API responses for UI testing*. Golden-file and snapshot
tests compare response bytes. A user who writes a fixture, snapshots the
response, and later reformats the fixture — or whose key order simply is
not alphabetical — gets a diff they did not cause and cannot explain,
because nothing documents that the bytes are rewritten.

It also contradicts the zero-config promise at its most literal: *the
JSON you put on disk is what a client gets back*.

**P-04 is the same line.** Every request re-parses and re-serialises a
file that was already valid. Serving the bytes removes the work
entirely.

## Goals

1. A `.json` file is served **byte-for-byte** as written.
2. An invalid `.json` file still fails, and fails at load
   (RFC 065 already made this a load-time error) rather than at request
   time.
3. The inline `respond.json` case keeps key order too.

## Non-goals

- Changing `.json5` handling. JSON5 is not JSON; converting it is the
  point, and a user writing JSON5 has already accepted a transformation.
- CSV conversion (`csv_records_key`), which is inherently a
  transformation.

## Design

**For `file_path`:** read the bytes and serve them, with
`content-type: application/json`. RFC 065 already validates the file at
load, so parsing at request time buys nothing.

**For inline `respond.json`:** enable `serde_json/preserve_order` so key
order survives. This is a workspace dependency-feature change and
affects every `Value` in the workspace — including the RFC 053 CLI
envelope, whose field order would become insertion order rather than
alphabetical.

> **That envelope-order change is user-visible and must be decided, not
> absorbed.** The dev team documented the current alphabetical order
> during RFC 064 as `serde_json`'s default. Consumers may have adapted.
> Either accept the change and note it, or scope `preserve_order` so it
> does not reach the envelope.

## Testing and verification

- A `.json` file with non-alphabetical keys and pretty formatting is
  served **byte-identical** — compare bytes, not parsed equality, or the
  test cannot fail.
- An invalid `.json` file still fails to load (RFC 065's behaviour
  unchanged).
- Inline `respond.json` preserves key order.
- `--format json` envelope: whatever is decided above, pinned by a test.
- The `serve-json-files-from-a-folder` example's tests still pass, or
  their expectations are updated **because the bytes are now correct** —
  say which.

## Risks

| Risk | Mitigation |
|---|---|
| A consumer depends on minified output | They depend on a transformation nobody documented. Release note |
| `preserve_order` changes the CLI envelope | The decision above — do not let it happen implicitly |
| Serving bytes skips a validity check | RFC 065 moved that to load time; this RFC depends on that and should say so if it ever changes |
