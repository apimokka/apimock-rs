# Roadmap

This file is the project's planning baseline. It records the agreed
themes, milestones, release cycle, and RFC portfolio — the plan that
the RFCs under `rfcs/` execute against.

It also keeps a history of design questions that were identified during
development and intentionally postponed, so the original context isn't
lost between releases. That history lives at the bottom of this file.

**Baseline approved.** 2026-08-02, by the project owner.
**Baseline covers.** v5.15.0 → the close of v5 *(extended 2026-08-17: v5 no longer ends at v5.18.0 — see § Closing v5, opening v6)*.
**Current version.** 5.19.0 — 47 RFCs implemented, 1 withdrawn.
**v5 is closed** (2026-08-17). `main` is the 6.0.0 line.
**M1 complete** (2026-08-03): RFCs 030–033 shipped in v5.15.0.
**M2 complete** (2026-08-10): RFCs 034–038 shipped in v5.16.0, together
with cross-cutting RFC 044 — the first release cut by automation.
**M3 partly shipped, then overtaken.** RFCs 046–047 shipped in v5.17.0,
045 and 049 in v5.18.0, 054 in v5.19.0. **RFCs 039, 041, 042 and 043 were
never written** and now target 6.0.0 — 041 because it is breaking, the
rest because v5 closed first. See § Closing v5, opening v6.

---

## Planning context

*Historical — this records the state that motivated the roadmap, as of
2026-08-02. Every condition listed below was resolved by M1 in v5.15.0
except the documentation one, which is M2's subject.*

Every RFC through 029 shipped, and `rfcs/proposed/` was emptied. The
v5.14.0 review that opened this planning round found the feature work in
good shape, and the surrounding process in poor shape:

- No CI gate runs `cargo fmt --check`, `cargo clippy`, or `cargo test`.
  The only Rust-side CI is a release-time `cargo build --locked`.
- Strict clippy fails and has been *growing* unobserved — 21 findings at
  the v5.14.0 handoff, 26 when re-measured on 2026-08-02.
- The npm publish path cannot work as written (see RFC 032).
- `docs/src/` documents roughly the v5.8.0 feature set and in places
  contradicts what ships.

So the first milestone buys back the ability to trust the pipeline; the
second makes the documentation true; the third clears the design work
that earlier RFCs explicitly deferred.

---

## Themes and milestones

| Milestone | Theme | Objective | Release |
|---|---|---|---|
| **M1** | Pipeline trust | Quality gates exist, run automatically, and pass. The release path — including npm — works end to end. | 5.15.0 |
| **M2** | Documentation and examples | A reader finds every shipped feature, finds nothing contradicting the code, and can predict a config change's effect before making it. | 5.16.0 — cut after all five RFCs land (see § M2) |
| **M3** | Deferred design, plus pipeline gaps found in v5.16.0 | The items RFCs 023 / 024 and open question Q-001 explicitly postponed are resolved. | **Partly shipped** — 046/047 in 5.17.0, 045 in 5.18.0. **039, 041, 042, 043 unwritten, carried to 6.0.0** |

**Order.** M1 → M2 → M3, sequential. M1 is first because the gates it
installs are what keep M2's and M3's work from decaying the way
v5.8.0–v5.14.0 did.

**Exit criteria per milestone.** Every RFC in the milestone is
Implemented and moved to `rfcs/done/`; all mandatory gates pass; the
release's documentation updates have landed; a release-readiness report
has been produced and the owner has approved the release.

**Dates.** Not yet set — pending a target window from the project owner.
The milestones are sequenced but unscheduled; this table gains dates
once that window is agreed.

---

## Release cycle

- **One minor release per milestone**, *where the milestone is
  release-shaped*. M1 → 5.15.0 followed this, continuing the batching
  pattern of v5.8.0–v5.14.0.

  **Amended 2026-08-04:** the rule does not fit a milestone whose work
  does not reach a release artifact. M2 is mostly documentation, and
  `docs.yaml` publishes the site on every push to `main` rather than on
  release — so those RFCs ship continuously and need no version. A
  release is cut for what changes the *artifact*, not for what completes
  a milestone. See § M2.
- **Patch releases** are reserved for defect fixes against a shipped
  minor. They are not used to land RFC work.
- **Release tags** are `X.Y.Z` with no `v` prefix, matching all existing
  tags.
- **Major version.** Whether and when 6.0.0 happens remains the project
  owner's decision alone, and completing this roadmap does not trigger
  it.

  **Amended 2026-08-17:** v6's *concept* is now accepted
  ([RFC 048](./rfcs/accepted/048-v6-cli-interface-concept.md)), and the
  transition is no longer out of scope — because the owner decided a
  deprecation window ships in a 5.x release first. That makes part of
  v6's design a prerequisite for finishing v5. See § Closing v5, opening
  v6. Timing of 6.0.0 itself is still unset.
- **Release gate.** From RFC 031 onward, no release candidate is
  prepared while any mandatory CI gate is failing.

---

## RFC portfolio

Priority: **P0** blocks the milestone · **P1** planned in the milestone ·
**P2** desirable, first to be cut if scope must shrink.

### M1 — Pipeline trust → 5.15.0

| RFC | Title | Pri | Depends on | State |
|---|---|---|---|---|
| 030 | [Warning-clean baseline](./rfcs/done/030-warning-clean-baseline.md) | P0 | — | Implemented (v5.15.0) |
| 031 | [CI quality gates](./rfcs/done/031-ci-quality-gates.md) | P0 | 030 | Implemented (v5.15.0) |
| 032 | [Release and packaging repair](./rfcs/done/032-release-and-packaging-repair.md) | P0 | — | Implemented (v5.15.0) |
| 033 | [Supply-chain gates](./rfcs/done/033-supply-chain-gates.md) | P1 | 031 | Implemented (v5.15.0) |

**Execution order.** 030 → 031 → 033, with 032 running in parallel from
the start — it shares no code with the others.

### M2 — Documentation and examples → 5.16.0

**Restructured 2026-08-04.** M2 is no longer a release-shaped milestone,
because most of it never reaches a release artifact. Two facts settled
this:

- `.github/workflows/docs.yaml` deploys the docs site on **every push to
  `main`**, not on release. Documentation is already decoupled from the
  release cycle.
- Only two of M2's five RFCs touch anything published in a release:
  RFC 036 (`examples/config/` is packaged into the crate) and RFC 037
  (`readme = "../../README.md"` — it is the crates.io landing page).

