# Implementation Handoff — two tests that prove less than they appear to

**Source.** ROADMAP findings recorded 2026-08-27 and 2026-08-28.
**Milestone.** 6.x. **Not blocking.**
**Baseline.** `main` @ `eb79803`.

**Self-contained.** Two items, one theme: **a test whose name promises
more than it checks is worse than a missing test**, because it stops
anyone looking again.

One of these has already cost us — the divergence it would have caught
shipped as far as review before anyone noticed.

---

## 1. Two `Respond` validators can diverge, and did

`apimock-routing`'s `Respond::validate` and `apimock-config`'s
`workspace/validate.rs::respond_node_validation` are **independent
implementations of the same rules.** The duplication is deliberate and
has a real reason, stated in that module's own header: a GUI needs
structured `(severity, message, target_id)` triples, not `log::error!`
plus a `bool`.

**Nothing keeps them in step, and RFC 065 proved it.** The config-side
copy did not learn about the new `json` body source, so **every rule
`apimock set --json` wrote would have made `apimock validate` report a
false error on every run.** Caught only because the dev team tested the
*success* path — a valid `respond.json` config that `validate` should
accept — rather than another failure case.

That module's header calls itself *"Single source of truth"*. That is
true of `validate()` and `snapshot()` agreeing with **each other**, and
says nothing about either agreeing with `Respond::validate`.

### What to build

**A test that runs both validators over one shared corpus of `Respond`
values and asserts they agree.** Agreement means: both accept, or both
reject. The exact message and severity need not match — only the
verdict.

The corpus must cover, at minimum:

- Each body source alone: `file_path`, `text`, `json`, `status`-only.
- **Every pairwise combination** — `json`+`text`, `json`+`file_path`,
  `file_path`+`text`, `file_path`+`status` — since mutual exclusivity
  is what diverged.
- Valid and malformed inline `json`.
- A referenced `.json` file, valid and malformed.
- The empty `Respond` (no field set).

**Where it lives is your call**, but it has to import both crates, so
`apimock-config`'s test tree is the natural home —
`apimock-config` already depends on `apimock-routing`.

**Do not consolidate the two validators.** The duplication is
justified; the defect is that nothing checks the justification still
holds. If you conclude consolidation is genuinely better, **report it,
do not do it** — that is a design change and it is the architect's.

### Acceptance

- [ ] The test exists and passes over the corpus above
- [ ] **Prove it catches divergence**: make one validator disagree with
      the other on one case — e.g. remove `json` from the config-side
      "at least one body source" check — and show the test fails,
      naming the disagreeing case. Revert
- [ ] Adding a **new** body source in future would fail this test until
      both validators know about it. Say how you satisfied yourself of
      that, since it is the property that matters and it cannot be
      demonstrated directly

## 2. Two `args.rs` tests pass with a fixture that cannot work

`bare_relative_config_resolves_the_same_as_dot_slash_prefixed` and
`config_equals_form_resolves_the_same_as_space_form` use a
`[listener]`-only config. **That config fails to load** — `missing
field "service"` — and the process exits 1.

They pass anyway, because they poll only for the `[config]` line, which
apimock prints **before** the load fails.

**They are not wrong.** Both assert path *resolution*, which is what
their names say and what they genuinely prove. The hazard is narrow and
specific: anyone strengthening either one to check the server actually
starts will hit a fixture that cannot start, for a reason invisible in
the test.

### What to build

Swap in a minimal genuinely-valid config — `[service]` with
`fallback_respond_dir = "."` — and `-p 0` for the port.

**Keep what each test asserts unchanged.** This is a fixture fix, not a
scope change: they should still prove path resolution and nothing more.
If a valid fixture makes it natural to assert the server started too,
that is a *separate* improvement — mention it, do not fold it in.

`serve_with_config_flag_behaves_like_bare_apimock_with_config_flag`
already uses the valid shape; match it.

### Acceptance

- [ ] Both tests use a config that actually loads
- [ ] Both still pass, and still assert the same thing
- [ ] **Confirm the new fixture genuinely loads** — the old one passed
      while failing, so "the test is green" is not evidence here.
      Show the config loading successfully outside the test
- [ ] No other test in `args.rs` uses the `[listener]`-only fixture; if
      any does, report it

## 3. Not in scope

- Consolidating the two validators (§ 1).
- Changing what any existing test asserts (§ 2).
- Any production code. Both items are tests and fixtures.

## 4. Report back

`.git-exclude/review-request/post-6-0-0-test-integrity/`, including the
§ 1 divergence demonstration and the § 2 evidence that the new fixture
loads.
