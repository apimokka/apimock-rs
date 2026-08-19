# RFC 054 — The v5 deprecation release

**Status.** Implemented (v5.19.0) — **the final v5 release**. Approved by
the project owner 2026-08-17; reviewed in `REVIEW-001` / `REVIEW-002`.
**Tracks.** Closing v5. The 5.x release that warns about what 6.0.0
changes, so a user or a pipeline meets the change once while the old
behaviour still works.
**Touches.** `crates/apimock/src/cmd/validate.rs`, `args/constant.rs`,
documentation, and the migration guide. **Cut from `5.18.0` on a
short-lived branch**, not from `main` (RFC 048 § 7.2).
**Depends on.** [RFC 053](./053-v6-cli-contract.md) — the envelope this
release lets people adopt early, and § 7's enumeration.

## Summary

Ship `5.19.0` from a branch off `5.18.0`, carrying deprecation warnings
for the CLI changes 6.0.0 makes — and, more usefully, carrying the *new*
output shape behind the new flag so the migration can be done and tested
before the break.

## Motivation

### Why this is writable now

RFC 048 § 7.3 concluded the deprecation release was gated on v6's CLI
surface being designed. Checking the code says otherwise, and the reason
is simple enough to state plainly.

**A deprecation warning is only needed where an *existing* invocation
changes.** v6's new commands are new subcommand tokens:

```rust
// crates/apimock/src/args.rs
if raw.get(1).map(String::as_str) == Some("match-test") { … }
if raw.get(1).map(String::as_str) == Some("validate")   { … }
```

Subcommands are matched positionally at `argv[1]` by exact string
comparison. `get` and `set` are new tokens that collide with nothing, so
they cannot change what any current invocation does. And bare `apimock`
is kept as an alias for `apimock serve` (RFC 053, Layer 1), so it does
not change either.

That leaves only the **output shapes of existing commands**, which
RFC 053 § 7 already enumerates. So this release can be written before
`get` and `set` exist — and v5 can close sooner than RFC 048 § 7.3
assumed.

### Why the branch

`main` carries breaking work — RFC 040's `TraceConfig` fields, RFC 050's
additions — landed before this enumeration existed. A deprecation
release cut from `main` would itself break, which defeats the purpose.
Owner decision, recorded at RFC 048 § 7.2.

## Goals

1. A user of `validate --json` is told, once, that its shape changes in
   6.0.0 — on **stderr**, at **exit 0**, naming the version.
2. **The new shape is available in 5.19.0**, so the migration can be
   made and tested before the break rather than after it.
3. Nothing that works in 5.18.0 stops working in 5.19.0.
4. What cannot be warned about is written down instead.

## Non-goals

- Implementing `get`, `set`, or any of RFC 053's envelope beyond what
  `validate` needs.
- Changing `validate`'s diagnostics, severities, or exit codes.
- Warning about library-side breaks. There is no mechanism (RFC 048
  § 7.3); they go to the migration guide.
- Bringing `main`'s breaking work into this release.

## Proposed design

### `--format`, alongside `--json`

`validate` gains `--format text|json`. `--json` keeps working and keeps
emitting **today's bare array**.

```
apimock validate -c ./apimock.toml --json
apimock validate: --json is deprecated and will be removed in 6.0.0.
  Use --format json, which emits the new response envelope.
```

Warning on **stderr**, exit code **unchanged**, printed **once**.

### The part that makes this a migration rather than an announcement

**`--format json` emits RFC 053's envelope, in 5.19.0.** Both shapes ship
in the same binary under different flags:

| Flag | Shape | In 6.0.0 |
|---|---|---|
| `--json` | today's bare array | removed |
| `--format json` | RFC 053's envelope | the only shape |

So a consumer can switch flags, adapt their parser, and verify it against
a real binary **before** 6.0.0 removes the old path. A deprecation
warning that only says "this will change" leaves the work until after the
break; this lets it happen before.

This is also the first real exercise of RFC 053's envelope, on the one
command that already has a machine-readable output — which is a better
place to find design problems than `get` would be.

### What 6.0.0 then does

Removes `--json`. Per RFC 048 § 7, the removal must **fail loudly** with
a machine-readable error naming the replacement, never silently do
something different.

## The release

- **Version 5.19.0**, cut from `5.18.0` on `release/5.19`.
- The branch carries this RFC's work **only** — no `main` commits.
- **Merged back to `main` afterwards**, so `--format` support exists for
  6.0.0 to build on. The `--json` path and its warning are then removed
  by 6.0.0's own work.
- `main` remains the 6.0.0 line and is not otherwise disturbed.

## Testing and verification

- `--json` emits a byte-identical array to 5.18.0's, and the warning is
  on **stderr** — proven by capturing the streams separately, since the
  whole point is that a parser reading stdout is unaffected.
- Exit code unchanged for every existing `validate` invocation.
- `--format json` emits a valid RFC 053 envelope: object, `schema`,
  `apimock`, exactly one of `result` / `error`.
- `--format text` matches today's default output.
- Both flags together is a usage error (exit 2), not a silent
  precedence rule.
- Full suite green; report the count against the branch's baseline —
  **425**, since it is cut from `5.18.0`, not `main`'s 437.

## Risks

| Risk | Mitigation |
|---|---|
| The branch diverges from `main` | It carries one RFC's work and merges back immediately. If it grows, that is a signal to stop and reconsider |
| The warning is printed per diagnostic rather than once | Explicit goal; a warning that repeats is noise a caller learns to filter |
| RFC 053's envelope changes after this ships | Then `schema` increments and 5.19.0's `--format json` was `schema: 1`. That is what the field is for — but it argues for settling RFC 053 before this ships, not after |
| `get`/`set` turn out to change an existing invocation after all | Would need a second deprecation release. The § Motivation argument says they cannot; if that reasoning is wrong, it is wrong now and worth challenging now |

## Unresolved questions

1. ~~**Does `match-test` need a row?**~~ ✅ **Resolved 2026-08-17 — no,
   and the question dissolves.** It was framed as "is its text output a
   contract, and must we therefore warn?" The better answer is to **not
   change its text output at all**: 6.0.0 *adds* `--format json` to
   `match-test` rather than reshaping what it prints today. Nothing
   breaks, so nothing needs warning, and the contract question never has
   to be answered. This release does not touch `match-test`.
2. **Is 5.19.0 the right number?** It is the next minor from 5.18.0, and
   `main`'s work becomes 6.0.0 — so 5.19.0 is never an ancestor of
   6.0.0's line except through the merge-back. Alternative is a patch
   release, but new flags are not a patch.
3. **Does the migration guide ship here or with 6.0.0?** Here, arguably:
   the library breaks it describes are already on `main` and a reader
   meeting the CLI warning will want the whole picture at once.
