# RFC 059 — CLI contract conformance: enforce it across every command, not per command

**Status.** **Accepted** — approved by the project owner 2026-08-20.
**Not yet implemented.**
[Handed off](../handoffs/059-cli-contract-conformance/implementation-handoff.md) 2026-08-20,
with its open questions decided. Blocking for 6.0.0.
**Tracks.** v6 CLI contract; release quality. **Blocking for 6.0.0** in
my view — see § Motivation.
**Touches.** `crates/apimock/src/args.rs` and each `cmd/*.rs`'s argument
parsing; a new shared test harness and conformance suite under
`crates/apimock/tests/`.
**Depends on.** [RFC 049](../done/049-cli-front-door.md) (the rule),
[RFC 053](../accepted/053-v6-cli-contract.md) (the taxonomy).

## Summary

Three of the four CLI commands accept unknown flags silently, each
failing differently. Fix them, and replace the per-command test files
that missed it with **one table asserting the contract across every
command**.

## Motivation

### The defect, measured

Verified 2026-08-20 against the built binary:

| Command | `--bogus` | Contract |
|---|---|---|
| `set` | exit **2** ✅ | 2 |
| `get` | exit **0** ❌ | 2 |
| `validate` | exit **0**, prints *"Validation passed"* ❌ | 2 |
| `match-test` | exit **1** ❌ — for an unrelated missing-argument reason, not the flag | 2 |

Only `set` is correct, and only because RFC 057's review caught it there.

**The case that decides this RFC's priority:**

```sh
$ apimock validate -c apimock.toml --strct   # typo of --strict
$ echo $?
0
```

`--strict` exists to make CI fail. Typo it and the job passes, having
silently run non-strict. Nothing warns.

### Why per-command tests could not catch it

`get_format.rs` (9 tests), `set_format.rs` (14), `validate_format.rs`
(9) and `args.rs` (9) each test their own command's paths. **No test
asserts a rule that must hold across all four**, so a rule implemented
in one place and forgotten in three looks locally fine everywhere.

Each of those files also defines its own `bin()` / `run_json()` /
`run_stderr()` helpers. There is no shared harness, so there is nowhere
for a cross-command assertion to live even if someone wanted to write
one. The structure of the tests is the reason the gap survived.

### Why it is blocking rather than polish

RFC 048 defines the v6 CLI as a first-class interface, and RFC 053's
whole purpose is a contract an **AI agent** can rely on. U2 is defined
as the user that *fails silently*. An agent that writes `--staus 404`
or `--strct` gets exit `0` and a success message, believes it worked,
and proceeds.

Shipping the contract while three of the four commands violate it makes
the contract a claim rather than a guarantee.

## Goals

1. Every command rejects unknown flags: `usage`, exit `2`, with the
   near-match suggestion `reject_unknown_arguments` already implements.
2. **One conformance table**, asserting exit code / kind / stream for
   every (command × scenario) pair — the mechanism that keeps 1 true.
3. One shared test harness; delete the four duplicated copies.
4. Settle the `config_invalid` exit-code contradiction (§ Design).

## Non-goals

- Changing RFC 053's taxonomy. This enforces it; it does not revise it.
- New commands or new flags, beyond § Unresolved 1's decision.
- Changing any command's *success* output.

## Design

### The conformance table

A single test module driving the real binary over a matrix:

| Scenario | Expected |
|---|---|
| unknown flag | `usage`, exit 2, stderr |
| known flag, missing value | `usage`, exit 2, stderr |
| mutually exclusive flags | `usage`, exit 2, stderr |
| config missing | `config_unreadable`, exit per § below |
| config malformed | `config_invalid`, exit per § below |
| success | exit 0, stdout only |

× every command. A new command joins by adding a row, which is the
point: **the contract becomes a property of the CLI rather than of
whoever wrote that command's tests.**

Where a command legitimately has no such scenario, the table says so
explicitly rather than omitting the row silently.

### The `config_invalid` contradiction, which must be settled here

RFC 053's table says `config_invalid` → exit **1**. Every command
actually exits **2**. A conformance suite cannot be written until one of
them is authoritative.

**Recommendation: change RFC 053's table to 2, not the code.** Exit `2`
for "I could not proceed with what you gave me" is consistent with the
usage class, is what three releases have shipped, and changing five
commands' exit codes at 6.0.0 to match a table nobody has implemented
would break CI jobs for a distinction of no practical value. The table
is the younger artifact and the wrong one.

This is a contract change, so it is the owner's call, not mine to
assume — but the suite is blocked until it is made.

## Testing and verification

- **The four repro commands above**, each asserted: exit `2`, kind
  `usage`, message naming the offending flag, nothing written to stdout.
- **`validate --strct` exits 2**, and the message suggests `--strict`.
  This is the regression test for the case that motivated the RFC.
- The conformance table passes for every command × scenario.
- **A deliberately broken command fails the table** — add a temporary
  command that ignores unknown flags, confirm the suite catches it,
  remove it. A conformance suite that cannot fail is decoration.
- The four duplicated helper sets are gone, and existing per-command
  tests still pass against the shared harness unchanged.

## Risks

| Risk | Mitigation |
|---|---|
| Fixing exit codes breaks someone's CI | It changes `0`→`2` only for invocations that were already wrong; a job relying on that was already not doing what it thought |
| The table becomes a place rows are added without thought | Each row asserts kind *and* stream *and* code; a wrong row fails rather than passes vacuously |
| Scope creep into rewriting per-command tests | Only the helpers are shared; the existing tests keep their assertions |

## Unresolved questions

1. **Does `match-test` join the envelope in 6.0.0?** It has **no
   `--format` support at all** today — it is the one command outside RFC
   053. RFC 054 deferred it deliberately, saying 6.0.0 would *add*
   `--format json` rather than change its text output. If it stays
   text-only, 6.0.0 ships a "first-class CLI contract" with one of four
   commands outside it, and an agent cannot consume `match-test` at all.
   Recommend adding it — additive, cheap, and it closes the surface.
2. **Should the near-match suggestion be mandatory?** `--strct` →
   *"did you mean `--strict`?"* is what turns a rejection into a fix.
   `reject_unknown_arguments` already has `near_match`; the question is
   whether the table asserts the suggestion or only the exit code.
   Recommend asserting it — it is the difference between a correct CLI
   and a usable one.