So M2 splits by artifact rather than by theme:

| | RFCs | Ships how |
|---|---|---|
| **Release-bearing** | 036, 037 | Define v5.16.0's content |
| **Continuously published** | 034, 035, 038 | Merge to `main` and go live; no version bump |

**Sequencing corrected 2026-08-04 — v5.16.0 waits for 035 and 038.**
The split above assumed the two halves were independent. They are not:
`README.md` is release-bearing and **frozen once published**, and it
links into the docs, which are continuous and moving. That coupling
means cutting v5.16.0 before the restructure would publish a
carefully-fact-checked README whose "learn more" link lands the reader on
a page still claiming `service.strategy` is *"the only value supported
today"* and documenting 5 operators against 49 in code.

RFC 037 removed an unverifiable claim on the principle that every claim
must be checkable. Pointing at a page that contradicts the product is the
same failure at one remove — so the release waits for the documentation
to be true.

Two consequences:

- **v5.16.0 is cut after RFCs 035 and 038 land**, not when 036 and 037
  do. Owner decision, 2026-08-04, taken on output quality over schedule.
- **RFC 037 is amended** to link the docs *root only*
  (`apimokka.github.io/apimock-rs/`), which never moves under any
  restructuring. Adopted regardless of ordering: a frozen artifact should
  not depend on a mutable URL structure. See RFC 037 § Amendment.

RFC 034 D6 (no redirects) therefore **stands, and stands more firmly** —
with the README no longer deep-linking, the inbound-link surface it was
weighing is smaller still.

**Consequence for the release cycle.** The roadmap's "one minor release
per milestone" rule still does not fit M2 mechanically — the docs RFCs
need no version bump and publish on merge. But per the sequencing
correction above, v5.16.0 is nonetheless **cut after all five land**,
because the README's frozen links make the halves interdependent in
practice even though they are independent in publishing mechanism.

**Consequence for RFC 034.** Its planned `SUMMARY.md` skeleton is
withdrawn: placeholder pages would go live immediately on merge. RFC 034
is now decisions-only, and 035/037/038 each land complete sections so
`main` is coherent at every commit. With the page map decided, those
three no longer gate each other and can run in parallel.

**Objective revised 2026-08-02 by the project owner.** M2 was originally
scoped as a correctness catch-up — "make the docs true". The owner
directed a rethink and restructure — "make the docs *good*": a reader
should be able to understand the tool and **predict the effect of a
configuration change before making it**. The correctness catch-up is
absorbed into that rewrite rather than run as a separate pass, so the
documentation is written once, not twice.

| RFC | Title | Pri | Depends on | State |
|---|---|---|---|---|
| 034 | [Documentation information architecture](./rfcs/done/034-documentation-information-architecture.md) | P0 | — | Implemented (v5.16.0) |
| 035 | [User guide and reference rewrite](./rfcs/done/035-user-guide-and-reference-rewrite.md) | P0 | 034 *(map decided)* | Implemented (v5.16.0) |
| 036 | [Example configurations](./rfcs/done/036-example-configs.md) | P0 | — | Implemented (v5.16.0) |
| 037 | [README rethink](./rfcs/done/037-readme-rethink.md) | P1 | 034 *(map decided)* | Implemented (v5.16.0) |
| 038 | [Technical reference and document integrity](./rfcs/done/038-technical-reference-and-document-integrity.md) | P1 | 034 *(map decided)* | Implemented (v5.16.0) |

RFC 034 was deliberately design-first and produced no prose. **Its
decisions landed 2026-08-04** — personas, a Diátaxis-based section model,
a placement rule, a page map with dispositions for all 38 current pages,
and a home for the predictability requirement. 035, 037, and 038 inherit
those decisions and, with the map settled, **run in parallel**.

RFC 035 absorbs the original catch-up scope — the operator tables (5
documented against 11 `RuleOp` / 13 `HeaderOperator` / 25 `BodyOperator`
in code), `service.strategy` still described as "the only strategy
supported today" when five ship plus the RFC 025 per-rule-set override,
and the wholly undocumented `apimock validate`, `apimock match-test`,
trace channel, TLS hot-reload, `[file_tree_view]` filtering, and rule
`priority` / `weight`.

RFC 036 is independent of the IA decision — examples live under
`crates/apimock/examples/`, not `docs/` — so it can run in parallel from
the start.

RFC 038 rewrites `docs/src/technical-reference/architecture.md`, which
still describes the pre-5.0.0 single-crate layout, and
`docs/src/technical-reference/workspace.md`, which calls `apimock` the
workspace-root crate (it moved to `crates/apimock/` in 5.1.1). It also
carries the document-integrity items: the duplicate `## [5.4.0]` entries
in `CHANGELOG.md` (lines 417 and 714), the broken `docs/CONFIGURE.md`
link in `vision-and-goals.md`, and the broken
`./getting-started/rule-based-routing.md` link in `user-guide/faq.md`
(the files are `-1.md` and `-2.md`).

**Scope added 2026-08-03 — the contributor path.** A survey found
`docs/` carries **no** local development procedure, no build-from-source
instructions, no test-running guide, and no `cargo install` path, despite
a crates.io badge in the README. The only contributor-facing technical
content anywhere is the six gate commands RFCs 031/033 added to
`.github/CONTRIBUTING.md`, unlinked from `docs/`. RFC 034 decides
*where* this lives (see its § 1); **RFC 038 owns writing it.** Without
this assignment the persona the project's own guidelines name third
would remain unserved by M2 — a gap in the milestone as I originally
scoped it.

**Scope added 2026-08-03 — RFC 037 (README).** Beyond the rethink, four
specific defects: two stale "4.7.0" references (`README.md:97`, `:100`)
that are meaningless to a 5.15.0 reader; an unverified "validated with
k6 load testing" performance claim (`README.md:52`, carried as RISK-004
since the v5.14.0 handoff with no evidence ever produced) — either
reproduce it or soften it; the absent "Features / Design Notes" section
that the project's own README structure rule specifies as section 5; and
an Acknowledgements list omitting `rustls`, `tokio-rustls`, `csv`,
`regex`, `globset`, `ignore`, `uuid`, and `indexmap`.

### Cross-cutting — release pipeline

| RFC | Title | Pri | Depends on | State |
|---|---|---|---|---|
| 044 | [Release process: documentation and automation](./rfcs/done/044-release-process-documentation-and-automation.md) | P1 | ships with 5.15.0 observed | Implemented (v5.16.0) |

