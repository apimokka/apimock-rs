# RFC 053 — The v6 CLI contract

**Status.** **Accepted** — approved by the project owner 2026-08-17.
First realised by [RFC 054](../done/054-deprecation-release.md) in 5.19.0 and
extended by [RFC 055](./055-get-command.md); it stays in `accepted/` until
`set` ([RFC 057](../proposed/057-set-command.md)) completes the surface it
specifies.

**Tracks.** v6 product direction. [RFC 048](./048-v6-cli-interface-concept.md)
§ 11 item 3 — the contract that `get` and `set` are then built against.
**Touches.** `crates/apimock/src/args.rs` and `cmd/`, documentation.
Specifies shape; implements no command.
**Blocks.** RFC 048 § 11 items 4 (`get`) and 5 (`set`). Also the
**deprecation release**, since it names the CLI changes that release must
warn about (RFC 048 § 7.3).

## Summary

Specify the four layers RFC 048 § 3 named — invocation, data, errors,
workflow — so that `get` and `set` are built against a contract rather
than inventing one each.

The measure of success is U2, the AI CLI agent: it must be able to tell
what happened without reading prose, and must never mistake a failure for
a success.

## Motivation

### What already exists, and is worth keeping

RFC 049 settled two of the four layers for the *existing* commands:

- **Exit codes** — `0` success, `2` usage error, `1` everything else.
- **Stream discipline** — `--version` / `--help` on stdout, every
  diagnostic on stderr.

Those were deliberately set as project-wide conventions for v6 to
inherit, and this RFC inherits them rather than reopening them.

### The precedent that must not be repeated

`apimock validate --json` is the only machine-readable output this
project has today. It emits a **bare JSON array**
(`crates/apimock/src/cmd/validate.rs`):

```rust
let items: Vec<serde_json::Value> = report.diagnostics.iter().map(|d| json!({
    "severity": …, "message": …, "node_id": …, "file": …,
})).collect();
println!("{}", serde_json::to_string_pretty(&items)…);
```

No envelope, no version, no room for anything that is not a diagnostic.
**A top-level array cannot gain a field.** Adding a summary, a schema
version, or provenance means changing the top-level type — which breaks
every parser at once, with no way to warn them.

That is the same failure as R-09 in a different medium: a shape that
asserts it is final, and is not. The remedy is the same in spirit —
build in room to grow, once, deliberately.

## Goals

1. Every machine-readable response has an **envelope** that can gain
   fields without breaking parsers.
