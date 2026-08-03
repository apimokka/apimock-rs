# RFC 036 — Example configurations for new users

**Status.** Proposed
**Tracks.** M2 (Documentation and examples). The shipped examples are
placeholders, not examples. They are also the *first* thing a new user
meets, because `apimock --init` scaffolds from them and the release
archives ship them alongside the binary.
**Touches.** `crates/apimock/examples/config/default/`, a new example
set under `crates/apimock/examples/`, and the `--init` scaffolding path
that copies them. Independent of RFC 034 — examples do not live in
`docs/`.

## Summary

Replace the three placeholder config files with a graded, runnable set
of example configurations organised by the task a user is trying to
accomplish, each self-contained and each demonstrating a feature that
currently ships undocumented and unexemplified.

## Motivation

### What ships today

Three files, 76 lines total, of which the majority are commented out:

**`crates/apimock/examples/config/default/apimock.toml`** (18 lines) —
reasonable as a minimal root config.

**`crates/apimock/examples/config/default/apimock-rule-set.toml`**
(30 lines) — two rules whose responses are `"hej ab"` and
`"hejhej cd"`, matching `/a/b` and `/c/d`. Everything else — `prefix`,
`default`, `guard`, headers, body conditions, file responses, delays —
is commented out.

**`crates/apimock/examples/config/default/apimock-middleware.rhai`**
(28 lines) — **entirely commented out.** There is no working middleware
example anywhere in the repository.

### Why this matters more than it looks

These are not merely sample files sitting in a directory. They are:

1. **What `apimock --init` writes.** A user's first config is a copy of
   these, so the placeholder quality is inherited by every new project.
2. **What ships in the release archives.** `release-executable.yaml`
   copies all three into every platform archive.

So `"hej ab"` is not a private placeholder — it is the product's
onboarding surface.

### What is wrong with them as examples

- **The responses are meaningless.** `"hej ab"` and `"hejhej cd"` teach a
  reader nothing about what a mock API response looks like. The target
  audience builds and consumes REST APIs; the examples should look like
  REST APIs.
- **No JSON response is demonstrated at all** — despite file-based JSON
  responses being the product's headline feature ("Drop JSON files into
  a folder and your API immediately exists").
- **Almost every feature is a comment.** A commented-out example cannot
  be run, cannot be verified, and cannot be tested, so it silently rots.
  The JSONPath-syntax incident recorded in `ROADMAP.md` § History is a
  worked example of exactly that failure mode.
- **The feature coverage is roughly 5%.** Nothing demonstrates: the four
  non-default strategies, rule `priority` or `weight`, 44 of the 49
  operator variants, status-code responses, CSV responses, `prefix`,
  `guard`, `[file_tree_view]` filtering, TLS, the trace channel, or
  either CLI subcommand.

## Goals

1. A new user can copy one example, run it, and see a realistic API.
2. Every example is **runnable and verified** — no commented-out
   feature demonstrations.
3. Examples are organised by user task, not by config-file section.
4. Coverage of the features a mock-server user actually reaches for.
5. `--init` scaffolding produces something a user would keep rather than
   immediately replace.

## Non-goals

- Documentation prose — RFCs 034/035 own `docs/`. Examples may carry
  explanatory comments; they are not a substitute for the guide.
- The test fixtures under `crates/apimock/examples/config/tests/`.
  Those serve the test suite and are out of scope.
- `crates/apimock/examples/bench_load.rs` — a benchmark harness, not a
  user-facing example.
- New product features. If an example cannot be written because a
  feature is missing or awkward, that is a finding to report, not a
  licence to change the product.

## Guide-level explanation

Examples become graded and task-named, so a user picks by intent:

```
crates/apimock/examples/
├── config/
│   ├── default/          ← what `--init` scaffolds; minimal but real
│   └── <task-named sets>  ← each self-contained and runnable
```

Each set carries a short `README.md` stating what it demonstrates, the
command to run it, and a `curl` invocation with its expected response —
so the example is verifiable by the reader, not just readable.

The exact set is for the implementer to propose against the goals above.
Candidate tasks, drawn from the coverage gap and the product's stated use
cases — **not a fixed list**:

