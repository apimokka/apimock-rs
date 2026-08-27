# Implementation Handoff — 6.0.0: an unknown subcommand must not start a server

**Governing RFCs.** [RFC 048](../../accepted/048-v6-cli-interface-concept.md)
§ 6; [RFC 059](../../accepted/059-cli-contract-conformance.md);
[RFC 064](../../accepted/064-cli-front-door-completion.md).
**Milestone.** 6.0.0. **Blocking the cut.**
**Baseline.** `main` @ `ed42fc0`.
**Found.** During 6.0.0 release preparation, verifying a claim in the
migration guide.

**Self-contained.** Everything binding is restated here.

---

## 1. The defect

**Any bare word in the subcommand position silently starts a server.**
Measured on `main`:

| Invocation | Today |
|---|---|
| `apimock banana` | **starts a server** on port 3001 |
| `apimock gte /x` (typo for `get`) | **starts a server** |
| `apimock validat -c x.toml` (typo for `validate`) | **starts a server** |
| `apimock set-rule …` | **starts a server** |
| `apimock serve` | **starts a server** |

None errors. Each runs until killed.

**For the CLI's primary user this is the defining failure.** U2 is
defined by failing silently; an agent invoking `apimock gte /x --format
json` does not get exit 2 and a suggestion — it gets a **hung process**
and no output it can parse. Nothing tells it the subcommand was
misspelled.

This is the same class RFC 059 closed for unknown *flags* and RFC 064
closed for *dangling* flags, on the same command, left open for
*subcommands*. `docs/src/reference/cli-reference.md` already promises
*"Anything starting with `-` that isn't one of the flags documented on
this page is an error, not silently ignored"* — a bare word is not
covered by that sentence, and is not covered by the code either.

### Root cause

`crates/apimock/src/args.rs:87–104` matches each subcommand by exact
string:

```rust
if raw.get(1).map(String::as_str) == Some("match-test") { … }
if raw.get(1).map(String::as_str) == Some("validate")   { … }
if raw.get(1).map(String::as_str) == Some("get")        { … }
if raw.get(1).map(String::as_str) == Some("set")        { … }
reject_unknown_arguments(&raw);   // ← bare tokens allowed here
```

Anything that is not one of the four falls through. `reject_unknown_
arguments` calls the shared `reject_unknown_flags` with
`strict_bare_tokens: false` — correct for `get`, whose positional
`<path>` depends on it, and wrong here: **the root command has no
positional argument at all.**

## 2. The fix

After the four exact matches, a **bare token** in position 1 — one that
does not start with `-` — is an unknown subcommand and must be a usage
error.

| | |
|---|---|
| Exit code | **2** |
| Stream | **stderr**; nothing on stdout |
| Message | Name the unknown subcommand, and **suggest a near match** where one exists |
| Effect | **No server started.** Assert this on the process, not just the exit code |

Reuse `crate::args::near_match` — the same machinery RFC 059 uses for
flags — against the four subcommand names. `gte` → `get` and `validat`
→ `validate` should both suggest; `banana` should not, and gets the
plain form.

**Must keep working, unchanged:**

- Bare `apimock` with no arguments — zero-config server. **This is not
  a bare token in position 1; there is no position 1.**
- `apimock -c x.toml`, `-p`, `-d`, `--init`, `--version`, `--help` —
  position 1 starts with `-`, so it is a flag, not a subcommand attempt.
- All four real subcommands, and `apimock <sub> --help`.

## 3. `apimock serve` does not exist — and the docs say it does

`docs/src/guides/migrating-to-6-0.md:56` states:

> *"Bare `apimock` keeps working as an alias for `apimock serve`."*

**There is no `serve` subcommand.** `apimock serve` "works" only by the
defect above — it is an ignored bare token. After this fix it will
error, correctly.

Fix the sentence to say what is true: bare `apimock` starts the server;
there is no `serve` subcommand. **Do not add one** — that is a feature,
not a correction, and is out of scope.

This is the sentence that led me to the defect. It is on the page a
5.19 user reads while upgrading.

## 4. Search, don't trust this list

*(A correction to how I write these: three times in this series my file
enumerations have read as complete and were not.)*

The named files are a **floor, not a ceiling**:

- `crates/apimock/src/args.rs` — the dispatch and `reject_unknown_arguments`.
- `docs/src/guides/migrating-to-6-0.md:56` — § 3.
- `docs/src/reference/cli-reference.md` — § "Unrecognised arguments"
  currently describes only `-`-prefixed tokens; extend it to subcommands.

**Then grep the tree** for `serve`, and for any doc, example README,
help text or test that assumes an unknown bare token is tolerated.
Report what you find, including anything you decide needs no change.

## 5. Not in scope

- Adding a `serve` subcommand.
- Any other command's parsing. `get`'s positional `<path>` **must**
  keep `strict_bare_tokens: false` — do not make it strict to be
  consistent.
- Version bump, CHANGELOG, README notice — release mechanics, and
  **not the dev team's** (RFC 066 § 2).

## 6. Acceptance

- [ ] `apimock banana` → exit 2, stderr names it, **no server started**
- [ ] `apimock gte` → exit 2, **suggests `get`**
- [ ] `apimock validat` → exit 2, **suggests `validate`**
- [ ] `apimock serve` → exit 2 (no longer starts a server)
- [ ] **No server is started in any of the above** — assert on the
      process, not the exit code. A test that only checks exit 2 would
      pass while a server leaked
- [ ] Bare `apimock` still starts the zero-config server
- [ ] `apimock -c <path>` / `-p` / `-d` / `--init` / `--version` /
      `--help` all unchanged
- [ ] All four subcommands and `<sub> --help` unchanged
- [ ] `get`'s positional `<path>` unchanged — pin it
- [ ] `crates/apimock/tests/args.rs` passes; note anything changed
- [ ] Migration guide and CLI reference corrected (§ 3, § 4)
- [ ] `cargo test --workspace`, `fmt`, `clippy -D warnings`,
      `mdbook build docs`
- [ ] CI green on all 9 jobs before merge

## 7. Report back

`.git-exclude/review-request/6-0-0-unknown-subcommand/`, including the
**exact stderr and exit code** for `banana`, `gte` and `serve`, captured
from real runs, and the § 4 sweep results.