2. Schema evolution has a **stated rule** that both sides can rely on.
3. Errors are **categorised** so U2 can branch without parsing prose.
4. Exit codes stay as RFC 049 set them, and stay a *coarse* signal.
5. `get` responses state **which configuration answered them**
   (RFC 048 § 4's drift requirement).
6. The CLI changes this implies are **enumerated**, because the
   deprecation release depends on that list.

## Non-goals

- Designing `get` or `set` themselves. This is the contract they use.
- The `set` workflow layer — transaction boundaries, preview semantics,
  behaviour under external change. Named in § 6 so the envelope can
  accommodate it; specified in RFC 048 § 11 item 5.
- MCP. It is an adapter over this (RFC 048 § 1, U5).
- Changing exit codes or stream discipline.

## Layer 1 — Invocation

### Bare `apimock` keeps working, and gains an explicit form

`apimock serve [flags]` becomes the explicit spelling. **Bare `apimock`
remains an alias for it.**

RFC 048 § 7 permits breaking invocations at a major, so requiring
`apimock serve` is *available*. It is not worth taking. Bare `apimock` is
the most-used invocation in every document, example and CI script this
project has, and breaking it buys tidiness alone.

It also reduces what the deprecation release must carry — which matters
now that RFC 048 § 7.3 has established the warning mechanism is narrow.
Every invocation we *don't* break is one the migration guide does not
have to explain.

### `--format`

One flag on every command producing machine-readable output:

```
--format text   (default)
--format json   (the envelope of § 2)
```

Default **text**, because U1 at a terminal is still a user and
`apimock get /health` should print something readable. Agents pass
`--format json` always, and the documentation says so in those words.

Not separate commands, not a `--json` boolean. `--json` is what
`validate` has today and it cannot express a third format; `--format`
can.

## Layer 2 — Data, and how it evolves

### The envelope

Every `--format json` response is a JSON **object**:

```json
{
  "schema": 1,
  "apimock": "6.0.0",
  "result": { … }
}
```

or, on failure:

```json
{
  "schema": 1,
  "apimock": "6.0.0",
  "error": { "kind": "config_invalid", "message": "…", "detail": { … } }
}
```

- **Object, never a bare array.** A collection goes *inside* `result`.
- **`schema`** — an integer, so a consumer can branch.
- **`apimock`** — the binary's version. Free provenance in every bug
  report, and the first thing anyone asks.
- **Exactly one of `result` or `error`.** A consumer checks which key is
  present, not an exit code it may not have captured.

### The evolution rule, stated once

> **Producers** may add fields at any time. They may not remove a field,
> change its type, or change its meaning without incrementing `schema`.
>
> **Consumers** must ignore fields they do not recognise.

This is the JSON analogue of `#[non_exhaustive]`, and it is here for the
same reason R-09 exists: without a stated rule, the first additive change
becomes an accidental break.

`schema` starts at `1` for v6 and is expected to stay there. Incrementing
it is a breaking change and gets the same treatment as any other.

## Layer 3 — Errors

### Categories

`error.kind` is a stable, lowercase, snake_case string from a closed set:

| `kind` | Meaning | Exit |
|---|---|---|
| `usage` | Bad invocation — unknown option, missing value | 2 |
| `config_invalid` | Configuration read but not valid | 1 |
| `config_unreadable` | Configuration missing or unreadable | 1 |
| `io` | Filesystem failure that is not the config | 1 |
| `conflict` | State changed underneath — `set` only, see § 6 | 1 |
| `internal` | A bug in apimock | 1 |

New kinds may be added — that is an additive change under § 2's rule, and
consumers must treat an unrecognised `kind` as a generic failure rather
than crashing.

### "No rule matched" is a result, not an error

RFC 048 § 12 left this open. **It is a successful answer**, exit `0`,
carried in `result`.

Three reasons. It is a legitimate answer to a legitimate question — *what
does this path return?* — and "nothing" is an answer. A CI job asserting
that a path is deliberately unmatched wants exit `0`. And W3's "why did
nothing match" output is most valuable in exactly this case, where
signalling an error would suggest the command failed to run.

### Exit codes stay coarse, deliberately

RFC 049's three codes are unchanged, and **`error.kind` is not encoded
into them**. An exit code is a one-byte channel that cannot carry
structure and cannot evolve; the taxonomy lives in the payload where it
can. A caller that only has the exit code learns *whether* it failed,
which is all an exit code should promise.

## Layer 4 — Provenance

Every `get` response carries which configuration produced it:

```json
"source": { "config": "/abs/path/apimock.toml", "rule_sets": ["…"] }
```

RFC 048 § 4 requires this: a static answer can disagree with a running
server started from a different config, and an agent told X while the
server does Y has been actively misled. Absolute paths, not as given, so
the answer is unambiguous about what was read.

## 6. Room reserved for `set`

Not specified here, but the envelope must accommodate it without a
`schema` bump, so it is named now:

- **Preview** — a `set --dry-run` returns the same envelope with a
  `result` describing what *would* change. `SaveResult`'s existing
  `changed_files` / `diff_summary` / `requires_reload` is close to the
  right shape already.
- **Conflict** — `error.kind: "conflict"` is reserved for RFC 024's
  external-change detection.

## 7. Breaking changes this enumerates

The deprecation release's CLI list, as far as this RFC determines it:

| Change | Deprecation warning possible? |
|---|---|
| `validate --json` — bare array becomes the § 2 envelope | **Yes** — warn when `--json` is used, naming `--format json` |
| `--json` superseded by `--format json` on `validate` | **Yes** — accept both in 5.x, warn on `--json` |
| ~~`match-test` output shape aligns to the envelope~~ | **Not breaking — resolved 2026-08-17.** 6.0.0 *adds* `--format json` to `match-test` rather than changing its text output. Nothing to warn about, and the "is this text a contract?" question never needs answering |
| Bare `apimock` | **Not breaking** — kept as an alias (§ Layer 1) |

Everything else v6 breaks is library-side and cannot be warned about
(RFC 048 § 7.3); it goes to the migration guide.

**This table is the deliverable that unblocks the deprecation release.**
It is incomplete until `get` and `set` are designed, but the three rows
above are knowable now, and they are all `validate`/`match-test` shaped —
which suggests the deprecation release can be written before `get` and
`set` are finished, contrary to what RFC 048 § 7.3 assumed. Worth
confirming once those RFCs exist.

## Testing and verification

- A golden-file test per response shape, asserting the **serialised**
  envelope — the thing a consumer parses.
- A test proving an unknown field is tolerated by our own parsing, since
  we are the first consumer and should follow our own rule.
- `--format text` output is not asserted character-by-character; it is
  for humans and should stay free to improve.
- Exit codes asserted independently of payload, since a caller may only
  have one of them.

## Risks

| Risk | Mitigation |
|---|---|
| The envelope is over-designed for two commands | It is four keys. The cost of adding one later, after parsers exist, is what this RFC is avoiding |
| `schema` is incremented casually | It is a breaking change and gets the same treatment as any other; § 2's rule makes additive change the default path |
| Text output drifts from JSON output | They answer the same question; a difference is a bug. Named here so it is not discovered as a surprise |
| The enumeration in § 7 is incomplete | Stated as incomplete. It is a deliverable that grows with `get` and `set` |

## Unresolved questions

1. ~~**Does `match-test`'s current output count as a contract?**~~ ✅
   **Resolved 2026-08-17 by not needing an answer.** 6.0.0 adds
   `--format json` to `match-test` and leaves its text output alone, so
   no consumer of that text is affected either way. A question you can
   design around is better than one you have to research.
2. **Should `--format json` imply `--quiet`?** Mixing human progress
   output with a JSON document on the same stream would break parsers,
   but stream discipline may already prevent it. Establish from RFC 049's
   implementation rather than assuming.
3. **Does the GUI want this contract**, or does it keep the library API?
   RFC 048 § 12 asked the same question and it is still open. It decides
   whether this is one interface or two.