| Task | Demonstrates |
|---|---|
| Serve a REST resource from JSON files | file-based routing, the headline feature |
| Match on headers and body | `when.request.headers` / `body.json`, dotted-path syntax |
| Return errors and status codes | `respond.status`, 4xx/5xx |
| Vary the response for one path | strategies, `priority` / `weight` |
| Simulate a slow or flaky backend | `delay_response_milliseconds` |
| Script a response with middleware | Rhai — currently zero working examples |
| Validate config in CI | `apimock validate`, `apimock match-test` |

## Reference-level explanation

### Content rules

1. **Runnable.** Every example runs as-is. Any example that cannot be
   demonstrated with a `curl` command and a stated expected response
   does not belong in the set.
2. **No commented-out features.** If a feature is worth showing, show it
   working. If it is not, leave it out. Commented-out config is how the
   JSONPath fixture bug survived three releases.
3. **Realistic shapes.** Resource paths, JSON bodies, and header names
   that resemble a real API. No `"hej ab"`.
4. **Self-contained.** One directory, runnable without editing paths.
5. **Body paths use the dotted mini-syntax**, never `$.`-prefixed
   pseudo-JSONPath, and say so where a reader might assume otherwise
   (ARCH-001).

### `--init` and release archives

`--init` and `release-executable.yaml` both consume
`config/default/`. If the default set changes shape or filename, both
paths must be updated in the same change. Changing what `--init` writes
is user-visible: it must stay minimal and must not become a tour of every
feature.

### Verification

Examples that are not executed will rot. This RFC requires a mechanism
that runs each example set and asserts its documented responses. Whether
that is an integration test, a script, or a CI job is the implementer's
call — but "we will keep them updated by hand" is not an acceptable
answer, since that is the current, failed arrangement.

## Required tests

1. Every example set starts the server successfully.
2. Every documented `curl` → expected-response pair is asserted
   automatically.
3. `apimock validate` passes on every example config.
4. `--init` scaffolding still produces a working config, verified by
   running it.
5. The existing suite (371 tests) still passes.

## Acceptance criteria

1. Every example is runnable, with zero commented-out feature
   demonstrations.
2. Each set has a README with purpose, run command, and verified
   request/response pairs.
3. The set covers the tasks agreed during implementation, including at
   least one **working** middleware example — the current count is zero.
4. Examples are automatically verified; a broken example fails a check.
5. `--init` output remains minimal and is verified to run.
6. `release-executable.yaml` still copies a valid default set.
7. No product behaviour changed.

## Drawbacks

1. **More examples is more to maintain.** Answered by the automatic
   verification requirement — unverified examples are the liability, not
   examples as such.
2. **Automatic verification costs CI time**, and example servers bind
   ports, which can be flaky in CI. Real cost; the alternative is
   documentation that silently lies.
3. **Changing `--init` output affects existing users' expectations.**
   Constrained above: minimal, and verified.

## Rationale and alternatives

**Alternative A (chosen): a graded, task-organised, verified set.**

**Alternative B: improve the three existing files in place.** Cheaper,
but a single rule-set file cannot demonstrate strategies, middleware, and
error handling without becoming the wall-of-config the reference page
already is.

**Alternative C: move examples into the docs as code blocks.** Rejected —
a code block cannot be run or verified, and `--init` and the release
archives need real files on disk regardless.

**Alternative D: leave examples, fix docs only.** Rejected: `--init`
scaffolding and the release archives are a first-run experience that
documentation does not reach.

## Unresolved questions

1. **How are examples verified?** Integration test, standalone script,
   or CI job. Deferred to implementation, but the choice must be stated
   and must not be manual.
2. **Does `--init` gain a `--template <name>` flag** to scaffold from a
   chosen example set rather than only the default? Attractive, and a
   natural fit — but it is a **CLI surface change**, therefore out of
   this RFC's scope. If the implementer thinks it is warranted, raise it
   as a design request; do not add it.
3. **Do the release archives ship more than the default set?** Currently
   three files are copied. Shipping the full set makes the archive a
   better artifact but larger.
