# RFC 049 — The CLI front door: version, help, and refusing what it does not understand

**Status.** Implemented (v5.18.0). Approved by the project owner
2026-08-17; reviewed in `REVIEW-001` / `REVIEW-002`.
**Tracks.** Correctness and product readiness. `apimock --version` starts
a mock server. So does `--help`. So does any typo. The CLI accepts
everything and understands almost none of it.
**Touches.** `crates/apimock/src/args.rs`, `crates/apimock/src/args/constant.rs`,
documentation. **No change to the mock-serving path.**
**Governed by.** [RFC 048](../accepted/048-v6-cli-interface-concept.md) § 6 — this
is the first of its prerequisites, and it blocks v5's deprecation
release.

## Summary

Make the CLI reject what it does not understand, and answer the two
questions every user asks first. Nothing here is new capability; it is
the difference between a launcher and an interface.

## Motivation

### What happens today

`args_option_value` (`crates/apimock/src/args.rs`) scans `env::args()`
for a fixed set of known names and ignores everything else. The
recognised surface is exactly:

```
match-test, validate                 (positional, checked at argv[1])
-c/--config  -p/--port  -d/--dir
--init  --middleware  -y/--yes
```

Anything else is silently discarded and the server starts. Verified
against the published v5.16.0 binary:

```
$ ./apimock --version
[log.verbose] header = No, body = No
Greetings from apimock-rs (API Mock) !!
Listening on http://127.0.0.1:3001 ...
```

Not an error. A running server, on a port the caller never asked for.

### Why this is more than untidiness

**For a person**, a typo means the tool quietly does something other than
what was asked. `apimock --prot 4000` starts on 3001, and the mistake
surfaces later as a connection failure somewhere else entirely.

**For an AI CLI agent — RFC 048's U2 — it is disqualifying.** The agent
runs a command, sees exit code 0 and a server starting, records success,
and proceeds on a false premise. Silent wrong behaviour that looks like
success is the single worst outcome for a non-interactive caller, and
every ambiguity we leave becomes a confidently wrong answer downstream.

**And it blocks v5's own ending.** RFC 048 § 7.1 commits to a deprecation
window in a 5.x release: old invocations must warn, naming their
replacement, before v6 removes them. **A CLI that silently ignores what
it does not recognise cannot deliver a warning anyone acts on.** The
deprecation release depends on this one.

### A second defect in the same surface

Bare relative `--config` does not resolve, while `./`-prefixed does:

```
$ ./apimock -c apimock.toml
Error: failed to resolve path `apimock.toml`: No such file or directory

$ ./apimock -c ./apimock.toml
[config] ./apimock.toml
```

Already recorded in `ROADMAP.md`'s findings table and documented as a
quirk in `docs/src/reference/cli-reference.md`. It belongs here because
it is the same surface and the same user encountering it — and because
an agent will write the bare form.

## Goals

1. An unrecognised argument is an **error**, not silence.
2. `--version` prints the version and exits 0.
3. `--help` prints usage and exits 0; the same text is reachable per
   subcommand.
4. Bare relative `--config apimock.toml` resolves like `./apimock.toml`.
5. Errors are written to **stderr**; `--version` / `--help` output to
   stdout. Exit codes distinguish success, user error, and internal
   failure.

## Non-goals

- Restructuring the CLI into subcommands. That is v6's decision
  (RFC 048 § 7), and doing it here would break invocations before the
  deprecation window exists to announce it.
- Adding `get` / `set`. This RFC only makes the door work.
- Adopting a CLI argument-parsing crate. It may be the right answer —
  see Unresolved question 1 — but it is a dependency decision, not a
  goal in itself.
- Changing what any existing, *valid* invocation does.

## Proposed design

**Reject unknown arguments.** After the known names are consumed, any
remaining argument that begins with `-` is an error naming the offender.
Where a near-match exists, say so — `unknown option '--prot'; did you
mean '--port'?` — because that line is what turns a failure into a
self-correction for both U1 and U2.

**`--version` and `--help` short-circuit**, before configuration is read
and before any listener is bound, so they work in a broken workspace.
That matters: "what version am I running" is the first question asked
when something is wrong.

**Exit codes**, and this RFC fixes them for the whole CLI rather than
inventing a scheme per command:

| Code | Meaning |
|---|---|
| 0 | Success, including `--version` / `--help` |
| 2 | Usage error — unknown option, missing or invalid value |
| 1 | Everything else (config invalid, bind failure, …) |

`2` for usage is the long-standing Unix convention and lets a caller
distinguish "I invoked it wrongly" from "it ran and failed", which is
exactly the distinction RFC 048 § 3.3 requires.

**Stream discipline.** `--version` / `--help` to stdout; all diagnostics
to stderr. RFC 048 § 3.1 makes this a hard rule for v6; adopting it here
means the deprecation warnings that follow have somewhere correct to go.

**Path resolution.** Resolve `--config` relative to the current working
directory when it has no directory component, matching how every other
CLI treats a bare filename. Confirm from source whether the same fault
affects `--dir`, and fix it there too if so — do not assume either way.

## Testing and verification

- Unknown option → exit 2, message on stderr, **no server started**.
- Near-match suggestion appears for a plausible typo.
- `--version` and `--help` → exit 0, output on stdout, **no server
  started**, and both work with no config file present and with a
  deliberately invalid one.
- `-c apimock.toml` and `-c ./apimock.toml` resolve identically.
- Every currently-valid invocation still behaves exactly as before —
  this is the regression that matters most, since the whole point is to
  reject *only* what was already meaningless.
- The example sets and `docs/src/reference/cli-reference.md` are checked
  for text this change falsifies; the `-c` quirk note in the CLI
  reference becomes wrong once goal 4 lands and must be updated in the
  same change.

## Risks

| Risk | Mitigation |
|---|---|
| Someone relies on a stray argument being ignored | Vanishingly unlikely, and it is precisely the behaviour being fixed. Called out in release notes as a behaviour change |
| Rejecting arguments breaks an existing script | Only arguments that previously did *nothing* are rejected; every meaningful invocation is unchanged, and the test suite must show it |
| Exit-code changes break CI callers | Today almost everything exits 0 or 1; introducing 2 for usage errors affects only invocations that were already wrong |

## Unresolved questions

1. **Hand-rolled or a parsing crate?** The current parser is ~15 lines
   and has one defect class. A crate (`clap` or similar) brings `--help`,
   `--version`, unknown-argument rejection and suggestions for free, but
   is a dependency and a public-behaviour change of its own, and v6 will
   restructure this surface anyway. Establish the cost both ways rather
   than assuming; report the recommendation with the reasoning.
2. **Does the same resolution fault affect `--dir`?** Establish from
   source, do not infer from `--config`.
3. **Should `apimock` with no arguments keep starting a server?** It is
   the current behaviour and much documentation depends on it. Assumed
   yes; flagged because a strict reading of goal 1 might argue otherwise.
