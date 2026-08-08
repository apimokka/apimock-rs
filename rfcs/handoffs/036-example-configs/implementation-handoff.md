# Implementation Handoff — RFC 036, Example configurations

**Governing RFC.** [RFC 036](../../done/036-example-configs.md)
**Milestone.** M2 (Documentation and examples) → v5.16.0
**Status.** Inherited from RFC 036 (Proposed)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

**Independent of RFC 034.** Examples live under
`crates/apimock/examples/`, not `docs/`, so this does not wait on the
information-architecture decision. Start whenever.

---

## 1. Purpose

Replace three placeholder config files with a graded, runnable,
automatically-verified set of examples organised by the task a user is
trying to accomplish.

## 2. Background — why this is not cosmetic

The current examples are 76 lines across three files, mostly commented
out. `apimock-rule-set.toml` has two rules responding `"hej ab"` and
`"hejhej cd"`. `apimock-middleware.rhai` is **entirely commented out** —
there is no working middleware example anywhere in the repository.

These are not private sample files:

- **`apimock --init` scaffolds from them**, so every new user's first
  config is a copy.
- **Every release archive ships them** — `release-executable.yaml` copies
  all three into all five platform archives.

So `"hej ab"` is the product's onboarding surface, and it went out again
in v5.15.0.

A 2026-08-03 survey found roughly 5% feature coverage: nothing
demonstrates the four non-default strategies, rule `priority`/`weight`,
44 of the 49 operator variants, status-code responses, CSV responses,
`prefix`, `guard`, file-tree filtering, TLS, the trace channel, or either
CLI subcommand.

## 3. Change scope

- `crates/apimock/examples/config/default/` — the `--init` scaffold
- New task-named example sets under `crates/apimock/examples/`
- Whatever verification mechanism you choose (§ 6.3)
- `.github/workflows/release-executable.yaml` **only if** the default
  set's filenames change (§ 6.2)

## 4. Explicit non-change scope

Do **not**:

- Touch `crates/apimock/examples/config/tests/` — those are test
  fixtures, not user examples.
- Touch `crates/apimock/examples/bench_load.rs` — a benchmark harness.
- Change any crate source. If an example cannot be written because a
  feature is missing or awkward, **that is a finding to report, not a
  licence to change the product.**
- Add a `--init --template <name>` flag. It is an attractive idea and
  RFC 036 names it explicitly as out of scope — it is a **CLI surface
  change**. Raise a design request if you think it is warranted.
- Write documentation prose in `docs/` — that is RFCs 034/035/037/038.
  Examples may carry explanatory comments.

## 5. Applicable requirements

RFC 036 in full, particularly its § Content rules.

## 6. Required implementation

### 6.1 The example set

Organised by **user task**, not by config-file section. Each set is
self-contained in one directory, runnable without editing paths, and
carries a `README.md` stating what it demonstrates, the command to run
it, and a `curl` invocation with its expected response.

RFC 036 lists candidate tasks — serving a REST resource from JSON files,
matching on headers and body, returning errors and status codes, varying
a response for one path, simulating a slow backend, scripting with
middleware, validating config in CI. **That list is candidates, not a
specification.** Propose the actual set against RFC 036's goals and say
why in the review request.

Non-negotiable content rules:

1. **Runnable.** If it cannot be demonstrated with a `curl` and a stated
   expected response, it does not belong.
2. **No commented-out feature demonstrations.** If a feature is worth
   showing, show it working. Commented-out config is how the JSONPath
   fixture bug survived three releases.
3. **Realistic shapes.** Resource paths, JSON bodies, and header names
   that resemble a real API. No `"hej ab"`.
4. **Body paths use the dotted mini-syntax**, never `$.`-prefixed
   pseudo-JSONPath, and say so where a reader might assume otherwise.
5. **At least one working middleware example.** Current count: zero.

### 6.2 `--init` and the release archives

Both consume `config/default/`. If its filenames or shape change, update
`release-executable.yaml`'s copy steps in the same change.

`--init` output must stay **minimal** — a starting point a user keeps,
not a tour of every feature. Changing what it writes is user-visible.

### 6.3 Verification — the part that matters most

Examples that are not executed will rot. This RFC requires a mechanism
that runs each set and asserts its documented responses.

Integration test, standalone script, or CI job — your call, but **state
which and why**. "We will keep them updated by hand" is not acceptable;
that is the current, failed arrangement.

If the mechanism binds ports, be aware CI runners are shared — prefer
ephemeral ports over fixed ones.

## 7. Required tests

1. Every example set starts the server successfully.
2. Every documented `curl` → expected-response pair is asserted
   automatically.
3. `apimock validate` passes on every example config.
4. `--init` scaffolding still produces a working config, verified by
   running it.
5. The existing suite still passes — **371 tests**, unchanged.

## 8. Acceptance criteria

1. Every example runnable; zero commented-out feature demonstrations.
2. Each set has a README with purpose, run command, and verified
   request/response pairs.
3. At least one **working** middleware example exists.
4. Examples are automatically verified; a broken example fails a check.
5. `--init` output remains minimal and is verified to run.
6. `release-executable.yaml` still copies a valid default set.
7. No crate source changed; no product behaviour changed.
8. 371 tests still pass.

## 9. Prohibited shortcuts

- Commenting out a feature demonstration that does not work rather than
  reporting why.
- Changing crate source to make an example work (§ 4).
- Manual-verification-only (§ 6.3).
- Expanding `--init` into a feature tour.

## 10. Escalation triggers

- **A feature cannot be exemplified** because it is missing, broken, or
  awkward to configure. Report it — that is a genuine product finding and
  precisely the kind of thing writing examples surfaces.
- **An example reveals a defect.** Do not fix crate source; raise it.
- You conclude `--init --template` is needed (§ 4).

## 11. Known risks

| Risk | Mitigation |
|---|---|
| More examples is more to maintain | Automatic verification — unverified examples are the liability, not examples |
| Verification costs CI time; example servers bind ports | Real cost, accepted; prefer ephemeral ports |
| Changing `--init` output affects existing users | Constrained to minimal, and verified to run |

## 12. Required evidence

- The verification mechanism running green, with its output.
- The same mechanism **failing** when an example is deliberately broken —
  a check observed only passing has not been tested. This milestone has
  made that point four times.
- `apimock validate` output for every example config.
- `--init` run end to end on a clean directory.
- `cargo test --workspace` at 371.

## 13. Required review-request format

Package at `.git-exclude/review-request/036-example-configs/` with an
entry-point file a reviewer can open cold. Per § 9.2 of the workflow
document. **Hand back one path — the entry-point file itself.**

Reviewer's focus: the verification mechanism, and whether any example is
shown working that has not actually been run.
