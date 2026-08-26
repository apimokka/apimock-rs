# RFC 064 — Finish the CLI front door: caller-supplied paths and flag values

**Status.** Proposed — awaiting owner approval.
**Tracks.** v6 CLI contract. **Blocking for 6.0.0** — these are RFC 048
§ 6 prerequisites that RFC 048 itself promotes to blocking.
**Touches.** `crates/apimock-config/src/path_util.rs`,
`crates/apimock/src/cmd/flags.rs` and its callers,
`crates/apimock/tests/cli_conformance.rs`,
`docs/src/reference/cli-reference.md`.
**Depends on.** [RFC 049](../done/049-cli-front-door.md),
[RFC 053](../accepted/053-v6-cli-contract.md),
[RFC 059](../accepted/059-cli-contract-conformance.md).

## Summary

Two caller-supplied inputs are still mishandled — a bare relative
`--config` path, and a known flag given no value. Both are the
silent-or-wrong-behaviour class RFC 048 calls disqualifying for U2, both
are named in RFC 048 § 6, and the contract page documents neither
correctly.

## Motivation

RFC 048 § 6 lists seven v5 items *"that v6 promotes to blocking"*. Four
are closed. This RFC closes two more. The seventh — RFC 045 Goal 4 — is
deliberately out of scope; see § Not in scope.

### Defect 1 — a bare relative `--config` fails

