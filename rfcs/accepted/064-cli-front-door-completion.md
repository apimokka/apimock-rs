# RFC 064 — Finish the CLI front door: caller-supplied paths and flag values

**Status.** Accepted — owner approved 2026-08-27.
**Tracks.** v6 CLI contract. **Blocking for 6.0.0** — these are RFC 048
§ 6 prerequisites that RFC 048 itself promotes to blocking.
**Touches.** `crates/apimock-config/src/path_util.rs`,
`crates/apimock/src/cmd/flags.rs` and its callers,
`crates/apimock/tests/cli_conformance.rs`,
`docs/src/reference/cli-reference.md`.
**Depends on.** [RFC 049](../done/049-cli-front-door.md),
[RFC 053](./053-v6-cli-contract.md),
[RFC 059](./059-cli-contract-conformance.md).

## Summary

Two caller-supplied inputs are still mishandled by the subcommands — a
bare relative `--config` path, and an optional flag given no value. Both
are the silent-or-wrong-behaviour class RFC 048 calls disqualifying for
U2, both are named in RFC 048 § 6, and the contract page is wrong about
both — in one case documenting the fixed behaviour as though it already
shipped.

Every claim below was measured against a binary built from `9bdc769`,
in a scratch directory populated by `apimock --init --yes`.

## Motivation

RFC 048 § 6 lists seven v5 items *"that v6 promotes to blocking"*. Four
are closed. This RFC closes two more. The seventh — RFC 045 Goal 4 — is
deliberately out of scope; see § Not in scope.

### Defect 1 — a bare relative `--config` fails on the subcommands

RFC 048 § 6 states the case exactly: *"Bare relative `--config
apimock.toml` fails — **an agent will write exactly that**."*

```
$ apimock validate -c apimock.toml
apimock validate: failed to load config: failed to resolve path
`apimock.toml`: No such file or directory (os error 2)     [exit 2]

