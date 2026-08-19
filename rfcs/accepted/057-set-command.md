# RFC 057 — `apimock set`: make the server answer X under condition Y

**Status.** **Accepted** — approved by the project owner 2026-08-19.
**Not yet implemented.** [Handed off](../handoffs/057-set-command/implementation-handoff.md)
2026-08-19, with its three open questions decided.
**Tracks.** v6. [RFC 048](./048-v6-cli-interface-concept.md) § 11 item 5
— the write half, and the last of the umbrella's portfolio.
**Touches.** `crates/apimock/src/cmd/` (a new command), and whatever
`apimock-config` must expose to address a node from outside a session.
**Depends on.** [RFC 053](./053-v6-cli-contract.md) (contract),
[RFC 056](./056-toml-edit-migration.md) (formatting survives a write),
RFC 048 § 9 **T2** (scope).

## Summary

`apimock set` adds and changes rules from the command line, writing
configuration files that keep their comments and formatting, previewing
before committing, and refusing rather than overwriting a file that
changed underneath it.

## Motivation

### The engine exists; the address does not

`apimock-config` already has an edit vocabulary built for the GUI —
**fifteen `EditCommand` variants** covering rule sets, rules, responds,
root settings and header conditions, with `apply()` validating and
`save()` writing. RFC 056 just made that write path preserve what a
person wrote.

So `set` is a second front-end over an existing engine, as RFC 048 said
v6 would mostly be. **Except for one thing, and it decides the design.**

### The finding: `NodeId` cannot appear in a CLI contract

`EditCommand` addresses every target by `NodeId`. Its own stability
contract says:

> Stable within one `Workspace` instance — that is, across any sequence
> of `apply()` calls. **IDs are reassigned on fresh `load()`.**

`NodeId(pub Uuid)` — a fresh UUID per load.

**Every `apimock set` invocation is a new process, so a new `load()`, so
new IDs.** An ID printed by one command is meaningless to the next. The
addressing model assumes a long-lived session holding a `Workspace` in
memory; a CLI is the opposite of that.

This is the same shape as RFC 055's dispatch trap: the machinery is
there, and the assumption underneath it does not hold for the new
caller. Designing `set` around `NodeId` would produce a command that
looks right and cannot work across two invocations.

**So `NodeId` must never appear in `set`'s contract.** Addressing is by
natural key — something a user or an agent can write down and reuse
tomorrow.

There is already a mechanism: `NodeAddress` (`workspace/id_index.rs`,
`pub(crate)`) with `Root` / `RuleSet` / `Rule` / `Respond` / `Middleware`
variants, resolved to a `NodeId` by `id_for`. That is the natural-key
layer, built and currently private.

## Goals

1. Add a rule, and change a rule's response, from one non-interactive
   command.
2. **Address by natural key**, stable across invocations. No `NodeId` in
   the contract.
3. **`--dry-run` previews** — what would change, without changing it.
4. A failed `set` changes **nothing** (RFC 056 gives this).
5. RFC 053's envelope under `--format json`, with `conflict` and `io`
   distinguished.
6. Comments and formatting survive (RFC 056 gives this).

## Non-goals

- **`service.middlewares`.** T2: deferred, not refused. `set` neither
  adds, changes nor removes an entry; existing entries pass through
  untouched.
- Replacing the GUI's `EditCommand` API. `set` is a second caller of it.
- Interactive editing. Non-interactive by construction — U2 and U3.
- Talking to a running server. Files on disk, as `get` does.
- Exposing every one of the fifteen `EditCommand` variants in the first
  cut. See § Unresolved 1.

## Design

### Addressing

A natural key that survives a process boundary. The obvious
candidates — a rule set's path plus a rule's index — are what
`NodeAddress` already models, and what `--why` output from `get`
already reports.

**That last point matters and should shape the choice**: `get --why`
already tells a caller *"rule set X, rule #2 decided this"*. If `set`
addresses rules the same way, the two commands compose — an agent reads
an address out of `get` and writes it into `set` without translation.
Anything else makes the pair harder to use than either alone.

### Preview

`--dry-run` returns the same envelope with a `result` describing what
*would* change. `SaveResult` already carries `changed_files`,
`diff_summary` and `requires_reload`; `compute_diff_summary` is
`pub(super)` today and would need exposing.

**This is also what makes `set` usable by the GUI** (G5: the GUI moves
onto the CLI contract), which needs a diff a person can read before
committing — the same need, arriving from a different user.

### Failure

Inherited from RFC 056 rather than invented: a content mismatch is
`SaveError::Conflict`, a read failure is `SaveError::Read`, both checked
across the whole write set before any write. Map them to RFC 053's
`conflict` and `io` kinds respectively — the library distinguishes them
precisely so the CLI can.

## The acceptance test — RFC 048's W7, concretely

RFC 048 proposed defining v6's completion as *a script that runs* rather
than a feature list. Here it is, and this RFC proposes it as the bar:

```sh
mkdir demo && cd demo
apimock set rule --path /users/1 --status 200 --json '{"id":1}'          --format json
apimock set rule --path /users/1 --header 'x-api-key: k1' --status 403   --format json
apimock get /users/1                                                     --format json
apimock get /users/1 --header 'x-api-key: k1'                            --format json
apimock validate                                                         --format json
```

Every step non-interactive, every exit code asserted, running in CI on
every commit. **If that script is awkward to write, the design is
wrong** — and we find out during design rather than after an agent tries
it.

The exact flag spellings are this RFC's to settle; the shape is the
commitment.

## Testing and verification

- Add a rule, then `get` it back and receive it. The round trip is the
  product.
- **A file with comments survives a `set`** — RFC 056's guarantee, tested
  again here because `set` is the surface that promises it to users.
- `--dry-run` changes nothing on disk, and its `result` matches what a
  real run then does.
- Conflict: change a file after load, `set`, receive `conflict` — and
  **no file modified**.
- `service.middlewares` untouched by every command, including when the
  config already has entries.
- The W7 script runs green in CI.

## Risks

| Risk | Mitigation |
|---|---|
| Addressing that does not survive a process boundary | The finding above is the reason this RFC exists; `NodeId` is excluded by design, not by care |
| `set` and `get` address rules differently | Explicit goal that they compose; the W7 script exercises exactly that |
| Exposing all fifteen `EditCommand`s at once | Unresolved 1 — start with what W7 needs |
| A partially-applied multi-command invocation | RFC 056 checks the whole write set before writing; keep that property rather than re-deriving it |

## Unresolved questions

1. **How much of `EditCommand` does the first cut expose?** W7 needs
   adding a rule with a status, a JSON body and a header condition.
   The other variants — `MoveRule`, `RemoveRuleSet`, root settings — are
   real but not on the path to the acceptance test. Recommend: what W7
   needs, then extend on evidence rather than symmetry.
2. **Does `set` apply more than one change per invocation?** A batch is
   more efficient for an agent and multiplies the failure modes. RFC 053
   § 6 reserved room for a transaction boundary without specifying one.
3. **Does `NodeAddress` become public, or does `set` get its own
   address type?** The former reuses what exists; the latter avoids
   exposing an internal shape on a contract that must stay stable.
   Establish from source what `NodeAddress` would commit us to.