RFC 044 belongs to no milestone — it is release-pipeline work, drafted
2026-08-02 and deliberately held until v5.15.0 had been cut by hand so
the automation was designed against observed behaviour rather than
assumed behaviour. Implemented and reviewed 2026-08-08; **v5.16.0 is the
first release cut through it.**

### M3 — Deferred design → 5.17.0 (partial) and 5.18.0

| RFC | Title | Pri | Depends on | State |
|---|---|---|---|---|
| 039 | [An additive-only gate for the public API](./rfcs/accepted/039-public-api-additive-only-gate.md) | P1 | 031 | **Drafted 2026-08-19.** Designed now, **enabled after 6.0.0** — a gate over a moving surface trains people to ignore it |
| 040 | [Trace channel: header redaction](./rfcs/accepted/040-trace-capture-and-redaction.md) | P1 | — | **Accepted** 2026-08-17 — implemented on `main`, awaiting 6.0.0. **Goal 3 removed** — see 050 |
| 050 | [Should non-JSON bodies be captured at all?](./rfcs/accepted/050-non-json-body-capture-decision.md) | P2 | 040 | **Accepted** — decided 2026-08-17, answer (2), presence only. Implemented on `main`, awaiting 6.0.0 |
| 051 | [Redact credential headers in verbose logging](./rfcs/accepted/051-verbose-log-header-redaction.md) | **P1** | 040 | **Accepted** 2026-08-17 — security. Implemented on `main`, awaiting 6.0.0. Was attempted **without new public fields** so it could ship in a true minor; it did not make 5.19.0, so it now ships with the rest |
| 052 | [`#[non_exhaustive]` on public types](./rfcs/accepted/052-non-exhaustive-public-types.md) | P1 | — | **Accepted** — decision approved 2026-08-17. Implemented on `main`; breaking, ships at the v6 boundary |
| 041 | [Error type shape: boxing, `kind()`, `#[non_exhaustive]`](./rfcs/accepted/041-error-type-shape.md) | P2 | — | **Accepted and handed off 2026-08-20; amended the same day to cover the whole public surface.** Breaking, so 6.0.0. 15 lint suppressions across 8 files, caused by `toml::de::Error` (88 bytes) held by value in two parse variants; plus `#[non_exhaustive]` on ~43 re-exported types, which **closes R-09** |
| 042 | [~~`sync_from_disk` incremental reconciliation~~ → **external change detection**](./rfcs/accepted/042-external-change-detection.md) | P2 | — | **Drafted 2026-08-19** — now a *correctness* RFC, not a feature one: `sync_from_disk`'s public doc promises NodeId preservation its own implementation contradicts three lines later, and `reseed_after_edit` was never built. **G1 answered 2026-08-17.** The owner rejects both automatic behaviour change *and* continuous watching, so no filesystem watcher and no `notify` dependency. Detection is a boot-time file list plus an existence/mtime poll; the response is to **ask the user**, not to act. That removes RFC 042's premise — it existed to make reconciliation *incremental* because wholesale reload was assumed too costly to do often, and a reload gated on explicit confirmation does not happen often. RFC 024 already covers part of the remainder |
| 043 | [Module split: `workspace/edit.rs`](./rfcs/accepted/043-module-split-edit-rs.md) | P2 | — | **Drafted 2026-08-19.** `server/trace.rs` **dropped from scope** — 570 code lines, not an outlier; its raw count was 44% tests. Sequence after RFC 057 |
| 045 | [Configuration accepted but ignored](./rfcs/done/045-configuration-accepted-but-ignored.md) | P1 | — | Implemented (v5.18.0) |
| 046 | [Test harness: port race and readiness](./rfcs/done/046-test-harness-port-race-and-readiness.md) | **P0** | — | Implemented (v5.17.0) |
| 047 | [Verify what was actually published](./rfcs/done/047-post-publish-artifact-verification.md) | P1 | 044 | Implemented (v5.17.0) |

**046 and 047 are unfinished M1 work, added 2026-08-12 after v5.16.0.**
M1's theme was pipeline trust, and both gaps are in that pipeline rather
than in M3's deferred-design theme — they are placed here because this is
the next release, not because they belong to the theme.

RFC 046 was **P0 and the only P0 in M3**: the harness flake it fixed
could fail `quality-gate`, which runs on every tag push, so it could fail
a release. Three consecutive local runs on 2026-08-12 failed twice,
against the roughly 1-in-8 recorded below. **Closed in v5.17.0** — 20
consecutive full runs across implementation and review, zero failures.

**v5.17.0 was cut with only 046 and 047**, ahead of the rest of M3 and
deliberately small. Two pipeline paths had never executed successfully —
`crates-io-publish` and 047's own verification jobs — and a one-API-
addition release is a better first exercise for them than a milestone's
worth of change. The remaining M3 RFCs move to v5.18.0. This follows the
release-cycle rule as amended for M2: a release is cut for what changes
the artifact, not for what completes a milestone.

RFC 047 closes the class of defect that let npm ship 4.6.9 binaries under
5.9.0–5.10.0 version numbers, undetected across several releases with
every CI job green. v5.16.0 was confirmed correct only because it was
checked by hand.

**RFC 041 is deferred to 6.0.0. Established 2026-08-17, from source
rather than assumed.** The large payload is a *public named field* —
`source: toml::de::Error` on `ConfigError::ConfigParse`
(`crates/apimock-config/src/error.rs:43`) and on the equivalent in
`apimock-routing` (`error.rs:46`). Neither enum is `#[non_exhaustive]`,
so downstream code can construct those variants, and changing the field
to `Box<toml::de::Error>` stops that construction compiling. Pattern
matches would mostly survive via `Deref`, but construction and any
explicit type annotation would not.

No non-breaking formulation exists: adding `#[non_exhaustive]` first is
itself breaking, and the payload cannot be shrunk without changing the
field's type. So 041 is a breaking change, and belongs where breaking is
sanctioned. Deferring costs nothing operationally — RFC 030's fifteen
suppressions are **function-targeted, not blanket**, so a new function
tripping `result_large_err` still fails `-D warnings`.

