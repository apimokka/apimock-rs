# Implementation Handoff — RFC 054, the v5 deprecation release

**Governing RFC.** [RFC 054](../../done/054-deprecation-release.md)
**Contract.** [RFC 053](../../accepted/053-v6-cli-contract.md) — the
envelope you are implementing for the first time
**Milestone.** **Closes v5.** Ships as **5.19.0**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. Read this first — you are not working on `main`

**Branch from the `5.18.0` tag, not from `main`:**

```sh
git checkout -b release/5.19 5.18.0
```

`main` carries breaking work — RFC 040's `TraceConfig` fields, RFC 050's
additions to `ParsedRequest` and `RequestSummary` — that must **not** be
in this release. A deprecation release that itself breaks is useless.

So your baseline is **425 tests**, not `main`'s 437, and none of RFCs
040/050/051 exists in your tree. If you find yourself wanting a commit
from `main`, stop and escalate — that is the signal RFC 054's risk table
names.

## 2. What this release does

`validate` gains `--format text|json`, and `--json` keeps working with a
deprecation warning.

| Flag | Emits | Warning |
|---|---|---|
| `--json` | **today's bare array, byte-identical** | yes, once, stderr |
| `--format json` | **RFC 053's envelope** | no |
| `--format text` | today's default output | no |
| neither | today's default output | no |

The warning text from RFC 054:

```
apimock validate: --json is deprecated and will be removed in 6.0.0.
  Use --format json, which emits the new response envelope.
```

**On stderr. Exit code unchanged. Printed once**, not per diagnostic.

## 3. Why the second row is the point

A deprecation warning that only says *"this will change"* leaves the work
until after the break. Shipping **both shapes in one binary** lets a
consumer switch flags, adapt their parser, and verify it against a real
binary **before** 6.0.0 removes the old path.

That is the difference between a deprecation release and an
announcement, and it is the reason this release exists rather than a
line in release notes.

## 4. The envelope — build it to be reused

RFC 053 § Layer 2:

```json
{ "schema": 1, "apimock": "5.19.0", "result": { … } }
{ "schema": 1, "apimock": "5.19.0", "error": { "kind": "…", "message": "…" } }
```

- Object, **never** a bare array. The diagnostics collection goes
  *inside* `result`.
- Exactly one of `result` / `error`.
- `apimock` is the running binary's version — so `5.19.0` here, not
  `6.0.0`.

**Implement it as a small reusable helper, not inline in `validate`.**
`get` and `set` will emit the same envelope, and RFC 053 exists
specifically so they do not each invent one. A private module in
`crates/apimock/src/` is fine; what matters is that the next command
does not copy it.

**You are its first consumer as well as its first producer.** If
something about the shape is awkward to produce or to parse, say so —
this release exists partly to find that out on `validate` rather than on
`get`.

## 5. Scope boundaries

- **In:** `crates/apimock/src/cmd/validate.rs`, argument constants, the
  envelope helper, documentation, the migration guide (§ 7).
- **Out:** `match-test` — deliberately untouched. 6.0.0 will *add*
  `--format json` to it rather than changing its text output, so nothing
  breaks and nothing needs warning here.
- **Out:** `get`, `set`, any other envelope consumer, and anything from
  RFCs 040/050/051.
- **Out:** changing `validate`'s diagnostics, severities or exit codes.
  The *shape* around them changes; the content does not.

## 6. Evidence required

- **`--json` emits a byte-identical array to 5.18.0's.** Capture stdout
  and stderr **separately** and show it — the entire promise is that a
  parser reading stdout is unaffected.
- The warning appears on **stderr**, exactly **once**, for a config
  producing several diagnostics.
- **Exit codes unchanged** for every existing `validate` invocation:
  clean, with errors, with `--strict`, with `--quiet`.
- `--format json` emits a valid envelope — object, `schema`, `apimock`,
  exactly one of `result`/`error` — asserted on the parsed JSON, not a
  string match.
- `--format text` matches today's default output.
- `--json --format json` together is a **usage error, exit 2** — not a
  silent precedence rule.
- Full suite green; report the count against the **425** baseline.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 7. The migration guide ships here

Per RFC 054 Unresolved 3, and because someone meeting the CLI warning
wants the whole picture at once. It covers what **cannot** be warned
about, because there is no mechanism for it (RFC 048 § 7.3):

- `TraceConfig`, `RequestSummary`, `ParsedRequest`, `LogConfig` and
  `VerboseConfig` become `#[non_exhaustive]` in 6.0.0 (RFC 052) —
  struct literals and exhaustive destructuring stop compiling.
- `TraceConfig`, `ParsedRequest` and `RequestSummary` gain fields
  (RFCs 040, 050).
- The error enums may be reshaped (RFC 041, deferred to 6.0.0).
- `validate --json` is removed; `--format json` replaces it.

Write it as a document a person migrating can act on, not a changelog.
Where you are unsure what a break means for a caller, say so rather than
guessing — an honest gap is more useful than a confident wrong
instruction.

## 8. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package. Specifically escalate if you
need anything from `main`, or if RFC 053's envelope proves awkward in
practice — both are findings worth having before `get` is built.