$ apimock validate -c ./apimock.toml
Validation passed (2 rules across 1 rule set(s)).          [exit 0]
```

Same on `get`. The file exists in both runs; only the spelling differs.
The error names the config file, so it reads as *"your config is
missing"* when the config is present and the empty string is the thing
that could not be resolved.

**Trigger condition** — worth stating, because it is why this survived:
the failure needs a config that loads far enough to resolve a path
relative to its own directory (one with `rule_sets`). A config that
fails to parse errors earlier and never reaches it, so a casual check
with a throwaway config shows no problem. *(I made exactly that mistake
while preparing this RFC — an invalid fixture showed the bare form
working, and the finding nearly went in backwards.)*

**Root cause**, `apimock-config/src/path_util.rs:19`:

```rust
let parent = Path::new(file_path).parent().ok_or_else(…)?;
relative_path(env::current_dir()?.as_path(), parent)
```

`Path::new("apimock.toml").parent()` returns **`Some("")`**, not `None`
— so the guard never fires, and `relative_path` then calls
`fs::canonicalize("")`, which fails with `ENOENT`. The doc comment above
it states the intended contract (*"`Path::parent()` returns `None` for
root-only paths and empty paths"*), and that is simply not what
`Path::parent()` does for a bare filename.

**This was already solved once, in the wrong place.** RFC 049 added
`normalize_bare_relative_path` (`crates/apimock/src/args.rs:397`), which
prepends `./` when a path has no directory component. It is applied at
`args.rs:187` — **the root/server parser only**. The subcommands
arrived later (RFC 055, RFC 057) with their own flag parsing in
`cmd/flags.rs`, and never picked it up. RFC 057's `set.rs` then worked
around the same problem internally a third time by keeping a `./`
prefix. Three encounters, no fix at the layer where it belongs.

### Defect 2 — an optional flag with no value is silently defaulted

RFC 059 fixed *unknown* flags. A **known** flag with a missing value is
the same failure class, and the split is sharp:

| | Dangling flag | Result |
|---|---|---|
| **Required flags — correct today** | `validate -c` | exit 2, *"missing required flag --config / -c"* |
| | `match-test --rule-set` | exit 2, *"--rule-set <path> is required"* |
| **Optional flags — silently defaulted** | `get … --method` / `--header` / `--body` / `--format` | **exit 0**, flag ignored |
| | `validate … --format` | **exit 0**, flag ignored |
| | `match-test … --path` / `--method` | exit 1 — defaulted, then "no match" |
| | `set rule … -c` | **exit 0**, and **writes to a config the caller never named** |

Commands with an explicit required-flag check catch it. Everything else
takes its default and reports success.

**Root cause**, `apimock/src/cmd/flags.rs:8`:

```rust
let idx = args.iter().position(|a| names.iter().any(|n| a == n))?;
args.get(idx + 1).filter(|v| !v.starts_with('-')).cloned()
```

It returns `Option<String>`, and *"flag present, no value"* collapses
into `None` — **indistinguishable from "flag absent"**. Every caller
then takes its default. `flag_values_all` (line 15) drops such a flag
the same way.

Two consequences deserve naming individually.

**`set` writes to the wrong file.** `set rule --path /x --status 418 -c`
exits 0 and appends the rule to the auto-discovered rule set. The caller
said *"use this config"*; apimock used a different one and reported
success. RFC 048 § 9's entire concern is `set` writing where it was not
asked to — this reaches it through the argument parser instead of
through a path.

**`--format` is the U2 case exactly.** U2 is defined by failing
*silently*. An agent that asks for `--format json` and loses the value —
dangling at the end, or followed by another flag — gets **human text,
exit 0**. No error, no envelope, and RFC 053's whole contract silently
absent. That is the single most likely way an agent meets this bug.

### Why the conformance suite did not catch it

`cli_conformance.rs` carries two deliberate empty tests —
`get_has_no_known_flag_missing_value_scenario` and its `set` twin —
asserting those commands have no such scenario. RFC 059 asked for
inapplicable scenarios to be **stated rather than omitted**, so this
followed instruction faithfully.

The instruction was right and the claim written under it was wrong. An
explicit "not applicable" is only as good as the check behind it, and
neither the dev team nor I verified this one.

### Defect 3 — the contract page is wrong, including about its own fix

`docs/src/reference/cli-reference.md` is what an agent author builds
against. Measured against it:

1. **It documents Defect 1 as already fixed.** The flags table says:
   *"A bare relative path resolves the same as one prefixed with `./` —
   `-c apimock.toml` and `-c ./apimock.toml` are equivalent."* True for
   the server, false for every subcommand. **This is worse than an
   omission** — an agent reading that line will write the bare form on
   the page's own authority.
2. **It documents exit `1` for a dangling flag.** Measured: `2` for
   required flags, `0` for optional ones. `1` occurs only for
   `match-test`'s "no match", which is that command's success/failure
   axis, not an argument error.
3. **It argues the defect is unavoidable**, in prose:
   *"telling them apart would mean changing that scan, which every
   other flag's exact behaviour depends on staying untouched."* This is
   false, and the code already disproves it —
   `reject_unknown_flags(args, known, no_value)` is **passed the closed
   set of flags that take no value** precisely so it can tell them
   apart. The information is already threaded through the parser.
4. `config_invalid` / `config_unreadable` exit codes are not stated,
   though RFC 053 was amended on 2026-08-20 to record them as `2`.

## Goals

1. A bare relative `--config apimock.toml` works on every command.
2. An optional flag given no value is a `usage` error, exit `2`, on
   every command — never silently defaulted.
3. The CLI reference matches the binary, including an exit code for
   every `error.kind`, with the false rationalisation removed.
4. The conformance table covers both defects, so neither can regress.

## Non-goals

- Rewriting argument parsing. RFC 049's scan stays; `flag_value` gains a
  third outcome.
- Changing which flags exist, or any command's success output.
- Changing required-flag handling — it is already correct.
- Making exit `1` reachable as a general error code; see § Unresolved 1.

## Not in scope — the fourth open prerequisite

RFC 048 § 6 also lists **RFC 045 Goal 4** — `validate` passing on inert
configuration — as blocking, on the grounds that *"U2 generating config
changes that calculus"*.

> **Correction, 2026-08-27.** An earlier revision of this section said
> Goal 4 *"remains assumed"* and called it an open owner decision. **That
> was wrong, and I stated it twice before checking.** RFC 045's own
> Unresolved question 2 records it as **✅ Resolved**: option 3
> (structural exhaustiveness) was investigated *with a working
> prototype* and rejected on a specific technical basis — `Respond` and
> `DefaultRespond` both carry a field named
> `delay_response_milliseconds`, so a name-based check reports
> "referenced" and passes while blind to whether the rule-set copy was
> ever read, which was RFC 045's Defect 2 exactly. Option 2 has no
> target: 0 of 47 fields were inert after the fixes, and everything
> known-inert (`[guard]`, `[file_tree_view]`, the trace channel) already
> has its own disposition. Option 1 stands, with reasons. RFC 045 asked
> for the outcome to be *established rather than assumed* — and it did
> exactly that.

What RFC 048 § 6 actually leaves open is narrower: whether v6's
machine-authored configs change that conclusion. **They do not, and the
surface is smaller than in v5.** Measured 2026-08-27: an unknown key is
**rejected at load**, both under `[rules.guard]` and at
`[service]` level (exit 2, *"configuration validation failed"* /
*"failed to load config"*). A config author — human or agent — cannot
write a setting apimock silently ignores, because a setting apimock does
not know is an error. The U2 concern was that an agent writes a key,
apimock ignores it, and the agent believes it took effect; that
sequence cannot occur.

**Disposition: closed, no owner decision required.** Excluded from this
RFC's implementation scope, and not blocking 6.0.0.

## Design

### Bare relative paths — fix at the config layer, not per caller

Map "no directory component" to "the current directory" in
`path_util.rs`, where every caller passes through:

```rust
let parent = Path::new(file_path).parent().unwrap_or_else(|| Path::new(""));
let parent = if parent.as_os_str().is_empty() { Path::new(".") } else { parent };
```

`Path::parent()` returning `Some("")` for a bare filename is correct
behaviour, not a quirk to route around. Fixing it here fixes all four
subcommands, the root parser, and any future caller, at once — which is
the point, given this is the third place the same problem has been
patched. Update the doc comment above it, which currently describes a
`Path::parent()` contract that does not hold.

Leave `normalize_bare_relative_path` in place. It becomes redundant but
remains correct, and `crates/apimock/tests/args.rs:280` pins its
empty-string behaviour deliberately — do not disturb that in this RFC.

### Flag values — make the third state unignorable

`flag_value` must distinguish three states, not two:

| Input | Today | Wanted |
|---|---|---|
| flag absent | `None` | absent |
| `-c path` | `Some(path)` | `Some(path)` |
| `-c` at end of args, or followed by another flag | `None` | **usage error** |

**Return a `Result`**, so a caller that ignores the case does not
compile. `flag_present` already exists, so presence-vs-value could be
distinguished caller-by-caller without a signature change — but doing it
that way invites exactly the omission this RFC is fixing, across 26 call
sites. Let the compiler enumerate them.

Apply the same to `flag_values_all`.

**The closed set of call sites** (26, measured — no others exist):

| File | Lines |
|---|---|
| `crates/apimock/src/cmd/set.rs` | 174, 175, 176, 182, 183, 184, 195, 201, 206, 210, 211, 220 |
| `crates/apimock/src/cmd/get.rs` | 86, 87, 90, 99, 100, 105 |
| `crates/apimock/src/cmd/match_test.rs` | 174, 177, 189, 190, 194, 204, 205, 208 |
| `crates/apimock/src/cmd/validate.rs` | 72, 76 |

Where a command already emits a good required-flag message
(`validate -c`, `match-test --rule-set`), **keep that message** — it is
better than a generic one. The new error is for the optional flags that
currently say nothing.

### Documentation

- Correct the flags-table claim about bare relative paths — after the
  fix it becomes true for every command, so verify rather than delete.
- Rewrite § Exit codes against measured behaviour; give each
  `error.kind` its exit code explicitly.
- **Delete the paragraph arguing the flag scan cannot tell the two
  cases apart.** It is not a caveat to soften; it is wrong.

## Testing and verification

- `-c apimock.toml` (bare) succeeds on `validate`, `get`, `set`,
  `match-test`, **using a config with `rule_sets`** — an invalid or
  minimal config does not exercise the failing path (see Trigger
  condition above).
- `-c ../apimock.toml`, `-c ./apimock.toml` and an absolute path keep
  working.
- **Every optional value-taking flag, on every command**, given no
  value, is `usage` / exit 2 — one conformance row per flag per
  command, driven from the table rather than hand-picked.
- Both forms of "no value": flag at end of args, **and** flag followed
  by another flag.
- Required-flag messages are unchanged (pin the two existing strings).
- **`set rule … -c` with a dangling flag writes nothing.** Assert on the
  file contents, not the exit code — this is the defect's sharp end, and
  an exit-code-only assertion would have passed while it wrote.
- **`--format` dangling never yields text output with exit 0.**
- The two placeholder conformance tests are replaced with real rows.
- Every documented exit code is asserted, so the page cannot drift again.
- Full suite green; `fmt`; `clippy -D warnings`; `mdbook build docs`.

## Risks

| Risk | Mitigation |
|---|---|
| Turning a tolerated invocation into an error breaks a script | It only breaks invocations that were already not doing what they said — `--format json` silently yielding text is not behaviour worth preserving |
| `Result` from `flag_value` touches 26 call sites | The compiler enumerates them; that is why it is a `Result` and not a convention. The closed set is listed above |
| The empty-parent fix changes path resolution elsewhere | It only affects paths with no directory component, which today fail outright — there is no working behaviour to regress |
| `normalize_bare_relative_path` and the new fix interact | The former prepends `./`, which the latter already handles; keep both, and keep `tests/args.rs:280` green |
| Documentation drifts again | Goal 4: each documented code gets a test |

## Amendment 1 — a `--flag=value` form (approved 2026-08-27)

**Status.** Approved after a risk assessment the owner asked for
explicitly. Brief:
`.git-exclude/reviewed/pre-6.0.0-audit/DECISION-001-flag-equals-value.md`.
**Handoff.** [`amendment-flag-equals-value.md`](../handoffs/064-cli-front-door-completion/amendment-flag-equals-value.md)

### Why

Found while reviewing this RFC's own implementation (§ 5 of the dev
team's package established there was no *regression* from
`-`-prefixed values; the further question was whether such a value is
expressible **at all**). It is not:

| Attempt | Result |
|---|---|
| `--text "- item"` | exit 2 — `unrecognized argument '- item'` |
| `--text=-hello` | exit 2 — `unrecognized argument '--text=-hello'` |
| `--text -- -hello` | exit 2 — `unknown option '--'` |

`reject_unknown_flags` rejects any token starting with `-` before
`flag_value` is reached, and neither standard escape hatch exists. So
`apimock set rule --text` cannot express a markdown bullet, a YAML
`---`, or a diff hunk — on v6's headline feature, whose intended author
is a machine generating fixtures. The error also misnames the problem:
*"unknown option '- item'; did you mean '-c'?"*

Pre-existing across all of v5; *loud* (exit 2) rather than silent, which
puts it a class below this RFC's original two defects — but with no
workaround at all.

### The security condition — not advice, a gate

**A no-value flag given any `=` form must be a usage error, exit `2`.**

Presence is read by `flag_present`, which matches the flag name exactly
and **discards any value**. A naive `=`-splitting implementation that
resolves `--allow-outside=false` to "flag present" would **disable
RFC 062's write-path confinement while the author wrote `false` to keep
it on** — a security property inverted by a token the parser throws
away, at exit 0, with no diagnostic.

Today all such forms are cleanly rejected (`unknown option
'--allow-outside=false'`). **This is therefore a regression to avoid,
not a hole to close** — a partial implementation is strictly worse than
shipping nothing.

Severity is not uniform, and is stated here rather than flattened:

| Flag | Command | If mis-parsed | Severity |
|---|---|---|---|
| `--allow-outside` | `set` | RFC 062 write confinement disabled | **Severe** |
| `--dry-run` | `set` | Nothing written when the caller expects a write | Moderate |
| `--yes` | root (`args.rs:120`, `args_option_value(…).is_some()`) | Prompts skipped, defaults accepted. **Cannot overwrite an existing config** — `--init` refuses regardless, verified | Moderate |
| `--why`, `--quiet`, `--strict`, `--json`, `--middleware` | various | Output/verbosity only | Low |

`=true` must be rejected exactly as `=false` is. Accepting one and not
the other is the asymmetry that gets "simplified" later into accepting
both.

### Scope: all three parsers, and consolidate

The same skip-next rule exists **three** times — `args.rs:340` (root),
`cmd/flags.rs:80` (`get`/`validate`/`match-test`), and
**`cmd/set.rs:127`, a private duplicate** serving `set`. RFC 059 left
the duplicate deliberately, which was sound then and is not now: **the
one command carrying `--allow-outside` is the one that does not share
the common parser.**

This is the same shape as this RFC's own root cause —
`normalize_bare_relative_path` fixed in one place and patched around
twice more. **Consolidate `set.rs`'s copy into the shared one** rather
than adding `=` handling to both. The root command is included for the
same reason: adding a form to two parsers out of three deepens exactly
the divergence that caused this.

### Settled decisions

| Question | Decision |
|---|---|
| Split on which `=`? | **The first**, and only for tokens starting with `-`. Values legitimately contain `=` — `-H "Authorization: Basic YWJj=="` and `get "/a?x=1&y=2"` both work today and must keep working. Gating on the leading `-` keeps the positional URL path untouched |
| Short flags too? | **Yes** — `-c=path` as well as `--config=path`. apimock has no clustered short flags, so there is no ambiguity, and teaching one form while rejecting the other sets a trap |
| `--text=` with nothing after? | **An explicit empty value**, distinct from a dangling `--text` (a usage error). Precedent exists: `--text ""` works today, exit 0 |
| Repeatable flags? | `--header=A: 1 --header=B: 2` must work — `flag_values_all` gets the same treatment |

### Rejected alternatives

| Option | Why not |
|---|---|
| `--` end-of-flags separator | Doesn't fit — `--` conventionally ends *all* flag parsing; it does not deliver a value to a preceding flag |
| Relax `!starts_with('-')` for content flags only | **Actively worse.** `--text --dry-run` would swallow `--dry-run` as text, reintroducing the silent-wrong-behaviour class this RFC just closed |
| Do nothing | Leaves the gap above, with a misleading error, on v6's headline feature |

## Unresolved questions

1. **Is exit `1` reachable outside `match-test`'s "no match"?** I could
   not reach it. RFC 053 names it *"everything else"*. If nothing emits
   it, either something should, or the contract should stop describing
   it — a code nothing produces is a contract that misdescribes itself.
   Establish before documenting.
2. **Should a repeated flag with one value missing error?**
   `--header a:b --header` — the first value is usable. Erroring is
   simpler to explain and matches Goal 2; accepting it silently is the
   behaviour this RFC exists to remove. **Recommend erroring**, and say
   so in the reference.
