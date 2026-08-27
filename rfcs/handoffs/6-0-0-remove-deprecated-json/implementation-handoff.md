# Implementation Handoff — 6.0.0: remove the deprecated `validate --json`

**Governing RFCs.** [RFC 054](../../done/054-deprecation-release.md)
§ *"What 6.0.0 then does"*; [RFC 048](../../accepted/048-v6-cli-interface-concept.md)
§ 7 (the loud-failure requirement).
**Milestone.** 6.0.0. **Blocking the cut.**
**Baseline.** `main` @ `13f5ddf`.

**Self-contained.** Everything binding is restated here.

---

## 1. Why this exists

RFC 054 shipped 5.19.0 as the deprecation release. Its § *"What 6.0.0
then does"* is one sentence, and it is the only part of RFC 054 still
outstanding:

> **Removes `--json`.** Per RFC 048 § 7, the removal must **fail
> loudly** with a machine-readable error naming the replacement, never
> silently do something different.

Nobody picked it up. Measured on `main` today:

```
$ apimock validate -c ./apimock.toml --json
apimock validate: --json is deprecated and will be removed in 6.0.0.
  Use --format json, which emits the new response envelope.
[]
Validation passed (2 rules across 1 rule set(s)).              [exit 0]
```

**If 6.0.0 were tagged today**, that binary would print *"will be
removed in 6.0.0"* while being 6.0.0 — a message contradicting itself —
and three documentation pages that already say *"removed in 6.0.0"*
would be false.

This is also the **only** exercise of RFC 048 § 7's loud-failure
requirement in the whole release. `--json` is the sole invocation 5.x
promised to remove (RFC 054's own table; confirmed by sweep). If this
removal is silent or absent, that requirement ships untested.

## 2. The closed set

`--json` on `validate` only. Confirmed against RFC 054's table and by
grepping the tree — nothing else is promised for removal in 6.0.0.
`get` and `set` are new commands, so no 5.x invocation changes for them.

**Do not remove `--format json`.** That is the replacement and the only
supported shape from 6.0.0 on.

## 3. What "fail loudly" must mean here

RFC 048 § 7 requires a **machine-readable error naming its
replacement**. Concretely, matching the CLI's existing conventions:

| | |
|---|---|
| Exit code | **2** |
| `error.kind` (RFC 053 envelope, under `--format json`) | **`usage`** |
| Stream | **stderr**; nothing on stdout |
| Message | Must name **`--format json`** as the replacement |

**It must not** silently ignore the flag, and must not fall back to
`--format text`. Silence here is the exact failure RFC 048 § 7 was
written to prevent — U2 and U3 do not read release notes, so this error
message is the entire migration path for them.

Treat `--json` as **removed, not unknown**: a bare *"unknown option
'--json'"* from RFC 059's near-match machinery would technically be
loud, but it would not name the replacement, and near-match would
likely suggest nothing useful. A caller upgrading from 5.19 deserves to
be told what to write instead. Keep a specific message.

## 4. What to change

- `crates/apimock/src/cmd/validate.rs` — the `--json` code path,
  `JSON_DEPRECATION_WARNING` (line 68), and its emission (line 183).
  Replace with the removal error.
- `--json` leaves the known-flag list only if that yields a worse
  message than § 3 requires — decide by what the caller sees, and say
  which you chose and why.
- The `--json` / `--format` mutual-exclusion rule goes with it.
- `crates/apimock/src/args.rs:432` — the embedded `validate --help`
  text still documents `--json`.
- `crates/apimock/tests/validate_format.rs` — lines ~95 and ~126 assert
  the deprecation warning. They must now assert the removal behaviour.

## 5. Documentation

Three pages state the promise; all must now state the fact:

- `docs/src/reference/cli-reference.md:153` — the `--json` row.
- `docs/src/guides/validate-config-in-ci.md:36`.
- `docs/src/guides/migrating-to-6-0.md:30` — **most important.** This is
  the page a 5.19 user upgrading will actually read. It must say
  `--json` is gone, what to use, and what the error looks like.

Keep `--json` *documented as removed* rather than deleting every trace —
a reader hitting the error needs to find it explained.

## 6. Not in scope

- The version bump, CHANGELOG, and README dev-line notice — release
  mechanics, and **not the dev team's** (RFC 066 § 2: no commit that
  changes a version number).
- Any other flag or command.
- `--format json`'s own behaviour.

## 7. Acceptance

- [ ] `validate --json` → **exit 2**, message names `--format json`,
      **stdout empty**
- [ ] Under `--format json`, the envelope carries `error.kind: "usage"`
- [ ] `validate --format json` unchanged — full RFC 053 envelope
- [ ] `validate --format text` and bare `validate` unchanged
- [ ] `--json --format json` no longer needs its own conflict message
      (or, if kept, says something true)
- [ ] No string anywhere still says *"will be removed in 6.0.0"*
- [ ] `validate --help` no longer lists `--json`
- [ ] The three doc pages state the removal, with the error shown on the
      migration page
- [ ] `cargo test --workspace`, `fmt`, `clippy -D warnings`,
      `mdbook build docs`
- [ ] CI green on all 9 jobs before merge

## 8. Report back

`.git-exclude/review-request/6-0-0-remove-deprecated-json/`, including
the **exact stderr text and exit code** for `validate --json`, captured
from a real run — that message is the whole migration path for a
machine consumer, so it is the deliverable, not an implementation
detail.
