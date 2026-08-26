# Acceptance / QA Checklist — RFC 064

**Governing RFC.** [RFC 064](../../accepted/064-cli-front-door-completion.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Every row is a claim a reviewer will re-run. Tick it only if you ran it.

---

## 0. Fixture — get this right or the results are meaningless

```
$ cd "$(mktemp -d)"
$ apimock --init --yes        # writes apimock.toml + apimock-rule-set.toml
```

**Use this fixture, not a hand-written one.** Defect 1 only appears with
a config that loads far enough to resolve a path relative to its own
directory (`rule_sets`). A minimal or invalid config errors earlier and
shows the bug as absent. This cost me a wrong reading while writing the
RFC.

- [ ] Fixture built with `--init --yes`; `apimock.toml` contains `rule_sets`.
- [ ] Exit codes captured from the binary directly — **not through a
      pipe**. `apimock … | head` reports `head`'s status, not apimock's.

---

## 1. Bare relative `--config` (Defect 1)

Run from the fixture directory. All must be exit 0 and behave
identically to their `./`-prefixed twin.

- [ ] `apimock validate -c apimock.toml` → exit 0
- [ ] `apimock get /x -c apimock.toml` → exit 0
- [ ] `apimock set rule --path /a --status 418 -c apimock.toml` → exit 0, writes
- [ ] `apimock match-test --rule-set apimock-rule-set.toml` → runs
- [ ] Each of the above is byte-identical in output to the `./` form
      (modulo the path string echoed back)

Non-regression:

- [ ] `-c ./apimock.toml` still works on all four
- [ ] `-c ../<dir>/apimock.toml` works from a subdirectory
- [ ] An absolute `-c /abs/path/apimock.toml` works
- [ ] A genuinely missing file still errors, and the message names the
      **file**, not an empty path
- [ ] `crates/apimock/tests/args.rs` — the whole file still passes,
      **including** `config_flag_with_no_value_fails_the_same_way_as_before`
      (line 280), which pins behaviour deliberately

---

## 2. Optional flag with no value (Defect 2)

For **every** optional value-taking flag on **every** command, in
**both** dangling forms:

- flag at the end of the argument list
- flag immediately followed by another flag

Expected in all cases: **exit 2**, `usage` error, message on **stderr**,
nothing on stdout, no side effects.

- [ ] `get`: `-c`, `--method`, `--header`, `--body`, `--body-file`, `--format`
- [ ] `set rule`: `-c`, `--rule-set`, `--rule`, `--path`, `--method`,
      `--header`, `--status`, `--json`, `--text`, `--file`, `--delay`, `--format`
- [ ] `validate`: `--format`
- [ ] `match-test`: `--rule`, `--path`, `--method`, `--header`,
      `--body`, `--body-file`, `--format`
- [ ] Driven from a table, one row per flag per command — **not**
      hand-picked examples. A hand-picked subset is how this was missed
      the first time.

### 2a. The two that matter most — assert on effects, not exit codes

- [ ] **`apimock set rule --path /x --status 418 -c` writes nothing.**
      Assert on **file contents** before and after. An exit-code-only
      assertion would have passed while it silently wrote — that is
      precisely what happened.
- [ ] **`--format` dangling never produces text output with exit 0**, on
      `get`, `validate` and `set`. This is the silent-failure case for
      the CLI's primary user.

### 2b. Required flags — unchanged

- [ ] `apimock validate -c` → exit 2, message still exactly
      *"missing required flag --config / -c"*
- [ ] `apimock match-test --rule-set` → exit 2, message still exactly
      *"--rule-set <path> is required"*
- [ ] Both strings pinned by a test, so a later refactor cannot degrade
      them into a generic error

### 2c. Values that legitimately start with `-`

- [ ] Establish and record what happens for e.g. `--text -5` or a
      negative `--delay`. If such values are unreachable, say so; if the
      fix makes a previously-working invocation fail, that is a
      regression and must be called out, not absorbed.

---

## 3. Conformance suite

- [ ] `get_has_no_known_flag_missing_value_scenario()` replaced with real rows
- [ ] `set_has_no_known_flag_missing_value_scenario()` replaced with real rows
- [ ] No remaining empty test asserting a scenario does not exist,
      anywhere in `cli_conformance.rs`, without evidence behind it
- [ ] Every exit code documented in the CLI reference is asserted by at
      least one test

---

## 4. Documentation

`docs/src/reference/cli-reference.md`:

- [ ] The flags-table claim *"`-c apimock.toml` and `-c ./apimock.toml`
      are equivalent"* is now **true for every command** — verified, not
      deleted
- [ ] § Exit codes rewritten against measured behaviour
- [ ] The paragraph claiming the flag scan cannot distinguish the two
      cases is **removed** (not softened)
- [ ] An exit code is stated for each `error.kind`: `usage` 2,
      `config_invalid` 2, `config_unreadable` 2, `io` 1, `conflict` 1,
      `internal` 1
- [ ] No other page contradicts the new behaviour — grep the docs for
      `-c ` and for `exit`
- [ ] `mdbook build docs` clean

---

## 5. Gates

- [ ] `cargo test --workspace` — all green, count reported
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo audit`
- [ ] CI green on all 8 jobs (Linux/macOS/Windows — RFC 061)

---

## 6. Report back

In `.git-exclude/review-request/064-cli-front-door-completion/`:

- [ ] Entry-point document
- [ ] **Answer to § 3.1 of the handoff**: is exit `1` reachable outside
      `match-test`'s "no match"? Evidence either way.
- [ ] **Answer to § 3.2**: repeated flag with one value missing — your
      decision and why. (My recommendation: error.)
- [ ] Anything from § 2c that changed behaviour for values starting `-`
- [ ] Any place where you found the RFC's measured claims to be wrong.
      Two of my initial readings were wrong before I re-measured; say so
      plainly if a third was.
