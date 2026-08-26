# Implementation Handoff — RFC 064, finish the CLI front door

**Governing RFC.** [RFC 064](../../accepted/064-cli-front-door-completion.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0. **Blocking for the release cut.**
**Baseline.** `main` @ `9bdc769`.

**Self-contained.** Everything you need is here. You do not need to read
RFC 048, 049, 053 or 059 to do this work — the parts that bind you are
restated below.

---

## 1. What is wrong

Two defects, both on the subcommand argument path, both measured against
a debug binary built from `9bdc769` in a scratch directory populated by
`apimock --init --yes`.

### Defect 1 — a bare relative `--config` fails on subcommands

```
$ apimock validate -c apimock.toml
apimock validate: failed to load config: failed to resolve path
`apimock.toml`: No such file or directory (os error 2)     [exit 2]

$ apimock validate -c ./apimock.toml
Validation passed (2 rules across 1 rule set(s)).          [exit 0]
```

The file exists in both runs. Same on `get`.

> ### ⚠️ Reproduce it with a config that actually loads
>
> The failure needs a config that gets far enough to resolve a path
> **relative to its own directory** — i.e. one with `rule_sets`, such as
> what `--init` writes. A config that fails to parse errors earlier and
> never reaches the failing call, so a throwaway fixture will show the
> bare form "working" and send you the wrong way.
>
> I hit exactly this while preparing the RFC: an invalid fixture made
> the bug look absent. Use `apimock --init --yes` for your fixture.

**Root cause — `crates/apimock-config/src/path_util.rs:19`:**

```rust
let parent = Path::new(file_path).parent().ok_or_else(…)?;
relative_path(env::current_dir()?.as_path(), parent)
```

`Path::new("apimock.toml").parent()` returns **`Some("")`**, not `None`.
The guard never fires; `relative_path` then calls
`fs::canonicalize("")`, which fails with `ENOENT`. The resulting error
names the *config file*, so it reads as "your config is missing" when
the config is fine and the empty string is what failed to resolve.

The doc comment above that function claims `Path::parent()` returns
`None` for empty paths. It does not, for a bare filename. Fix the
comment along with the code.

**This has been patched around twice already, never fixed:**

| Where | What happened |
|---|---|
| `crates/apimock/src/args.rs:397` `normalize_bare_relative_path` | RFC 049 added it — prepends `./` when a path has no directory component |
| `crates/apimock/src/args.rs:187` | …but it is applied **only on the root/server parser** |
| `crates/apimock/src/cmd/set.rs` | RFC 057 worked around the same problem a third time by keeping a `./` prefix internally |

The subcommands (RFC 055's `get`, RFC 057's `set`) parse flags through
`cmd/flags.rs` and never picked it up.

### Defect 2 — an optional flag with no value is silently defaulted

Measured. The split is sharp:

| | Dangling flag | Result |
|---|---|---|
| **Required — already correct, do not change** | `validate -c` | exit 2, *"missing required flag --config / -c"* |
| | `match-test --rule-set` | exit 2, *"--rule-set <path> is required"* |
| **Optional — silently defaulted** | `get … --method` / `--header` / `--body` / `--format` | **exit 0**, ignored |
| | `validate … --format` | **exit 0**, ignored |
| | `match-test … --path` / `--method` | exit 1 — defaulted, then "no match" |
| | `set rule … -c` | **exit 0**, and **writes to a config the caller never named** |

**Root cause — `crates/apimock/src/cmd/flags.rs:8`:**

```rust
pub(super) fn flag_value(args: &[String], names: &[&str]) -> Option<String> {
    let idx = args.iter().position(|a| names.iter().any(|n| a == n))?;
    args.get(idx + 1).filter(|v| !v.starts_with('-')).cloned()
}
```

`Option<String>` cannot express three states. "Flag present, no value"
collapses into `None`, which is indistinguishable from "flag absent", so
every caller takes its default. `flag_values_all` (line 15) drops it the
same way.

**Two consequences worth understanding before you fix it:**

**`set` writes to the wrong file.** Verified:

```
$ apimock set rule --path /dangling --status 418 -c
Applied:
  rule set: apimock-rule-set.toml (new rule)                [exit 0]
$ diff BEFORE.snapshot apimock-rule-set.toml
*** the rule was appended ***
```

The caller said "use this config". apimock used a different one and
reported success.

**`--format` is the case that matters most.** The CLI's primary user is
an AI agent, and that user is defined by failing *silently*. An agent
asking for `--format json` that loses the value gets **human text, exit
0** — no error, no RFC 053 envelope, no signal at all. This is the most
likely way the bug is met in practice.

---

## 2. What to change

### 2.1 `crates/apimock-config/src/path_util.rs`

Map "no directory component" to "the current directory":

```rust
let parent = Path::new(file_path).parent().unwrap_or_else(|| Path::new(""));
let parent = if parent.as_os_str().is_empty() { Path::new(".") } else { parent };
```

Fix at this layer, not per caller — it fixes all four subcommands, the
root parser and every future caller at once, which is the whole point
after three separate patch-arounds. Update the misleading doc comment.

**Leave `normalize_bare_relative_path` alone.** It becomes redundant but
stays correct, and `crates/apimock/tests/args.rs:280`
(`config_flag_with_no_value_fails_the_same_way_as_before`) deliberately
pins its empty-string behaviour. Keep that test green; do not "clean up"
around it in this RFC.

### 2.2 `crates/apimock/src/cmd/flags.rs`

Give `flag_value` a third outcome:

| Input | Today | Wanted |
|---|---|---|
| flag absent | `None` | absent |
| `-c path` | `Some(path)` | `Some(path)` |
| `-c` at end of args, **or** followed by another flag | `None` | **usage error** |

**Return a `Result`.** `flag_present` already exists, so you *could*
distinguish presence from value caller-by-caller without touching the
signature — don't. Across 26 call sites that invites exactly the
omission this RFC exists to fix. Make the compiler enumerate them.

Apply the same to `flag_values_all`.

**Closed set of call sites — 26, measured; there are no others:**

| File | Lines |
|---|---|
| `crates/apimock/src/cmd/set.rs` | 174, 175, 176, 182, 183, 184, 195, 201, 206, 210, 211, 220 |
| `crates/apimock/src/cmd/get.rs` | 86, 87, 90, 99, 100, 105 |
| `crates/apimock/src/cmd/match_test.rs` | 174, 177, 189, 190, 194, 204, 205, 208 |
| `crates/apimock/src/cmd/validate.rs` | 72, 76 |

**Keep the existing required-flag messages.** `validate -c` and
`match-test --rule-set` already produce better errors than a generic one
would. The new error is for the optional flags that currently say
nothing at all.

**Error shape** — this is a `usage` error and must match what the rest
of the CLI already does:

- exit code **2**
- `error.kind` = **`usage`** in the RFC 053 JSON envelope
  (`{ schema, apimock, error }`)
- message to **stderr**; nothing on stdout
- no server started, no file written

### 2.3 `crates/apimock/tests/cli_conformance.rs`

Two tests currently assert this defect does not exist:

- `get_has_no_known_flag_missing_value_scenario()`
- `set_has_no_known_flag_missing_value_scenario()`

Both are empty and both claims are false. Replace them with real rows.

For context, so this doesn't read as blame: RFC 059 asked for
inapplicable scenarios to be **stated rather than silently omitted**,
and writing them was the right response to that instruction. The
instruction was right; the claim written under it was not checked — by
you or by me. The lesson is that an explicit "not applicable" needs the
same evidence as an assertion.

### 2.4 `docs/src/reference/cli-reference.md`

Four corrections:

1. **The flags table already claims Defect 1 is fixed** — *"`-c
   apimock.toml` and `-c ./apimock.toml` are equivalent"*. True for the
   server, false for every subcommand. After your fix it becomes true
   everywhere: **verify it, don't delete it.**
2. **§ Exit codes documents exit `1` for a dangling flag.** Measured:
   `2` for required, `0` for optional. Rewrite against real behaviour.
3. **Delete the paragraph claiming the flag scan cannot tell the two
   cases apart** (*"telling them apart would mean changing that scan,
   which every other flag's exact behaviour depends on staying
   untouched"*). It is wrong, and the code disproves it —
   `reject_unknown_flags(args, known, no_value)` is already **passed the
   closed set of no-value flags** for this exact purpose. Do not soften
   it into a caveat; remove it.
4. **State the exit code for every `error.kind`.** The closed set from
   RFC 053, restated so you need not go looking:

   | `error.kind` | Exit |
   |---|---|
   | `usage` | 2 |
   | `config_invalid` | 2 |
   | `config_unreadable` | 2 |
   | `io` | 1 |
   | `conflict` | 1 |
   | `internal` | 1 |

---

## 3. Two open questions — answer them, don't guess

Both are flagged in the RFC as unresolved. **Report your findings in the
review-request package rather than deciding silently.**

1. **Is exit `1` reachable outside `match-test`'s "no match"?** I could
   not reach it. RFC 053 defines it as "everything else". If nothing
   emits it, either something should or the contract should stop
   describing it — but establish which before documenting it.
2. **Should a repeated flag with one value missing error?**
   `--header a:b --header` — the first value is usable.
   **My recommendation: error.** It matches the goal and is simpler to
   explain than a partial-acceptance rule. If you find a reason to
   accept it instead, say so explicitly rather than implementing it
   quietly.

---

## 4. What is NOT in scope

- **RFC 045 Goal 4** (`validate` passing on inert configuration). It is
  the fourth open RFC 048 § 6 prerequisite, and it is an **owner
  decision**, not implementation work. Do not touch it.
- Rewriting argument parsing. RFC 049's scan stays.
- Changing which flags exist, or any command's success output.
- Changing required-flag handling — it is already correct.
- Making exit `1` reachable (see § 3.1 — investigate and report only).

---

## 5. Definition of done

- Every item in [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md) passes.
- `cargo test --workspace` green; `cargo fmt --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`;
  `mdbook build docs`.
- Review-request package in
  `.git-exclude/review-request/064-cli-front-door-completion/` with an
  entry-point document, per the usual convention, including your answers
  to § 3.