RFC 048 § 6 states the case exactly: *"Bare relative `--config
apimock.toml` fails — **an agent will write exactly that**."*

```
$ apimock validate -c apimock.toml
apimock validate: failed to load config: failed to resolve path
`apimock.toml`: No such file or directory (os error 2)
```

The file exists. `-c ./apimock.toml` succeeds. Reproduced on all three
commands (`validate`, `get`, `set`), each exiting 2.

**Root cause**, `apimock-config/src/path_util.rs:18`:

```rust
let parent = Path::new(file_path).parent().ok_or_else(…)?;
relative_path(env::current_dir()?.as_path(), parent)
```

`Path::new("apimock.toml").parent()` returns **`Some("")`**, not `None`
— so the guard does not fire, and `relative_path` then calls
`fs::canonicalize("")`, which fails with `ENOENT`. The error names the
config file, so it reads as *"your config is missing"* when the config
is present and the empty string is the thing that could not be resolved.

This was seen twice before and fixed neither time: noted in RFC 055's
review as a pre-existing gap, and hit again inside RFC 057, where the
implementation worked around it internally (`set.rs` keeps the `./`
prefix deliberately). **The workaround landed; the flag surface did
not.**

### Defect 2 — a known flag with no value is silently ignored

RFC 059 fixed *unknown* flags. A **known** flag whose value is missing is
the same failure class and is not fixed:

| Command | Documented | Actual |
|---|---|---|
| `validate -c` | exit 1 | **exit 2** |
| `get /a -c` | exit 1 | **exit 0** — flag ignored, request answered |
| `set … -c` | exit 1 | **exit 0** — flag ignored, **wrote to the default config** |

**Root cause**, `apimock/src/cmd/flags.rs:8`:

```rust
let idx = args.iter().position(|a| names.iter().any(|n| a == n))?;
args.get(idx + 1).filter(|v| !v.starts_with('-')).cloned()
```

It returns `Option<String>`, and *"flag present, no value"* collapses
into `None` — **indistinguishable from "flag absent"**. Every caller
then takes its default.

The `set` row is the one that matters. A dangling `-c` means *"use this
config"*, and apimock wrote to a different file, one the caller never
named. RFC 048 § 9's whole concern is `set` writing where it was not
asked to; this is that, reached through the argument parser rather than
through a path.

### Why the conformance suite did not catch it

`cli_conformance.rs` carries two deliberate empty tests —
`get_has_no_known_flag_missing_value_scenario` and its `set` twin —
asserting those commands have no such scenario. RFC 059 asked for
inapplicable scenarios to be **stated rather than omitted**, so this
followed instruction faithfully.

The instruction was right and the claim written under it was wrong. That
is worth recording: an explicit "not applicable" is only as good as the
check behind it, and neither the dev team nor I verified this one.

### Defect 3 — the contract page is wrong about both

`docs/src/reference/cli-reference.md` § Exit codes is what an agent
author builds against. Measured:

- *"`1` … the same code as a referenced file not existing"* — that case
  exits **2**.
- *"`-c` with nothing after it … fails later … as exit `1`"* — wrong for
  all three commands (2, 0, 0).
- Exit `1` appears **unreachable** on every path tried.
- `config_invalid` / `config_unreadable` exit codes are not stated,
  though RFC 053 was amended on 2026-08-20 to record them as `2`.

## Goals

1. A bare relative `--config apimock.toml` works, on every command.
2. A known flag given no value is a `usage` error, exit `2`, on every
   command — never silently defaulted.
3. The exit-code documentation matches the binary, including the codes
   for every `error.kind`.
4. The conformance table covers both, so neither can regress.

## Non-goals

- Rewriting argument parsing. RFC 049's scan stays; `flag_value` gains a
  third outcome.
- Changing which flags exist, or any command's success output.
- Making exit `1` reachable. If it is genuinely unreachable, the
  documentation should say so — see § Unresolved 1.

## Not in scope — the fourth open prerequisite

RFC 048 § 6 also lists **RFC 045 Goal 4** — `validate` passing on inert
configuration — as blocking, on the grounds that *"U2 generating config
changes that calculus"*.

No RFC after 045 addresses it. RFC 045 said of the structural option:
*"it may be impractical; that is a legitimate outcome, **but it should
be established rather than assumed**."* It remains assumed.

**That is a decision, not implementation work, and it is the owner's.**
Accepting the limitation with a stated reason is a legitimate
resolution; leaving it unexamined at the release boundary is not, since
RFC 048 made it blocking. Deliberately excluded here so this RFC stays
one coherent piece of work.

## Design

### Bare relative paths

Treat an empty parent as the current directory:

```rust
let parent = Path::new(file_path).parent().unwrap_or(Path::new(""));
let parent = if parent.as_os_str().is_empty() { Path::new(".") } else { parent };
```

`Path::parent()` returning `Some("")` for a bare filename is correct
behaviour, not a quirk to work around elsewhere — the mapping from "no
directory component" to "the current directory" belongs here, once,
rather than in each caller as `set.rs` had to do.

### Flag values

`flag_value` must distinguish three states, not two:

| Input | Today | Wanted |
|---|---|---|
| flag absent | `None` | absent |
| `-c path` | `Some(path)` | `Some(path)` |
| `-c` (end of args, or followed by another flag) | `None` | **usage error** |

`flag_present` already exists, so callers can distinguish presence from
value without a signature change — but doing it caller-by-caller invites
exactly the omission this RFC is fixing. **Return a `Result`** so a
caller that ignores the case cannot compile.

Apply to every value-taking flag, not just `-c`: `--rule-set`, `--rule`,
`--path`, `--method`, `--status`, `--json`, `--text`, `--file`,
`--delay`, `--format`, `--header`, `--body`, `--body-file`.

### Documentation

Rewrite § Exit codes against measured behaviour, and give each
`error.kind` its exit code explicitly. If exit `1` is unreachable, say
so plainly rather than describing a code nothing emits.

## Testing and verification

- `-c apimock.toml` (bare) succeeds on `validate`, `get`, `set`, from a
  directory where the file exists.
- `-c ../apimock.toml` and an absolute path keep working.
- **Every value-taking flag**, given no value, is `usage` / exit 2 —
  driven from the conformance table, one row per flag per command.
- **`set … -c` with a dangling flag writes nothing.** Check the
  directory, not just the exit code — this is the defect's sharp end.
- The two placeholder conformance tests are replaced by real rows.
- Every documented exit code is asserted by a test, so the page cannot
  drift from the binary again.
- Full suite green; `fmt`; `clippy -D warnings`; `mdbook build docs`.

## Risks

| Risk | Mitigation |
|---|---|
| Turning a previously-tolerated invocation into an error breaks a script | It only breaks invocations that were already not doing what they said. That is the point |
| `Result` from `flag_value` touches every caller | The compiler enumerates them; that is why it is a `Result` rather than a convention |
| The empty-parent fix changes path resolution elsewhere | It only affects paths with no directory component, which currently fail outright |
| Documentation drifts again | Goal 4: each documented code gets a test |

## Unresolved questions

1. **Is exit `1` reachable at all?** I could not reach it. If it is not,
   either something should use it or the contract should drop it —
   RFC 053 names it as *"everything else"*, and a code nothing emits is
   a contract that misdescribes itself. Establish before documenting.
2. **Should a flag repeated with one value missing error?**
   `--header a:b --header` — the first is usable. Erroring is simpler to
   explain and matches Goal 2; accepting it silently is the behaviour
   this RFC exists to remove. Recommend erroring.