RFC 039 closes open question Q-001 by turning DEC-014's additive-only
promise into a build-time check. RFC 040 resolves RFC 023's Unresolved
§1 and §2 — the second is security-relevant. RFC 041 arose from RFC 030's
escalation 002: `RoutingError`, `ConfigError`, `WorkspaceError`, and
`ServerError` each carry a `toml::de::Error` of ≥136 bytes, so every
`Result` returning through them trips `clippy::result_large_err`; RFC 030
suppressed 15 instances with justified `#[allow]`s, and RFC 041 decides
whether to box the variants and, if so, removes all 15 in the same
change. RFC 042 resolves the simplification recorded as ARCH-002 /
DEC-024 and is the only item in this roadmap that changes what the GUI
must do; it requires a compatibility round-trip with the GUI team
*before* the RFC is written. RFC 043 addresses the two files over the
500-ELOC split recommendation: `workspace/edit.rs` (822) and
`server/trace.rs` (514).

**Renumbering note.** M3's items moved from 037–041 to 039–043 on
2026-08-02 to make room for M2's revised scope. This is permitted: under
[RFC 000](./rfcs/done/000-rfc-lifecycle-policy.md) a number is assigned
when the *file* is created, and none of these existed as files. **RFC 041
deliberately kept its number** — it is already referenced externally by
`.git-exclude/reviewed/030-warning-clean-baseline/DECISIONS-001-002.md`.

---

## Closing v5, opening v6

**Added 2026-08-17.** v6's concept is accepted and recorded in
[RFC 048](./rfcs/accepted/048-v6-cli-interface-concept.md): the CLI
becomes an interface in its own right, with `get` and `set` families
aimed principally at AI CLI agents and CI.

### Why this changes v5's end, rather than following it

The owner decided that v6 **may break 5.x invocations**, and that a
**deprecation window ships in a 5.x release first** (RFC 048 § 7, § 7.1).

A deprecation warning has to name its replacement. So the *enumeration*
of v6's breaking changes — which invocations change, and to what, not
how each is designed — **must be settled before the final 5.x ships**.
The last v5 release is therefore a v6 deliverable, and v5 cannot simply
be finished and handed over.

### What now closes v5

| | Work | State |
|---|---|---|
| 1 | M3's remainder — RFCs 039–043 | Planned; 042 still blocked on the GUI round-trip (D-02) |
| 2 | CLI hygiene — **[RFC 049](./rfcs/done/049-cli-front-door.md)** | ✅ **Shipped v5.18.0.** Item 4 is unblocked — an unrecognised flag now fails loudly, so a deprecation warning can be delivered |
| 3 | Enumerate v6's breaking changes | **Blocks the final 5.x** |
| 4 | Deprecation warnings — stderr, exit code 0, naming the removal version | **Cut from `5.18.0` on a short-lived branch** (owner decision 2026-08-17) — `main` now carries breaking work and can no longer produce a non-breaking release. See RFC 048 § 7.2 |

**Amended again 2026-08-17 — item 4 is unblocked and drafted as
[RFC 054](./rfcs/done/054-deprecation-release.md).** Checking the
code showed the deprecation release does *not* wait on v6's CLI design.
A warning is only needed where an *existing* invocation changes;
subcommands are matched positionally at `argv[1]` by exact string, so
`get` and `set` are new tokens that cannot change what any current
invocation does, and bare `apimock` is kept as an alias. That leaves only
the output shapes RFC 053 § 7 already enumerates. **v5 can close sooner
than the amendment below assumed.**

**Amended 2026-08-17 — item 3 is narrower, and item 4's blocker moved.**
A deprecation window can only warn about **CLI invocations**. There is no
mechanism to warn that a struct is about to become `#[non_exhaustive]`
or gain a field — both change what downstream *may write*, and no lint
covers that. So RFC 052's change, RFC 041's error boxing and RFC 050's
field additions reach users through the **migration guide**, announced
rather than warned (RFC 048 § 7.3).

The consequence is that the deprecation release is gated on **v6's CLI
surface being designed**, not on the library enumeration — which is
already largely known. That design is RFC 048 § 11's items 3–5, and it
is now the work standing between here and the close of v5.

Item 2 was "small but user-facing" while v5 was the whole story. Under
RFC 048 it is load-bearing twice over: a CLI that silently ignores an
unknown flag cannot deliver a deprecation notice anyone acts on, and
silent-wrong-behaviour is disqualifying for the agent users v6 targets.

### Where v5's release numbering lands

**Not yet decided.** v5.18.0 carries M3's remainder as planned. Whether
the deprecation release is v5.19.0 or v5.18.0 grown depends on how much
of the breaking enumeration is ready when M3 lands, and is a release
decision to take then rather than now.

### Security

RFC 048 § 9 opens a threat model for v6 and the owner has asked for it to
be refined during development. Its sharpest item is **T2**: `set` built
on the existing `Workspace::save` path would inherit the ability to
attach Rhai middleware (`toml_writer.rs:190` already emits
`service.middlewares`), turning a configuration write into code
execution. That is a scope decision, and it is wanted **before** `set` is
designed rather than after.

---

## v6 API principle — stated by the owner 2026-08-17

> "I would not like to pay such backward compatibility cost. … I would
> like to make APIs of v6 simple, clean, robust and stable, functional,
> and sophisticated as possible."

Given in answer to G4 (should `validate --json` be removed), but stated
generally, so it governs decisions not yet taken: **prefer a clean v6 API
over preserving a compatibility affordance.**

**It puts one already-taken decision back in play.** RFC 053 Layer 1 kept
bare `apimock` as an alias for `apimock serve`, explicitly on
compatibility grounds — *"breaking it buys tidiness alone"*. That
reasoning predates this principle and now sits against it.

My reading is that the alias survives, because bare `apimock` is not a
compatibility affordance but the zero-config entry point — the README's
opening promise, and the shape every example uses. Keeping it is a UX
decision, not a debt. **But that is a reading, and the owner's principle
is the owner's; it is theirs to overturn.**

---

## Dependency map

```
M1  030 ──▶ 031 ──▶ 033                     ▶ 5.15.0
    032 ─────────────────┘  (parallel)

M2  034 ──┬─▶ 035
          ├─▶ 037                           ▶ 5.16.0
          └─▶ 038
    036 ─────────────────┘  (parallel)

M3  031 ──▶ 039
    040
    041
    042  (needs GUI-team round-trip first)
    043                                     ▶ 5.18.0
    046  (shipped)                          ▶ 5.17.0
    047  (shipped, depends on 044)          ▶ 5.17.0
```

Cross-milestone: 039 depends on the CI infrastructure landed by 031.
Nothing else in M2 or M3 depends on M1, so M2 could be resequenced ahead
of M1 if priorities change — at the cost of leaving the npm path broken
longer.

