# Implementation Handoff — RFC 059, CLI contract conformance

**Governing RFC.** [RFC 059](../../done/059-cli-contract-conformance.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0 — **blocking**
**Self-contained.** Every fact you need is restated here, including RFC
053's taxonomy. RFC 059 is the authority; if the two disagree, report it
rather than following this.

---

## 1. The defect, measured — reproduce it before you fix it

Against the current binary, in a directory with a valid config:

| Command | `--bogus` | Should be |
|---|---|---|
| `set` | exit **2** ✅ | 2 |
| `get` | exit **0** ❌ | 2 |
| `validate` | exit **0**, prints *"Validation passed"* ❌ | 2 |
| `match-test` | exit **1** ❌ — for an unrelated missing-argument reason | 2 |

**The case that made this blocking:**

```sh
$ apimock validate -c apimock.toml --strct    # typo of --strict
$ echo $?
0
```

`--strict` exists to make CI fail. Typo it and the job passes, having
silently run non-strict.

Reproduce all four yourself first. Everything after this is easier once
you have seen `validate` cheerfully report success on an invocation it
did not understand.

## 2. Why this is not four parser edits

`get_format.rs` (9 tests), `set_format.rs` (14), `validate_format.rs`
(9) and `args.rs` (9) each test their own command. **No test asserts a
rule that must hold across all four.** A rule implemented once and
forgotten three times looks locally fine everywhere.

Each of those files also defines its **own** `bin()`, `run_json()` and
`run_stderr()`. There is no shared harness, so a cross-command
assertion has nowhere to live.

Fix the four parsers and you fix today's bug. Add the table and the next
command cannot reintroduce it.

## 3. The contract, restated in full

### Exit codes

`0` success · `2` usage error · `1` everything else. Diagnostics to
stderr; stdout carries only the result.

### The envelope

```json
{ "schema": 1, "apimock": "6.0.0", "result": { … } }
{ "schema": 1, "apimock": "6.0.0", "error": { "kind": "…", "message": "…" } }
```

Object, never a bare array. Exactly one of `result` / `error`.
`crates/apimock/src/cmd/envelope.rs` already produces this — use it.

### `error.kind` — closed set

| `kind` | Meaning | Exit |
|---|---|---|
| `usage` | Bad invocation — unknown option, missing value | **2** |
| `config_invalid` | Configuration read but not valid | **2** |
| `config_unreadable` | Configuration missing or unreadable | **2** |
| `io` | Filesystem failure that is not the config | 1 |
| `conflict` | State changed underneath — `set` only | 1 |
| `internal` | A bug in apimock | 1 |

**Note the two exit `2` rows.** RFC 053 originally said `1` for both.
The code has always said `2`, and **RFC 053 was amended on 2026-08-20 to
match the code** — the table was wrong, not the implementation. Do not
"fix" these to `1`.

## 4. The three open questions, decided

### Does `match-test` join the envelope? **Yes.**

`match-test` has **no `--format` support at all** today — it is the only
command outside RFC 053. Add `--format text|json`, emitting the same
envelope via the same helper, exactly as `get` and `validate` do.

**This is the one scope decision in this handoff, and it is mine, so
flag it if you think it is wrong.** The reasoning: 6.0.0's headline is a
CLI an agent can rely on, and shipping it with one of four commands an
agent cannot parse makes the claim false for a quarter of the surface.
It is additive — text output is unchanged and stays the default.

### Is the near-match suggestion mandatory? **Yes — assert it.**

`--strct` must produce *"did you mean `--strict`?"*.
`reject_unknown_arguments` already has `near_match`; wire every command
to it. The table asserts the suggestion, not just the exit code — a
rejection without it is correct and useless.

### The `config_invalid` contradiction? **Settled — see § 3.** The
table now says `2`, matching the code. Nothing for you to decide.

## 5. The work

**a. One shared harness.** A single test-support module with `bin()`,
`run_json()`, `run_stderr()`. Delete the four duplicated copies; the
existing per-command tests keep their assertions and just use it.

**b. Fix the three commands.** `get`, `validate` and `match-test` reject
unknown flags as `usage` / exit `2`, with the suggestion.

**c. The conformance table.** One module driving the real binary over
(command × scenario):

| Scenario | Expected |
|---|---|
| unknown flag | `usage`, exit 2, stderr, near-match suggestion |
| known flag, missing value | `usage`, exit 2, stderr |
| mutually exclusive flags | `usage`, exit 2, stderr |
| config missing | `config_unreadable`, exit 2 |
| config malformed | `config_invalid`, exit 2 |
| success | exit 0, stdout only |

Where a command genuinely has no such scenario, **say so in the table
explicitly** — an omitted row and an inapplicable row must not look the
same.

**d. `match-test --format json`** (§ 4).

## 6. Evidence required

- All four repro commands from § 1 exit `2` with kind `usage`, and
  **nothing on stdout**.
- **`validate --strct` exits 2 and suggests `--strict`.** The regression
  test for the case that motivated this RFC.
- The conformance table passes for every command × scenario.
- **Prove the table can fail.** Temporarily make one command ignore
  unknown flags again, confirm the suite catches it, restore. Report
  that you ran this — a conformance suite that cannot fail is decoration,
  and this is the one check that distinguishes the two.
- `match-test --format json` emits a valid envelope; its text output is
  byte-identical to before.
- The four duplicated helper sets are gone; every pre-existing test still
  passes unmodified.
- Full suite green with the count against `main`'s baseline;
  `cargo fmt --all --check`; `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`.

## 7. Escalation

Blocking issues and design questions go in a
`.git-exclude/review-request/` package.

Escalate if: adding `--format` to `match-test` turns out to be more than
additive (§ 4 assumes it is); a command has a scenario the table cannot
express; or fixing a parser changes behaviour for an invocation that was
previously *correct* — that last one would mean the fix is too broad,
and I want to hear about it before it lands.
