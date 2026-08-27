# Handoff — RFC 064 Amendment 1, a `--flag=value` form

**Governing RFC.** [RFC 064, Amendment 1](../../accepted/064-cli-front-door-completion.md#amendment-1--a---flagvalue-form-approved-2026-08-27)
**Risk brief.** `.git-exclude/reviewed/pre-6.0.0-audit/DECISION-001-flag-equals-value.md`
**Milestone.** 6.0.0, **blocking**.
**Baseline.** `main` @ `87d2584`.

**Self-contained.** Everything binding is restated here. You do not need
to re-read RFC 064's original body, RFC 059 or RFC 062.

> ## 🔒 Read this before writing any code
>
> This change touches **`--allow-outside`**, which is RFC 062's
> **write-path confinement opt-out**. Get the rule in § 2 wrong and
> `apimock set` writes outside the workspace on an invocation that
> explicitly asked it not to.
>
> **Today's behaviour is safe** — every `=` form is rejected. So this is
> a regression to avoid, not a hole to close, and **a partial
> implementation is strictly worse than shipping nothing.** If you can't
> land § 2 and § 3 together, land neither and say so.

---

## 1. The gap being closed

No flag value can begin with `-`, by any means:

| Attempt | Today |
|---|---|
| `--text "- item"` | exit 2 — `unrecognized argument '- item'` |
| `--text=-hello` | exit 2 — `unrecognized argument '--text=-hello'` |
| `--text -- -hello` | exit 2 — `unknown option '--'` |

`reject_unknown_flags` rejects any token starting with `-` before
`flag_value` sees it, and there is no `--flag=value` form and no `--`
separator. So `apimock set rule --text` cannot express a markdown
bullet, a YAML `---`, or a diff hunk.

Add a `--flag=value` form. That is the whole feature.

## 2. The security rule — a hard acceptance gate

**A no-value (boolean) flag given any `=` form is a usage error,
exit `2`.** Never "present".

Why it matters: `flag_present` matches the flag name exactly and
**discards any value**. If `--allow-outside=false` resolves to "flag
present", confinement is disabled while the author wrote `false` to keep
it on — at exit 0, with no diagnostic.

**Reject `=true` exactly as you reject `=false`.** Accepting one and not
the other is the asymmetry someone later "simplifies" into accepting
both. There is no `--flag=bool` feature here; there is only rejection.

The closed sets already exist in the code — this is a lookup, not a
judgement call:

| Command | Constant | No-value flags |
|---|---|---|
| `get` | `cmd/get.rs:47` | `--why` |
| `set` | `cmd/set.rs:86` | `--dry-run`, **`--allow-outside`** |
| `validate` | `cmd/validate.rs:56` | `--strict`, `--quiet`, `--json` |
| `match-test` | `cmd/match_test.rs:56` | `--quiet` / `-q` |
| root | `args/constant.rs` | `--init`, `--middleware`, `--yes`/`-y`, `--version`, `--help`/`-h` |

Severity differs and is worth knowing so you weight your own testing —
but **all of them are rejected**, regardless of severity:

| Flag | If mis-parsed | Severity |
|---|---|---|
| `--allow-outside` | RFC 062 write confinement disabled | **Severe** |
| `--dry-run` | Nothing written when the caller expects a write | Moderate |
| `--yes` | Prompts skipped, defaults accepted. Cannot overwrite an existing config — `--init` refuses regardless (verified) | Moderate |
| the rest | Output/verbosity only | Low |

## 3. Consolidate first, then add the form

The same skip-next rule exists **three** times:

| # | Location | Serves |
|---|---|---|
| 1 | `crates/apimock/src/args.rs:340` | root command |
| 2 | `crates/apimock/src/cmd/flags.rs:80` `reject_unknown_flags` | `get`, `validate`, `match-test` |
| 3 | **`crates/apimock/src/cmd/set.rs:127`** — private duplicate | **`set`** |

RFC 059 left #3 as its own copy on purpose (*"it predates this one and
there was no reason to touch working code"*). That was sound then and is
not now: **the one command carrying `--allow-outside` is the one that
does not share the common parser.** A fix applied to `flags.rs` alone
leaves the security-relevant path on the old rule.

**Fold #3 into #2**, then add `=` handling once. Include #1 (the root
command) too — adding a form to two parsers out of three deepens exactly
the divergence that caused RFC 064's original defect, where
`normalize_bare_relative_path` was fixed in one place and patched around
twice more.