Within M2, RFC 034 gates three of the four remaining RFCs because it
decides the structure they write into. RFC 036 is the exception and
starts immediately.

---

## Risk register

| ID | Risk | Impact | Likelihood | Mitigation | Owner |
|---|---|---|---|---|---|
| R-01 | RFC 031 turns on `-D warnings` while findings remain, blocking every subsequent PR | Development halts until fixed | Low | 031 depends on 030; 031's acceptance criteria require a green clippy run on the merge commit | architect |
| R-02 | The 26 clippy fixes in RFC 030 change behaviour | Silent regression | Low | Full test suite must pass unchanged; no test may be modified within RFC 030's scope | architect |
| R-03 | ~~RFC **042** breaks the GUI team's integration~~ **Closed 2026-08-17** — G1 answered and rescoped it; there is no incremental-reconciliation design left to break against. Original text: RFC **042** breaks the GUI team's integration *(corrected 2026-08-12 — both this row and D-02 read "039", a leftover of the 037–041 → 039–043 renumbering. 039 is the additive-only API gate, which cannot break a consumer; 042 is `sync_from_disk` reconciliation, the only item that changes what the GUI must do)* | Downstream breakage | Medium | GUI-team compatibility round-trip is a precondition for writing the RFC, not a follow-up | owner + architect |
| R-04 | Publishing npm at 5.15.0 leaves 5.10.1–5.14.0 permanently unpublished on that channel | User confusion about which versions exist | High (accepted) | Owner-accepted consequence of repairing rather than backfilling; noted in the 5.15.0 release notes. *Range corrected 2026-08-03 — last published npm version is 5.10.0, not 5.7.0* | owner |
| R-05 | ~~CI tracks Rust `stable` while `Cargo.toml` pins MSRV 1.91.0~~ | — | — | **Closed 2026-08-12.** RFC 031's `msrv` job exists and reads the pin from `Cargo.toml` rather than hard-coding it (`.github/workflows/ci.yaml:106`) | architect |
| R-06 | Scope creep from docs work — rewriting docs surfaces genuine feature gaps | M2 expands into feature work | Medium | Feature gaps discovered during M2 become new RFCs for a later milestone; they do not join M2 | architect |
| R-08 | `crates-io-publish` has never executed successfully — v5.16.0's crates were published by hand | A first-run failure blocks a release mid-flight, after npm has already published | Medium | v5.17.0 is its first real run and is treated as unproven; crates.io's "require trusted publishing" toggle stays off until it goes green (`RELEASING.md`) | architect |
| R-09 | **Public structs in the trace/request path are not `#[non_exhaustive]`, so adding a field is a breaking change** — `TraceConfig`, `RequestSummary` (`apimock-server::trace`), `ParsedRequest` (`apimock-routing`, re-exported at `lib.rs:43`). RFC 040 already added three fields to `TraceConfig`; that break is unreleased on `main`. RFCs 050 and 051 would add more | A minor release ships a breaking API change, unnoticed | **High — already happened once** | **Decided 2026-08-17: mark them `#[non_exhaustive]` — one deliberate break, then immunity. See [RFC 052](./rfcs/accepted/052-non-exhaustive-public-types.md).** Spans RFCs 040/050/051, so decided once rather than per-RFC. Exactly the class **RFC 039**'s additive-only gate exists to catch, and 039 is not built. Note `apimock-config`'s `view.rs` types *are* `#[non_exhaustive]` with a comment explaining why — so this is drift in applying a known idiom, not unawareness of it. `LogConfig` / `VerboseConfig` are exposed the same way. **Reopened 2026-08-19 — R-09 is not closed.** Establishing RFC 058's Unresolved 1 showed `apimock-routing`'s own config types are public and bare: `Prefix` (`rule_set::prefix`), and `RuleSet` / `Rule` / `Respond` re-exported at the crate root (`lib.rs:44`) — **none `#[non_exhaustive]`**. RFC 052 covered the trace and request path, not the routing config path, and this table recorded it as closed. RFC 058 marks `Prefix` because it adds a field there. **Decided 2026-08-20 — the open question is closed:** the owner made `#[non_exhaustive]` the **default for the whole re-exported public API** from 6.0.0, not a per-type judgement. A sweep found 84 bare public types, ~43 of them re-exported and therefore reachable. Folded into [RFC 041](./rfcs/accepted/041-error-type-shape.md) rather than a new number, since 041 already applies exactly this treatment to the six error enums — splitting one rule across two RFCs would mean applying it twice. 6.0.0 is the last free window: after it, every type left bare makes adding a field to that type breaking | owner + architect |
| R-10 | **`respond_dir` grows by one `./` segment on every `Workspace::save()`, in released code.** `RuleSet::new` defaults `respond_dir_prefix` to `"."`, joins it with the config-relative dir, and writes the result back (`apimock-routing/src/rule_set.rs:106-127`); `toml_writer` then persists it whenever it is `Some` (`apimock-config/src/toml_writer.rs:84-85`). Each load+save cycle joins once more onto what the last one wrote. **Both halves are present at the 5.19.0 tag**, so this is shipped, not a `main`-only regression, and not caused by RFC 056. The sibling fields are spared: `url_path_prefix` only `.map()`s an existing value, and `compute_fallback_respond_dir` guards on the default (`config.rs:171`) | A runtime-resolved value is persisted as if a user wrote it, and degrades the file on every save. **The GUI hits this today** — it calls the same `save()`. Config still loads, so it corrupts quietly | **High — happens on every save** | Found 2026-08-19 while reviewing RFC 057, by the dev team after my own bisection got it wrong. Not RFC 057's defect; `set` is just the first caller that saves often enough to expose it. ****RESOLVED 2026-08-20** — [RFC 058](./rfcs/accepted/058-respond-dir-prefix-persistence.md) implemented and merged to `main`. One field held both the authored and the resolved value; they are now separate, as `Respond.status`/`status_code` already was | architect |
| R-07 | No load/performance evidence backs the README's k6 claim | Cannot verify a public claim | Low | Out of scope for this roadmap; revisit only if a regression is suspected | **CLOSED 2026-08-20 — premise is stale.** Re-checked while auditing 6.0.0 readiness: there is **no k6 claim** in `README.md` or anywhere under `docs/src/**/*.md`. It was removed during RFC 037's README rethink and the risk row outlived it. What remains ("Fast to boot, light on memory") is a qualitative claim, not a benchmark figure, and needs no load evidence to stand. Reopen only if a numeric performance claim is reintroduced |

---

## Open decisions

