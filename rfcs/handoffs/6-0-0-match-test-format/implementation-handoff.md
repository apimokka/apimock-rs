# Implementation Handoff — 6.0.0: `match-test --format` is undiscoverable

**Governing RFC.** [RFC 059](../../done/059-cli-contract-conformance.md);
[RFC 048](../../done/048-v6-cli-interface-concept.md) § 5.
**Milestone.** 6.0.0. **Blocking the cut.**
**Baseline.** `main` @ `a0a3f65`.
**Found.** Writing the 6.0.0 CHANGELOG, checking a migration-guide claim
before repeating it.

**Self-contained.** Small — one flag, two surfaces.

---

## 1. The gap

`apimock match-test --format json` **works**:

```
$ apimock match-test --rule-set ./apimock-rule-set.toml --path /hello --format json
{
  "apimock": "…",
  "result": { "matched": false, … }
}
$ echo $?
1
```

It is specified by RFC 059, and `docs/src/guides/migrating-to-6-0.md`
advertises it — *"6.0.0 adds `--format json` to it"*.

**It appears in neither `match-test --help` nor the CLI reference.**

| Surface | Lists `--format`? |
|---|---|
| `crates/apimock/src/cmd/match_test.rs`'s `USAGE` constant | **yes** |
| `crates/apimock/src/args.rs`'s `help_text` `Some("match-test")` arm | **no** |
| `docs/src/reference/cli-reference.md` § `apimock match-test` | **no** — zero occurrences |
| The migration guide | claims it exists |

`get`, `set` and `validate` all advertise their own `--format`
correctly. **This is `match-test` only.**

## 2. Why this blocks a release rather than waiting for 6.0.1

RFC 048 § 5: *"the documented contract is the only thing an agent's
author can build against."*

`--format json` exists **for** agents — `match_test.rs`'s own module doc
says the problem it solved was that *"an agent driving it had to scrape
text"*. Shipping it discoverable only by reading the source or the
migration guide reproduces the problem it was added to fix.

Nothing misbehaves. This is a discoverability defect, not a correctness
one — but it is on the machine-readable surface of an agent-facing
command, in a release whose theme is that surface.

## 3. A correction to the previous package, stated plainly

`.git-exclude/review-request/6-0-0-serve-and-help/README.md` § 3 says:

> *"Checked the other three subcommands' `USAGE` constants against their
> `help_text` arms too (`get.rs`, `validate.rs`, `match_test.rs`) — no
> equivalent drift found in any of them."*

**There is drift in `match_test.rs`**, and it is the same shape as the
F-1 you fixed: `USAGE` lists `[--format text|json]`, `help_text` does
not.

Not a criticism of the fix you shipped — F-1 was correct and so was
checking the neighbours. The check simply missed one. Worth knowing
because the same comparison is the natural way to look for the next
instance, and it can return a false negative.

**When you fix this, re-run that comparison mechanically rather than by
eye** — diff each `USAGE` constant's flag list against its `help_text`
arm's, for all four subcommands, and report the result. If it is clean
afterwards, say so; that closes the class rather than the instance.

## 4. What to change

- `crates/apimock/src/args.rs` — `help_text`'s `Some("match-test")`
  arm: add `--format text|json` to the synopsis line **and** the flag
  list, matching how `get`/`set`/`validate` present theirs.
- `docs/src/reference/cli-reference.md` § `apimock match-test` —
  document `--format`, consistent with the other three sections,
  including what `--format json` emits.

Check the exit-code line stays accurate: `match-test` uses `1` for "no
rule matched", which is its own success/failure axis. `--format json`
must not change that.

## 5. Not in scope

- Any behaviour change. `--format` already works; this is help and docs.
- Any other flag or command — unless § 3's mechanical re-check finds
  one, in which case **report it**; do not fix it here without saying so.
- Version bump, CHANGELOG, README notice — RFC 066 § 2.

## 6. Acceptance

- [ ] `match-test --help` lists `--format text|json`, in the same style
      as the other three
- [ ] The CLI reference's `match-test` section documents it
- [ ] `--format json` and `--format text` both still behave as today —
      captured from real runs
- [ ] `match-test`'s exit codes unchanged (`0` matched, `1` no match,
      `2` argument error) under both formats
- [ ] § 3's mechanical `USAGE`-vs-`help_text` comparison re-run for all
      four subcommands, result reported
- [ ] `cargo test --workspace`, `fmt`, `clippy -D warnings`,
      `mdbook build docs`
- [ ] CI green on all 9 jobs before merge

## 7. Report back

`.git-exclude/review-request/6-0-0-match-test-format/`, including § 3's
comparison result for all four subcommands.
