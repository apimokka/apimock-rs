# Roadmap

This file is the project's planning baseline. It records the agreed
themes, milestones, release cycle, and RFC portfolio — the plan that
the RFCs under `rfcs/` execute against.

It also keeps a history of design questions that were identified during
development and intentionally postponed, so the original context isn't
lost between releases. That history lives at the bottom of this file.

**Baseline approved.** 2026-08-02, by the project owner.
**Baseline covers.** v5.15.0 → v5.17.0.
**Current version.** 5.16.0 — 39 RFCs implemented, 1 withdrawn.
**M1 complete** (2026-08-03): RFCs 030–033 shipped in v5.15.0.
**M2 complete** (2026-08-10): RFCs 034–038 shipped in v5.16.0, together
with cross-cutting RFC 044 — the first release cut by automation.

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
| **M3** | Deferred design | The items RFCs 023 / 024 and open question Q-001 explicitly postponed are resolved. | 5.17.0 |

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
- **Major version** is out of scope for this roadmap. Whether and when
  6.0.0 happens is the project owner's decision alone, and completing
  this roadmap does not trigger it.
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

### M3 — Deferred design → 5.17.0

| RFC | Title | Pri | Depends on | State |
|---|---|---|---|---|
| 039 | `cargo public-api` additive-only enforcement | P1 | 031 | Planned |
| 040 | Trace channel: non-JSON body capture and redaction | P1 | — | Planned |
| 041 | Shrink large error variants (`result_large_err`) | P2 | — | Planned |
| 042 | `sync_from_disk` incremental reconciliation | P2 | — | Planned |
| 043 | Module split: `workspace/edit.rs`, `server/trace.rs` | P2 | — | Planned |
| 045 | [Configuration accepted but ignored](./rfcs/proposed/045-configuration-accepted-but-ignored.md) | P1 | — | Proposed |

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
    043                                     ▶ 5.17.0
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
| R-03 | RFC 039 breaks the GUI team's integration | Downstream breakage | Medium | GUI-team compatibility round-trip is a precondition for writing the RFC, not a follow-up | owner + architect |
| R-04 | Publishing npm at 5.15.0 leaves 5.10.1–5.14.0 permanently unpublished on that channel | User confusion about which versions exist | High (accepted) | Owner-accepted consequence of repairing rather than backfilling; noted in the 5.15.0 release notes. *Range corrected 2026-08-03 — last published npm version is 5.10.0, not 5.7.0* | owner |
| R-05 | CI tracks Rust `stable` while `Cargo.toml` pins MSRV 1.91.0 | Release build and local dev can diverge | Medium | RFC 031 adds an explicit MSRV job | architect |
| R-06 | Scope creep from docs work — rewriting docs surfaces genuine feature gaps | M2 expands into feature work | Medium | Feature gaps discovered during M2 become new RFCs for a later milestone; they do not join M2 | architect |
| R-07 | No load/performance evidence backs the README's k6 claim | Cannot verify a public claim | Low | Out of scope for this roadmap; revisit only if a regression is suspected | unassigned |

---

## Open decisions

| ID | Decision required | Owner | Blocking |
|---|---|---|---|
| D-01 | Target calendar window for M1–M3, so milestones can be dated | project owner | Scheduling only; RFC work can start without it |
| D-02 | Whether RFC 039 proceeds, pending the GUI-team compatibility round-trip | project owner | M3 scope |

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

Mostly raised by RFC 036 (2026-08-04) while writing runnable examples;
the prerelease row came out of RFC 044's live test (2026-08-08).
Recorded here so they are not lost between milestones.

| Finding | Disposition |
|---|---|
| `respond.headers` dropped on `status` responses; `content-type` overwritten on `text` | **RFC 045** |
| `[default].delay_response_milliseconds` parses, validates, logs, does nothing | **RFC 045** |
| `apimock validate` passes on inert configuration | **RFC 045** goal 4 — the finding underneath both |
| Bare relative `--config apimock.toml` fails to resolve; `./apimock.toml` works | Defect-fix task, no RFC. Narrow, unambiguous |
| `Guard` is a zero-field struct with a `// todo:` comment, published in the rule-set schema | **Owner decision** — implement, remove, or document as reserved |
| Trace channel has no config or CLI surface | Not a defect. RFC 035 documents it as library-only |
| **No prerelease version is releasable.** `[workspace.dependencies]` pins the internal crates with caret requirements (`version = "5"`), and a caret requirement never matches a prerelease. Any RC/beta tag fails resolution — established empirically during RFC 044 with both `0.0.0-rfc044-test` and `5.16.1-rfc044-test` | Unassigned. Not a defect; a constraint. Changing the pins to something prerelease-inclusive is a prerequisite for ever cutting an RC |
| Pre-existing ~1-in-8 port race in `dynamic_port()` (`tests/util/test_setup.rs`) | Unassigned. A fix was attempted during RFC 036, regressed every IPv6 bound-address test, and was reverted in full — see that review |