| ID | Decision required | Owner | Blocking |
|---|---|---|---|
| D-01 | Target calendar window for M1–M3, so milestones can be dated | project owner | Scheduling only; RFC work can start without it |
| D-02 | ~~Whether RFC **042** proceeds~~ ✅ **Resolved 2026-08-17.** G1 answered: no watcher, no automatic action, confirm with the user. RFC 042 is rescoped to a fraction of its original size — see the M3 table | project owner | M3 scope |

Decisions taken on 2026-08-02:

- Milestone order: M1 first, then M2, then M3.
- npm remains a supported distribution channel; RFC 032 repairs it.
- Clippy findings are fixed, not suppressed; then gated.
- One minor release per milestone.
- **D-03 — npm resumes at the next release.** Not backfilled; the
  version gap gets a line in the 5.15.0 release notes.

  *Corrected 2026-08-03 against the live registry:* the last version
  actually published to npm is **5.10.0** (2026-05-16), not 5.7.0, so
  the gap is **5.10.1–5.14.0**, not 5.8.0–5.14.0. The original figure
  was inferred from `npm/package.json`'s local content rather than from
  the registry. crates.io, by contrast, is current — all four crates
  published through 5.14.0.
- **Release-archive layout: no change.** The DEC-031 / RISK-003
  "discrepancy" inherited from the v5.14.0 handoff was a
  misapplication — the flat-extraction rule governs *project structure
  archives* delivered to the owner, not the binary release assets built
  by CI. Removed from RFC 032's scope; no rule amendment needed.

---

## History — resolved deferred items

Items below were postponed during earlier development and have since
been resolved. They are kept, not deleted, so the original rationale
stays discoverable.

### Hidden / VCS / build-artifact directory filtering in `FileTreeView`

**Identified during:** 5.3.0 design discussion.

**Status:** ✅ **Resolved in 5.8.0 (RFC 005) + 5.9.0 (RFC 012) + 5.11.0 (RFC 019).**

- 5.8.0 (RFC 005): `FileTreeFilter` introduced with dotfile hiding and `BUILTIN_EXCLUDES` list. Default filter applied on `FileTreeView` build.
- 5.9.0 (RFC 012): `[file_tree_view]` TOML section, `RootSettingKey` variants, config-driven filter.
- 5.11.0 (RFC 019): `extra_excludes` upgraded from exact-match to glob patterns (via `globset`). `respect_gitignore` opt-in via the `ignore` crate. `RootSettingKey::FileTreeRespectGitignore` added.

### Header / body.json round-trip through `toml_writer`

**Status:** ✅ **Resolved in 5.5.0.** The headers / body / condition_statement / body_kind modules in the routing crate were promoted to `pub mod`, exposing the existing public-field `Headers`, `Body`, `ConditionStatement`, and `BodyKind` types. `toml_writer::request_table` now round-trips these conditions, and `EditCommand::UpdateRule` preserves them when the GUI's `RulePayload` (which doesn't surface these fields) calls back into the apply layer.

### Routing crate test coverage for `Headers::is_match` and `Body::is_match`

**Status:** ✅ **Resolved in 5.6.0.** Added 36 dedicated tests in the routing crate covering every operator variant of `Headers::is_match`, the request-shape edge cases (key missing, UTF-8 decode failure), `Body::is_match` across jsonpath hits / misses / value-type coercion, multi-condition AND for both Headers and Body, `validate()` for both, and the TOML deserialise surface. Tests live in `headers/tests.rs` and `body/tests.rs` to follow the existing routing-crate convention.

### 5.5.0 round-trip test fixtures used non-existent JSONPath syntax

**Status:** ✅ **Resolved in 5.7.0.** The 5.5.0 round-trip tests in `apimock_config::toml_writer::tests` and `apimock_config::workspace::tests` used `body.json` keys like `"$.user.name"` and `"$.action"` — syntax that *looks* like canonical JSONPath but isn't supported by the routing crate's dotted-path mini-syntax (`apimock_routing::util::json::json_value_by_jsonpath`). The tests still passed because they only verified round-trip preservation, never calling `is_match`. 5.7.0 rewrote the fixtures to use the correct dotted form (`"user.name"`, `"action"`), strengthened the rustdoc on `apimock_routing::util::json`, `apimock_routing::rule_set::rule::when::request::body::Body`, and `apimock_config::toml_writer::request_table` with explicit "not canonical JSONPath / RFC 9535" warnings, expanded the `apimock` example TOML's `body.json` block with realistic dotted-path examples, and added a JSONPath-mismatch note to `docs/src/advanced-topics/rule-set-config-structure/rules/when.md`.

---

## Findings awaiting disposition

Things noticed while doing other work and deliberately not fixed there —
most from RFC 036 (2026-08-04) while writing runnable examples, the
prerelease row from RFC 044's live test (2026-08-08), the CLI-flag row
from verifying the published v5.16.0 npm binary (2026-08-12). Recorded
here so they are not lost between milestones.