If consolidating #3 turns out to change `set`'s behaviour in any way
beyond this amendment, **stop and report** rather than absorbing it —
`set`'s parser predates the shared one and may differ in ways neither of
us has noticed.

## 4. Settled decisions — implement these, don't re-litigate

| Question | Decision |
|---|---|
| Which `=` to split on | **The first**, and only for tokens starting with `-`. Values legitimately contain `=`: `-H "Authorization: Basic YWJj=="` (base64 padding) and `get "/a?x=1&y=2"` both work today and must keep working. Gating on the leading `-` keeps the positional URL path untouched |
| Short flags | **Yes** — `-c=path` as well as `--config=path`. There are no clustered short flags, so no ambiguity; teaching one form while rejecting the other sets a trap |
| `--text=` (nothing after `=`) | **An explicit empty value** — distinct from a dangling `--text`, which is a usage error (RFC 064). Precedent: `--text ""` works today, exit 0 |
| Repeatable flags | `--header=A: 1 --header=B: 2` must work. `flag_values_all` gets the same treatment |
| Unknown flag with `=` | Still an unknown-flag error, and **keep the near-match suggestion working** — `--txt=x` should still suggest `--text` |

## 5. Acceptance checklist

### 5a. The gate — do this first

- [ ] **Every** no-value flag, on **every** command incl. root, given
      `=false`, `=true`, and `=` (empty): **exit 2, usage error**.
      Table-driven from the `NO_VALUE_FLAG_NAMES` constants, one row per
      flag per form — not hand-picked.
- [ ] **`apimock set rule … --allow-outside=false` does not write
      outside the workspace.** Assert on the filesystem, not the exit
      code. This is the one that matters.
- [ ] `--allow-outside=true` is *also* rejected.
- [ ] `--dry-run=false` rejected; the rule set is byte-identical after.

### 5b. The feature

- [ ] `set rule --text=-hello` writes a rule whose text body is
      `-hello`. Assert the **written file's contents**.
- [ ] `--text="- item"`, `--text=---`, and a diff-hunk value all work.
- [ ] `-c=./apimock.toml` and `--config=./apimock.toml` both work, on all
      four subcommands and the root command.
- [ ] `--text=` yields an empty body, exit 0 — matching `--text ""`.
- [ ] Dangling `--text` (no `=`, nothing after) is still exit 2.
- [ ] `--header=A: 1 --header=B: 2` yields two headers.

### 5c. Non-regression

- [ ] `-H "Authorization: Basic YWJj=="` still works (first-`=` split).
- [ ] `get "/a?x=1&y=2"` still works — the positional path is untouched.
- [ ] Every RFC 064 behaviour still holds: bare relative `-c`, dangling
      optional flags exit 2, `set rule … -c` writes nothing.
- [ ] `crates/apimock/tests/args.rs` passes unmodified, including
      `config_flag_with_no_value_fails_the_same_way_as_before` (line 280).
- [ ] Near-match suggestions still fire for `--txt=x`.
- [ ] Consolidating `set.rs`'s parser changed no `set` behaviour — state
      how you established this.

### 5d. Documentation

- [ ] `docs/src/reference/cli-reference.md`: the `=` form documented
      once, centrally, including that boolean flags reject it and why.
- [ ] The root/subcommand exit-code split text updated if consolidation
      changes it.
- [ ] `mdbook build docs` clean.

### 5e. Gates

- [ ] `cargo test --workspace`, `cargo fmt --check`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo audit`
- [ ] CI green on all 8 jobs before merge.

## 6. Not in scope

- Any other new flag form (`-cpath` clustering, `--` separator).
- Changing which flags exist, or any command's success output.
- RFC 045 Goal 4 — closed, see RFC 064.

## 7. Report back

`.git-exclude/review-request/064-amendment-flag-equals-value/`, entry
point document, including:

- [ ] Evidence for **5a's `--allow-outside=false`** row specifically.
- [ ] How you established § 5c's "consolidation changed nothing".
- [ ] Anything in § 4 you found reason to disagree with — say so rather
      than implementing around it.
