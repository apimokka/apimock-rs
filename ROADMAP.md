# Roadmap

This file is the project's planning baseline. It records the agreed
themes, milestones, release cycle, and RFC portfolio — the plan that
the RFCs under `rfcs/` execute against.

It also keeps a history of design questions that were identified during
development and intentionally postponed, so the original context isn't
lost between releases. That history lives at the bottom of this file.

**Baseline approved.** 2026-08-02, by the project owner.
**Baseline covers.** v5.15.0 → v5.17.0.
**Current version.** 5.14.0 — 29 RFCs implemented, 1 withdrawn.

---

## Planning context

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
| **M2** | Truthful docs | A user reading `docs/src/` finds every shipped feature, and finds nothing that contradicts the code. | 5.16.0 |
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

- **One minor release per milestone.** 5.15.0, 5.16.0, 5.17.0. This
  continues the batching pattern used across v5.8.0–v5.14.0 (2–3 RFCs
  per minor release).
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

**Objective revised 2026-08-02 by the project owner.** M2 was originally
scoped as a correctness catch-up — "make the docs true". The owner
directed a rethink and restructure — "make the docs *good*": a reader
should be able to understand the tool and **predict the effect of a
configuration change before making it**. The correctness catch-up is
absorbed into that rewrite rather than run as a separate pass, so the
documentation is written once, not twice.

| RFC | Title | Pri | Depends on | State |
|---|---|---|---|---|
| 034 | [Documentation information architecture](./rfcs/proposed/034-documentation-information-architecture.md) | P0 | — | Proposed |
| 035 | User guide and configuration reference rewrite | P0 | 034 | Planned |
| 036 | [Example configurations](./rfcs/proposed/036-example-configs.md) | P0 | — | Proposed |
| 037 | README rethink | P1 | 034 | Planned |
| 038 | Technical reference refresh and document integrity | P1 | 034 | Planned |

RFC 034 is deliberately design-first and produces no prose: it decides
the personas, the navigation, and what belongs where, because 035, 037,
and 038 all inherit those decisions. Writing them before 034 settles
would mean inventing their structure twice.

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
in `CHANGELOG.md` (lines 417 and 714) and the broken `docs/CONFIGURE.md`
link in `vision-and-goals.md`.

### M3 — Deferred design → 5.17.0

| RFC | Title | Pri | Depends on | State |
|---|---|---|---|---|
| 039 | `cargo public-api` additive-only enforcement | P1 | 031 | Planned |
| 040 | Trace channel: non-JSON body capture and redaction | P1 | — | Planned |
| 041 | Shrink large error variants (`result_large_err`) | P2 | — | Planned |
| 042 | `sync_from_disk` incremental reconciliation | P2 | — | Planned |
| 043 | Module split: `workspace/edit.rs`, `server/trace.rs` | P2 | — | Planned |

RFC 039 closes open question Q-001 by turning DEC-014's additive-only
promise into a build-time check. RFC 040 resolves RFC 023's Unresolved
§1 and §2 — the second is security-relevant. RFC 041 arose from RFC 030's
escalation 002: `RoutingError`, `ConfigError`, `WorkspaceError`, and
`ServerError` each carry a `toml::de::Error` of ≥136 bytes, so every
`Result` returning through them trips `clippy::result_large_err`; RFC 030
suppressed 16 instances with justified `#[allow]`s, and RFC 041 decides
whether to box the variants and, if so, removes all 16 in the same
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
| R-04 | Publishing npm at 5.15.0 leaves 5.8.0–5.14.0 permanently unpublished on that channel | User confusion about which versions exist | High (accepted) | Owner-accepted consequence of repairing rather than backfilling; note it in the 5.15.0 release notes | owner |
| R-05 | CI tracks Rust `stable` while `Cargo.toml` pins MSRV 1.91.0 | Release build and local dev can diverge | Medium | RFC 031 adds an explicit MSRV job | architect |
| R-06 | Scope creep from docs work — rewriting docs surfaces genuine feature gaps | M2 expands into feature work | Medium | Feature gaps discovered during M2 become new RFCs for a later milestone; they do not join M2 | architect |
| R-07 | No load/performance evidence backs the README's k6 claim | Cannot verify a public claim | Low | Out of scope for this roadmap; revisit only if a regression is suspected | unassigned |

---

## Open decisions

| ID | Decision required | Owner | Blocking |
|---|---|---|---|
| D-01 | Target calendar window for M1–M3, so milestones can be dated | project owner | Scheduling only; RFC work can start without it |
| D-02 | Whether RFC 039 proceeds, pending the GUI-team compatibility round-trip | project owner | M3 scope |
| D-04 | Whether RFC 033 keeps its dependency-licence check at all | project owner | RFC 033 scope only |

Decisions taken on 2026-08-02:

- Milestone order: M1 first, then M2, then M3.
- npm remains a supported distribution channel; RFC 032 repairs it.
- Clippy findings are fixed, not suppressed; then gated.
- One minor release per milestone.
- **D-03 — npm resumes at the next release.** 5.8.0–5.14.0 are not
  backfilled; the npm channel goes from 5.7.0 to 5.15.0. The version
  gap gets a line in the 5.15.0 release notes.
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