| Finding | Disposition |
|---|---|
| **No CLI flag value can begin with `-`, and neither standard escape hatch exists.** Found 2026-08-27 while verifying RFC 064's § 5 (which correctly established only that there was no *regression*). Measured: `--text "- item"` → exit 2 *"unrecognized argument"*; `--text=-hello` → exit 2; `--text -- -hello` → exit 2 *"unknown option '--'"*. `reject_unknown_flags` (RFC 059) rejects any token starting with `-` before `flag_value` is reached, and there is no `--flag=value` form and no `--` end-of-flags separator. So `apimock set rule --text` cannot express a markdown bullet, a YAML `---`, or a diff hunk. Pre-existing across all of v5; **not** introduced by RFC 064 | **Owner decision, before the 6.0.0 cut.** A class below RFC 064's defects — it is *loud* (exit 2), not silent — but there is **no workaround at all**, and the message misnames the problem (*"unknown option '- item'; did you mean '-c'?"*). Weighs more in v6 than v5 because `set` is the headline feature and its intended author is a machine generating fixtures. **My recommendation: amend RFC 064** to add a `--flag=value` form — same files, same tests, same subject — rather than open a new RFC |
| **The CLI front door still mishandles two caller-supplied inputs.** Found in the pre-6.0.0 audit, 2026-08-27, and measured against a binary built from `9bdc769`. **(1)** A bare relative `-c apimock.toml` fails on every subcommand — exit 2, *"failed to resolve path"* — while `-c ./apimock.toml` works. `Path::new("apimock.toml").parent()` returns `Some("")`, not `None`, so `path_util.rs:19`'s guard never fires and `canonicalize("")` fails. RFC 049 already solved this for the root parser (`normalize_bare_relative_path`); the subcommands never picked it up, and RFC 057 worked around it a third time inside `set.rs`. **(2)** An *optional* flag given no value is silently defaulted — `flag_value` returns `Option`, so "present, no value" is indistinguishable from "absent". `set rule … -c` exits **0** and writes to a config the caller never named; a dangling `--format` returns human text, exit 0, to a caller that asked for JSON. Required flags are caught correctly and are not in scope | **Resolved — [RFC 064](./rfcs/accepted/064-cli-front-door-completion.md), merged to `main` 2026-08-27 (`9f9a193`), CI green on all 8 jobs.** Two of the three remaining RFC 048 § 6 prerequisites. Also corrects `docs/src/reference/cli-reference.md`, which documents (1) as **already fixed**, states the wrong exit code for (2), and argues in prose that (2) is unavoidable — a claim `reject_unknown_flags`'s own `no_value` parameter disproves. Worth recording separately: the conformance suite asserted, in two empty tests, that this scenario did not exist. RFC 059 asked for inapplicable scenarios to be stated rather than omitted, which was right; the claim written under it was never checked, by the dev team or by me |
| **RFC 045 Goal 4 — `validate` passing on inert configuration — is still assumed, not established.** The fourth open RFC 048 § 6 prerequisite. No RFC after 045 addresses it; RFC 045 itself said the structural option *"may be impractical; that is a legitimate outcome, but it should be established rather than assumed"* | **Owner decision, before the 6.0.0 cut.** Deliberately excluded from RFC 064 so that RFC stays one coherent piece of work. Accepting the limitation with a stated reason closes it legitimately; leaving it unexamined does not, since RFC 048 made it blocking |
| 🔴 **Remotely reachable path traversal in `dyn_route`.** `curl --path-as-is 'http://host/../outside.txt'` returns **HTTP 200 with the file's contents** — verified 2026-08-20 against a running server. `dyn_route` joins a request-derived path onto the respond dir (`dyn_route.rs:126`) and checks only `.exists()`; `normalize_url_path` does not strip `..`, and there is **no traversal guard anywhere in `apimock-server`**. Encoded `%2e%2e` does not work; only a raw `..`. **Present in 5.19.0 and, by the code, every v5 release.** Found while scoping RFC 063 for the config-side sibling below | **BLOCKING for 6.0.0, and a shipped-software issue.** Bounded by the `127.0.0.1` default bind and by needing a client that does not normalise `..` — but bounded is not fixed, and "don't expose it" is not a control. [RFC 063](./rfcs/accepted/063-serve-path-confinement.md) drafted. **Owner decisions needed:** backport to a 5.19.1? publish an advisory? does 6.0.0 wait? My recommendation: backport and publish |
| **`RUSTSEC-2026-0258` — `h2` 0.4.15, "unbounded empty DATA frames", published 2026-08-17.** The `audit` job has been failing on `main` since. Found by the dev team across six RFC 061 runs and correctly reported rather than folded into a CI change; verified independently 2026-08-20 with `cargo audit`. `cargo update -p h2 --dry-run` resolves **0.4.15 → 0.4.18** — one package, MSRV-compatible with the pinned 1.91.0, 50 other dependencies unchanged | **BLOCKING for 6.0.0. Architect's.** apimock is an HTTP server and `h2` is hyper's HTTP/2 layer — production, not dev-only. Shipping a mock server with a known DoS-class advisory when the fix is a one-package lockfile bump is not defensible. Also note `RUSTSEC-2026-0249` (`smartstring` unmaintained) is present as an allowed warning, not a vulnerability — no action, recorded so it is not rediscovered as new |
| **`respond.file_path` has no path-traversal protection at serve time.** A rule set with `respond_dir = "responses"` and `file_path = "../../outside.txt"` serves that file — reproduced 2026-08-20 against the built binary. **Not remotely reachable**: `file_path` values are static, operator-authored config, never built from request input. What it is: a config file can serve any file the process can read. Found by the dev team while writing RFC 062's threat-model page, reported rather than fixed as the handoff asked | **Superseded 2026-08-20 by a worse sibling — see the row above.** Decide before 6.0.0. Matters more in v6 than v5 because v6 is *designed for machine-generated configs* — "don't run untrusted configs" is weak guidance for a tool whose headline feature is an agent writing them, and `set --file` is how such a value gets in. RFC 062 also leaves an unprincipled asymmetry: the write path is now confined, the read path is not. **RFC 063 recommended** — confine `file_path` under `respond_dir` with an opt-out, symmetric with RFC 062, or accept and document. Architect to draft |
| ~~**A regression test I required cannot fail.**~~ **RETRACTED 2026-08-27 — the claim was false and was never tested.** I asserted that `save.rs::respond_dir_is_a_fixed_point_across_repeated_real_saves` could not observe RFC 058's bug, because it does one `Workspace::load` then three `apply`+`save` rounds on the same instance. Verified properly this time: RFC 058's fix was reverted faithfully — all three source files from `84803a6`, adapted only for RFC 041's error boxing so it compiles — and **the original test fails**, at round 0. It catches the regression exactly as intended | **No action; the test is fine as written.** The error was mine and is the same one I have repeatedly flagged in others: I reasoned about a test's behaviour from reading it instead of running it. The dev team's RFC 060 § 4 said *their first property* had that limitation and noted the hand-written test shared its shape; I generalised that to a defect claim about the existing test and recorded it here without checking. Retained struck through rather than deleted, since a retracted finding is itself worth seeing |
| **`apimock set --json` produces a `text/plain` response.** The value is stored as `respond.text` and served as `content-type: text/plain; charset=utf-8`. Not an implementation slip — `RespondPayload` exposes only `file_path`/`text`/`status`/`delay_milliseconds` (`view.rs:444`) and `Respond` has no JSON field either (`respond.rs:19`), so mapping `--json` to `text` is the only thing the `EditCommand` surface can express. Found 2026-08-19 reviewing RFC 057 | **Architect's, before 6.0.0.** A flag named `--json` that yields a non-JSON response is misleading in exactly the script meant to show an agent building a mock API. Either document it as inline body text and stop implying JSON in W7, or give `RespondPayload` a content-type mechanism in a follow-up RFC |
| **Three of the four CLI commands accept unknown flags silently — the front-door rule is enforced only on `set`.** Verified 2026-08-20 against the built binary: `get /a --bogus` exits **0**; `validate -c cfg --bogus` exits **0** and prints "Validation passed"; `match-test /a --bogus` exits **1** for an unrelated missing-argument reason, not for the flag. Only `set` exits `2` (fixed by RFC 057). **The sharpest case: `validate --strct` — a typo of `--strict` — exits 0**, so a CI job that asked for strict validation silently gets non-strict and passes. Violates RFC 049's front-door rule and RFC 053's `usage`/exit-`2` contract, on the interface 6.0.0 is *about*. **Corrects an earlier row of mine** that recorded only `get` and stated `validate` rejects correctly — that came from testing `validate --bogus` with no config present, where it exits 2 for a different reason entirely | **Blocking for 6.0.0** in my view: U2 is the user that fails silently, and this is silence on the command CI runs. **[RFC 059](./rfcs/accepted/059-cli-contract-conformance.md) **accepted 2026-08-20**** — fixes all three and replaces the per-command tests that missed it with one cross-command conformance table |
| `respond.headers` dropped on `status` responses; `content-type` overwritten on `text` | **RFC 045** |
| `[default].delay_response_milliseconds` parses, validates, logs, does nothing | **RFC 045** |
| `apimock validate` passes on inert configuration | **RFC 045** goal 4 — the finding underneath both |
| Bare relative `--config apimock.toml` fails to resolve; `./apimock.toml` works | **Resolved for the top-level flag — RFC 049.** Fixed at the CLI layer (input normalisation in `args.rs`), not in `apimock-config` where the underlying `Path::parent()`/`canonicalize("")` fault actually lives. **`apimock validate --config` still has it** — `validate.rs` parses its own `--config` separately and was out of RFC 049's scope, so a bare relative path there still fails; `apimock validate --config apimock.toml` needs the same `./` prefix it always has |
| `Guard` is a zero-field struct with a `// todo:` comment, published in the rule-set schema | **Owner decision** — implement, remove, or document as reserved |
| Trace channel has no config or CLI surface | Not a defect. RFC 035 documents it as library-only |
| **A non-JSON request body's raw content is unrecoverable by the time `ParsedRequest` exists.** `parsed_request_from` (`crates/apimock-server/src/parsed_request.rs:39`) collects the body into a local `Bytes`, but `ParsedRequest.body_json` (`apimock-routing`) is `Some` only for valid JSON — everything else, the bytes are dropped before `ParsedRequest` is built. Found 2026-08-17 implementing RFC 040 goal 3 (a truncated UTF-8 snippet for non-JSON bodies), which needs exactly that data | **Resolved differently — RFC 050.** The decision was presence-only, never content, so the raw bytes were never needed after all; `ParsedRequest.body_len` and `RequestSummary.body_len` report any body's length (JSON included, after a review fix — see that RFC's review history), nothing more |
| `capture_in_log`'s `[request.headers]` line (`crates/apimock-server/src/parsed_request.rs:134`, gated by `log.verbose.header`, default off) prints every request header verbatim to the console — the same `authorization` / `cookie` / `x-api-key` values RFC 040 now redacts in the trace channel, unaffected by that redaction because it's a wholly separate code path. Found 2026-08-17 while implementing RFC 040 | **Resolved — RFC 051.** `capture_in_log` now calls the same `TraceConfig::is_header_redacted` the trace channel uses — one policy, shared by reference. `log.verbose.body` remains unredacted; see the row below |
| `log.verbose.body` prints a JSON request body's fields verbatim to the console, unredacted — a logged `{"password": "hunter2"}` is as exposed as an unredacted header was. Established while implementing RFC 051, per its handoff § 4 | Reported, not fixed — RFC 051 §4/Unresolved 2. Name-based header redaction doesn't transfer to body content (no header names to match against); fixing this means the value-scanning problem both RFC 040 and RFC 050 declined to solve. Unassigned |
| **No prerelease version is releasable.** `[workspace.dependencies]` pins the internal crates with caret requirements (`version = "5"`), and a caret requirement never matches a prerelease. Any RC/beta tag fails resolution — established empirically during RFC 044 with both `0.0.0-rfc044-test` and `5.16.1-rfc044-test` | ✅ **Resolved 2026-08-17.** `version.sh` now writes the exact workspace version into the pins, so a caret requirement containing a prerelease matches. Verified by bumping to `6.0.0-alpha.1`, building `--locked`, and reverting |
| **`apimock --version` and `--help` are not supported, and unknown flags are silently ignored** — the binary starts a mock server instead. `args_option_value` (`crates/apimock/src/args.rs:223`) looks up known option names and ignores everything else, so a typo'd flag launches a server rather than erroring. Found 2026-08-12 while verifying the published v5.16.0 npm binary | **Resolved — RFC 049.** `--version`/`--help` short-circuit before config or a listener; an unrecognised flag is now exit 2 on stderr with a near-match suggestion, no server started |
| `TestSetup.current_dir_path` calls `env::set_current_dir`, which is process-global while tests run concurrently — the field's own doc comment says *"caution: affects globally"* | Unassigned. Recorded by **RFC 046**, deliberately out of its scope |
| **The trace channel's redaction policy has no configuration surface.** `header_denylist` / `header_allowlist` live on `TraceConfig` (`apimock-server`), which is configurable only at the Rust level — so a GUI or CLI user cannot opt a redacted header back in even deliberately. Surfaced 2026-08-17 when the GUI confirmed it displays trace headers | Unassigned. Giving `TraceConfig` a config-file surface is its own piece of work; RFC 040 deliberately did not add one |
| **`apimock validate` can never exit `1`, and `--strict` has nothing to act on.** `Workspace::load` checks — identically — every condition the diagnostics walker reports on (empty or conflicting `respond`, missing `respond.file_path`, missing `fallback_respond_dir`), so a config either loads with zero diagnostics (exit 0) or fails to load (exit 2); nothing anywhere constructs a warning-severity diagnostic. True since `validate` shipped in v5.13.0. Found 2026-08-17 while building RFC 054's test fixtures | Unassigned. Documented honestly in v5.19.0 rather than fixed — a real fix loosens a load gate shared with server startup, or defers rejection into `validate()`, both larger than that release |
| Pre-existing ~1-in-8 port race in `dynamic_port()` (`tests/util/test_setup.rs`) | Unassigned. A fix was attempted during RFC 036, regressed every IPv6 bound-address test, and was reverted in full — see that review |
