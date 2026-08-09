# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [5.16.0] - 2026-08-10

A documentation and examples release. **No behaviour change, no API
change** — the library, CLI, and config surface are identical to 5.15.0.
What changed is what you can find out about them.

### Added

- **Eight runnable example configurations (RFC 036).** Task-named and
  self-contained — serving JSON resources, matching on headers and body,
  status codes and errors, varying a response by strategy, simulating a
  slow backend, Rhai middleware, TLS, and validating in CI. Each has a
  README with the command to run it and a `curl` with its expected
  response, and each is verified by an integration test that runs on
  every `cargo test --workspace`.

  These replace three placeholder files that were mostly commented out
  and answered `"hej ab"`. They matter beyond the repository: `apimock
  --init` scaffolds from them and every release archive ships them, so
  they were the first thing a new user saw. There is now a working
  middleware example, where before there were none anywhere.

- **A contributor section in the documentation.** How to build and test
  locally, what the six CI gates catch, and how the RFC process works —
  none of which was documented anywhere.

### Changed

- **The documentation is restructured and, for the first time in a
  while, true (RFCs 034, 035, 038).** Five sections replacing the old
  four, each answering one question: Getting started, Guides, Reference,
  How it works, Contributing. The previous "Advanced Topics" section —
  whose charter had become "things that did not fit" — is gone, its
  contents redistributed.

  Roughly two years of shipped features were undocumented and are now
  covered: `apimock match-test`, the four non-default rule-evaluation
  strategies, rule `priority` and `weight`, `structural_contains`,
  `map_has_key`, the negated operators, and `[file_tree_view]`
  filtering.

  The operator reference now lists **all 49 operators** — 11 `url_path`,
  13 header, 25 body — generated from the source enums rather than
  transcribed. The previous reference documented five.

- **The README links the documentation root only.** It ships frozen
  inside the published crate as the crates.io landing page, so it no
  longer depends on a URL structure that can change.

### Fixed

- **The configuration reference stated that four shipped strategies did
  not exist.** It described `first_match` as *"the only value supported
  today"*. `uniform_random`, `weighted_random`, `priority`, and
  `round_robin` have shipped since 5.8.0–5.9.0, along with RFC 025's
  per-rule-set override.

- **The architecture page described software that has not existed since
  5.0.0** — `src/config.rs`, `src/server.rs`,
  `src/core/server/routing.rs`. A contributor following it looked for
  files that were not there.

- **A duplicate `## [5.4.0]` entry in this changelog.** Both were
  introduced in the same commit as a paste duplication; the one with
  accurate line counts and correct chronological position was kept.

- Two dead documentation links, and a flowchart showing `200 OK` for
  `OPTIONS` where the code returns `204 No Content`.

### Documented honestly rather than fixed

Writing runnable examples and verifying every documented claim against
code turned up features that do not work the way their names suggest.
None are fixed in this release — they are **documented as they actually
behave**, and tracked for a later one:

- **`[file_tree_view]` does not filter what the server serves.** It
  governs the config editor's browsable view only. A file matching an
  exclude pattern is still served on an exact URL request.
- **TLS hot-reload is not reachable from the CLI.** The mechanism exists
  and is tested, but nothing in the shipped binary exposes it. Restarting
  the process remains the only way to rotate a certificate.
- **The live match-trace channel has no configuration or CLI surface.**
- **`[default].delay_response_milliseconds` has no effect.** The
  per-rule `respond.delay_response_milliseconds` works correctly.
- **`respond.headers` is unevenly honoured** — dropped entirely on
  `status`-bearing responses and on plain-text file responses.
- **`[guard]` does nothing.** It is a placeholder with no fields.

### Internal

- **The release process is automated and written down (RFC 044).** This
  is the first release cut through it. A tag push now runs the version
  and quality gates, creates a **draft** GitHub Release with notes taken
  from this file, and attaches all five platform archives to it before
  anything is visible. Publishing that draft — the one human decision —
  is what triggers npm and crates.io publishing, in a separate workflow
  that a tag push cannot reach.

  Previously the Release was created by hand, assets trickled in over
  the following minutes while it was already public, and crates.io
  publishing had no automation at all — its script was broken by the
  5.1.1 workspace split and later deleted. The runbook lives in
  `RELEASING.md` at the repository root.

### Test count

| Crate / target | 5.15.0 | 5.16.0 | Delta |
|---|---|---|---|
| apimock (lib) | 22 | 22 | — |
| apimock integration (`tests/`) | 158 | 158 | — |
| apimock examples (`tests/examples.rs`) | — | **38** | **+38** |
| apimock-config | 60 | 60 | — |
| apimock-routing | 116 | 116 | — |
| apimock-server | 14 | 14 | — |
| apimock-config (doctest) | 1 | 1 | — |
| **Total** | **371** | **409** | **+38** |

## [5.15.0] - 2026-08-03

A quality and release-infrastructure release. **No user-facing feature
changes and no API changes** — the library, CLI, and config surface
behave identically to 5.14.0. What changed is everything around them:
the npm packaging that was shipping wrong versions, and the absence of
any automated quality gate.

### Fixed

- **npm packaging has been shipping the wrong binaries, and still is
  until this release publishes (RFC 032).** The currently-published
  `apimock-rs@5.10.0` on the npm registry pins its `optionalDependencies`
  platform packages at `4.6.9`, even though those platform packages have
  themselves already been published up to `5.10.0` — so installing
  `apimock-rs` from npm today resolves binaries several minor versions
  behind the package's own version. All npm package versions and
  platform pins are now correct in the tree and verified in CI before
  publication.

  **npm consumers:** the last version actually published to npm is
  `5.10.0` (2026-05-16); `5.10.1` through `5.14.0` were never published
  and are not being backfilled, so the npm version history jumps from
  `5.10.0` to `5.15.0`. (crates.io, unlike npm, is current: all four
  crates are published through `5.14.0`.)

- **`version.sh --update` was a silent no-op (RFC 032).** Two independent
  defects, both introduced by the 5.1.1 workspace split: it searched for
  npm manifests under each crate's directory, which stopped reaching
  root-level `npm/`; and its TOML rewriter could not match
  `version.workspace = true`. It reported success without changing
  anything. Rewritten around an explicit target list, now covering the
  workspace manifest, the internal crate pins' major component,
  `Cargo.lock`, and every npm manifest — and it verifies every target it
  claims to have updated, exiting non-zero if any did not land.

- **`cargo build --workspace --all-features` did not compile.** The
  `spawn`-gated `apimock::new` takes three arguments, but the binary's
  only call site passed one, so any `--all-features` build failed. The
  binary now calls `App::new` directly, matching what every other in-repo
  caller already did. Behaviour is unchanged.

- **A test could never pass on a host without globally-routable IPv6.**
  `ipv6_localhost_bound_nonlocalhost_request` selected any non-loopback
  IPv6 interface, including link-local (`fe80::/10`) addresses, which
  cannot be dialled without a zone ID — producing `EINVAL` rather than
  the expected `ConnectionRefused`. It now excludes link-local addresses
  and skips with a note when no suitable interface exists. Found by CI on
  its first run.

### Changed

- **The whole workspace is warning-clean (RFC 030).** 120 clippy findings
  across all four crates and three `rustc` build warnings resolved;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  now exits 0, and `cargo build --workspace --all-targets` emits nothing.
  No behaviour change: no public signature altered, no test assertion
  touched, and the suite count held constant throughout.

  17 `#[allow]` annotations were added, each with an inline justification.
  15 are `clippy::result_large_err` on the four public error types, all
  tracing to a `toml::de::Error` of ≥136 bytes carried by each; that is a
  tracked follow-up, not an oversight.

- **CI now gates every push and pull request (RFC 031).** A new `ci.yaml`
  runs format, lint, test, and MSRV checks in parallel, all blocking.
  Previously the only Rust-side CI was a release-time build, so every
  quality claim in this project's history rested on manual discipline.
  The release workflow additionally re-runs the gates before building any
  artifact.

  The MSRV job reads `rust-version` from `Cargo.toml` rather than
  hardcoding it, and this is the first release in which the pinned
  `1.91.0` has actually been verified rather than asserted.

- **The mandatory test command is now `cargo test --workspace`.** The
  previously recorded gate was `--workspace --lib`, which ran 212 tests
  and silently skipped 159 integration tests. Those 159 were passing all
  along — the coverage was real, the measurement was not.

- **Dependency hygiene is checked automatically (RFC 033).** `cargo audit`
  runs on push, pull request, and a weekly schedule — the schedule
  matters because advisories are published against code that has not
  changed. A lockfile-freshness check runs on push and pull request.

### Test count

The total did not grow; the measurement became honest.

| Crate / target | 5.14.0 (as reported) | 5.15.0 (actual) |
|---|---|---|
| apimock (lib) | 22 | 22 |
| apimock-config | 56→60 | 60 |
| apimock-routing | 111→116 | 116 |
| apimock-server | 14 | 14 |
| integration tests (`crates/apimock/tests/`) | *not counted* | 159 |
| **Total** | **212** | **371** |

## [5.14.0] - 2026-05-22

### Added

- **RFC 027 — Rule priority field in view, payload, and TOML writer.**
  `Rule.priority: Option<i32>` already existed in the routing crate
  and was used by the `Priority` strategy, but it was invisible to the
  GUI: not in `RuleView`, `RulePayload`, or `toml_writer`. This RFC
  surfaces it across the full stack — `RuleView.priority`, `RulePayload.priority`,
  `build_rule_from_payload` wiring, and `rule_table()` emitting
  `priority = <n>` when set. Save/load round-trip now preserves
  per-rule priorities. (`apimock-routing`, `apimock-config`)

- **RFC 028 — `StructuralContains` body operator.** Checks whether the
  value at the configured path is an array containing at least one
  element that is a structural *superset* of the configured JSON object
  (i.e. every key in the needle is present in the element with an equal
  value; the element may have additional keys). Recursive for nested
  objects. For non-object needles falls back to strict equality, making
  it a generalisation of `ArrayContains`. Added to `BodyOperator`,
  `BodyOp`, `body_op_name`, and payload converter. (`apimock-routing`,
  `apimock-config`)

- **RFC 029 — Per-condition diff granularity.** `DiffKind` gains four
  new variants: `HeaderConditionAdded`, `HeaderConditionRemoved`,
  `BodyConditionAdded`, `BodyConditionRemoved`. `compute_diff_summary`
  now calls `append_condition_diff` for each changed rule, emitting
  fine-grained items alongside the existing `RuleUpdated` item.
  Condition-level `DiffItem`s carry the condition's `NodeId` (from
  RFC 016) so the GUI can highlight exactly which condition was added
  or removed. (`apimock-config`)

### Fixed

- `body_op_name` in `view/build.rs` was missing `StructuralContains`
  (caught immediately by the exhaustive match added in this release).

### Test count

| Crate | v5.13.0 | v5.14.0 | Delta |
|---|---|---|---|
| apimock (façade) | 22 | 22 | — |
| apimock-config | 56 | 60 | +4 (RFC 027/029) |
| apimock-routing | 111 | 116 | +5 (RFC 028) |
| apimock-server | 14 | 14 | — |
| **Total** | **203** | **212** | **+9** |



## [5.13.0] - 2026-05-22

### Added

- **RFC 024 — Workspace external-change detection.** `Workspace` gains
  `has_external_changes() -> bool` (polls mtime + size of every tracked
  config file) and `sync_from_disk() -> Result<(), WorkspaceError>`
  (reloads the workspace in-place). `Workspace` now stores a
  `file_metas: HashMap<PathBuf, FileMeta>` field, populated at `load()`
  and refreshed after each `save()`, so `has_external_changes()` returns
  `false` immediately after a save. (`apimock-config`)

- **RFC 025 — Per-rule-set strategy override.** `RuleSet` gains an
  optional `strategy: Option<Strategy>` field (TOML-deserialisable).
  `find_matched` uses the per-rule-set strategy first, falling back to
  the service-level strategy. `RuleSetView` exposes the override as
  `strategy: Option<String>`. New `EditCommand::UpdateRuleSetStrategy {
  id, strategy }` sets or clears the override; unknown strategy names
  return `ApplyError::InvalidPayload`. `toml_writer` emits the field
  when set. (`apimock-routing`, `apimock-config`)

- **RFC 026 — `apimock validate` CLI subcommand.** `apimock validate
  --config <apimock.toml>` loads the workspace, runs validation, prints
  diagnostics, and exits 0 (pass), 1 (errors / `--strict` warnings), or
  2 (load failure). Flags: `--strict`, `--quiet`, `--json`. Dispatch
  wired in `args.rs` alongside `match-test`. (`apimock`)

### Test count

| Crate | v5.12.0 | v5.13.0 | Delta |
|---|---|---|---|
| apimock (façade) | 18 | 22 | +4 (validate CLI) |
| apimock-config | 49 | 56 | +7 (RFC 024/025) |
| apimock-routing | 111 | 111 | — |
| apimock-server | 14 | 14 | — |
| **Total** | **192** | **203** | **+11** |



## [5.12.0] - 2026-05-22

### Added

- **RFC 021 — Negated value operators.** Four negated string-style operators
  added to every matching surface (`url_path`, `header`, `body.json`):
  `not_contains`, `not_starts_with`, `not_ends_with`, `not_regex`.
  `not_regex` with an invalid pattern returns `false` (non-matching),
  consistent with `regex`. (`apimock-routing`, `apimock-config`)

- **RFC 022 — `map_has_key` / `map_does_not_have_key` body operators.**
  Two new `BodyOperator` variants that check for the presence of a
  named key within a JSON object at the resolved path. Both return
  `false` when the resolved value is not an object. (`apimock-routing`,
  `apimock-config`)

- **RFC 023 — Body capture in match-trace events.** `RequestSummary` gains
  `body_json: Option<serde_json::Value>` and `body_truncated: bool`.
  `TraceEmitter` gains `with_config(TraceConfig)` constructor and
  `enrich_with_body(&mut summary, body)` helper. Body capture is off by
  default; enabled via `TraceConfig { capture_body: true, .. }`.
  `AppState` now carries a `TraceEmitter`; the `service()` handler
  emits a trace event on each matched request. `RootSettingKey` gains
  `TraceCaptureBody` and `TraceMaxBodyBytes` variants (both `SoftReload`).
  (`apimock-server`, `apimock-config`)

### Test count

| Crate | v5.11.0 | v5.12.0 | Delta |
|---|---|---|---|
| apimock (façade) | 18 | 18 | — |
| apimock-config | 49 | 49 | — |
| apimock-routing | 91 | 117 | +26 (negated + MapHasKey) |
| apimock-server | 10 | 14 | +4 (body capture) |
| **Total** | **168** | **198** | **+30** |



## [5.11.0] - 2026-05-22

### Fixed

- **RFC 017 — Payload operator routing parity.** Five payload-layer operators
  silently mapped to the wrong routing operators since v5.8.0:
  - `UrlPathOp::EndsWith` → was `Contains`; now correctly `ends_with`.
  - `HeaderOp::EndsWith` → was `Contains`; now correctly `ends_with`.
  - `HeaderOp::Regex` → was `Equal`; regex is now applied.
  - `HeaderOp::Exists` → was `Equal ""`; now correctly checks key presence.
  - `HeaderOp::Absent` → was `Equal ""`; now correctly checks key absence.
  - `UrlPathOp::NotEqual` doc-comment corrected (previously said "Regular expression match").

  **Breaking:** rules that relied on the old (incorrect) behaviour will now
  behave as their operator names suggest. `ends_with` rules no longer
  over-match; `exists`/`absent` rules now work. (`apimock-routing`,
  `apimock-config`)

### Added

- **RFC 017 — `HeaderOperator` enum.** New flat 9-variant enum in
  `apimock-routing` (mirrors `BodyOperator` pattern). `Headers` now uses
  `HeaderConditionStatement { op: Option<HeaderOperator>, value: String }`
  in place of the shared `ConditionStatement`. `ConditionStatement` is now
  unused and may be removed in a future release.
  (`apimock-routing`)

- **RFC 017 — `UrlPathOp::Regex`.** Regex matching is now available for
  URL-path conditions through the GUI payload, closing the asymmetry with
  `HeaderOp::Regex`. Resolves RFC 001 Unresolved §1. (`apimock-config`)

- **RFC 017 — `RuleOp::EndsWith` and `RuleOp::Regex`.** Two new variants in
  the routing crate's core operator enum. (`apimock-routing`)

- **RFC 019 — `.gitignore` honouring in `FileTreeFilter`.** New
  `respect_gitignore: bool` field (default `false`). When enabled, the file
  tree builder parses `.gitignore` files in the workspace tree and its
  ancestors, hiding entries that Git would ignore. (`apimock-routing`,
  `apimock-config`)

- **RFC 019 — Glob-pattern `extra_excludes`.** `FileTreeFilter::extra_excludes`
  entries are now evaluated as `globset` glob patterns instead of exact
  name matches. Literal names continue to work. `include` patterns also
  upgraded to glob. New `RootSettingKey::FileTreeRespectGitignore` variant.
  (`apimock-routing`, `apimock-config`)

  **Breaking:** Entries in `extra_excludes` containing glob metacharacters
  (`*`, `?`, `[`, `]`) will now be interpreted as glob patterns. Bare names
  without metacharacters are unaffected.

- **RFC 020 — TLS certificate hot-reload (Outcome C).** New
  `ReloadableCertResolver` in `apimock-server::tls` uses
  `rustls::ResolvesServerCert` to swap the active certificate atomically
  without restarting the HTTPS listener. `ServerHandle` gains a
  `cert_reloader: Option<Arc<ReloadableCertResolver>>` field and a
  `reload_tls_certs(cert_path, key_path)` method.
  `ReloadHint::for_key` updated: `TlsCertFile` / `TlsKeyFile` now return
  `SoftReload` instead of `HardRestart`. `TlsEnabled` toggle still requires
  `HardRestart`. (`apimock-server`, `apimock-config`)

### Documentation / RFC lifecycle

- **RFC 018 — ConditionalFallback withdrawn.** Audit found the existing
  multi-rule-set fall-through dispatch already provides the intended
  behaviour. RFC 018 moved to `rfcs/archive/`. An addendum is appended to
  `rfcs/done/007-rule-evaluation-strategy-variants.md` explaining the
  withdrawal and documenting the correct multi-rule-set pattern.
  (`rfcs/`)

- **RFC housekeeping.** Stale `rfcs/proposed/001-008-*.md` duplicates
  removed (were identical to `rfcs/done/` versions except for the Status
  field). `rfcs/README.md` updated with the full Done table
  (RFCs 000–020) and the Archive table.

### Internal

- `regex = "1"`, `globset = "0.4"`, `ignore = "0.4"` added as workspace
  and `apimock-routing` dependencies.
- Internal `cargo` dependency versions updated in `apimock` crate
  Cargo.toml.

### Test count

| Crate | v5.10.1 | v5.11.0 | Delta |
|---|---|---|---|
| apimock (façade) | 17 | 20 | +3 (TLS tests) |
| apimock-config | 49 | 49 | — |
| apimock-routing | 78 | 113 | +35 (header op + rule_op) |
| apimock-server | 5 | 9 | +4 (TLS reload tests) |
| **Total** | **149** | **191** | **+42** |



## [5.10.1] - 2026-05-17

### Fixed

- **Re-export oversight** — The `pub use view::{}` statement in `apimock_config::lib.rs` was missing the 7 types added via RFC 001/002/016. GUI library users were unable to import types like `apimock_config::HeaderConditionPayload` directly. Fixed.

  Types added: `BodyConditionKind`, `BodyConditionPayload`, `BodyOp`, `ConditionWithId`, `HeaderConditionPayload`, `HeaderOp`, `UrlPathOp`.

- **Internal crate version specification outdated** — `Cargo.toml` files for `apimock-config`, `apimock-server`, and `apimock` still specified `version = "5.6.0"` for internal dependencies. Updated to `5.10.1`.



## [5.10.0] - 2026-05-16

### Added (RFC 014–016 — medium-priority pass)

- **RFC 014** — `Headers` backing store changed from `HashMap` to `IndexMap`.
  Header conditions inserted programmatically now iterate in insertion order,
  so `WhenView.headers` from `build_when_view` reflects the order in which
  conditions were added (e.g. via `AddHeaderCondition`). The sort workaround
  in `build_header_condition_views` is removed. `Body`'s inner condition map
  also migrated to `IndexMap` for consistency.
  **Note:** header conditions loaded directly from TOML iterate in the order
  the `toml` crate produces them (typically alphabetical); only programmatic
  insertion is guaranteed ordered. (`apimock-routing`, `apimock-config`)

- **RFC 015** — `apimock match-test` CLI subcommand. Evaluates a rule-set file
  against a synthetic request without starting the server. Flags: `--rule-set`,
  `--rule` (1-based), `--path`, `--method`, `--header` (repeatable),
  `--body` (inline JSON), `--body-file`, `--quiet`. Prints per-condition
  pass/fail with actual values; exit codes 0 (match), 1 (no match), 2 (error).
  Per-condition breakdown covers url_path, method, headers, and body.json
  conditions including the full RFC 008/010 operator set. (`apimock`)

- **RFC 016** — Per-condition `NodeId` addressability. `NodeAddress` gains
  `HeaderCondition { rule_set, rule, header_name }` and
  `BodyCondition { rule_set, rule, path }` variants. Six new `EditCommand`
  variants: `AddHeaderCondition`, `UpdateHeaderCondition`,
  `RemoveHeaderCondition`, `AddBodyCondition`, `UpdateBodyCondition`,
  `RemoveBodyCondition`. GUI code can now add, update, or remove individual
  header/body conditions without replacing the full condition list via
  `UpdateRule`. `ConditionWithId<V>` wrapper type available for snapshot-level
  consumers that need to pair a condition view with its `NodeId`.
  (`apimock-config`)

### Internal

- `indexmap` added as a workspace dependency (v2, serde feature).
- `hyper` and `serde_json` promoted from dev-dependencies to regular
  dependencies in the `apimock` crate (required by the `cmd::match_test`
  library module).
- `NodeAddress` is no longer `Copy` (gained `String` fields). All
  `id_index.rs` and `id_shift.rs` sites updated to use `.clone()` or
  `.cloned()` as appropriate.

### Documentation

- `README.md` — removed the `## Workspace layout (5.0.0+)` block
  (full duplicate of `docs/src/technical-reference/workspace.md`).
- `docs/src/technical-reference/workspace.md` — removed stale
  `5.1.0`/`5.2.0` phasing notes; updated `RulePayload` example to use
  `..Default::default()`; added `ws.save()` to the code sample.

### Test count

| Crate | v5.9.0 | v5.10.0 | Delta |
|---|---|---|---|
| apimock (façade) | 10 | 10+7* | +7* |
| apimock-config | 45 | 49 | +4 |
| apimock-routing | 76 | 78 | +2 |
| apimock-server | 5 | 5 | — |
| **Total** | **136** | **142+7*** | **+6+7*** |

\* apimock crate tests include rhai (large compile time); counted separately.



## [5.9.0] - 2026-05-16

### Added (RFC 009–013 — quality and completeness pass)

- **RFC 009** — Trace socket transport. `TraceTransport::accept_loop` is now fully
  implemented (no longer `unimplemented!()`). UDS (`TraceTransportConfig::Uds`) on
  Unix/macOS removes any stale socket path at startup and accepts up to 4 concurrent
  subscribers. TCP loopback (`TraceTransportConfig::Tcp`) is the portable fallback.
  Subscribers receive newline-delimited JSON with `schema_version: 1` and `dropped_count`
  for gap detection. `MatchTraceEvent` now derives `Serialize`. New TCP integration test.
  (`apimock-server`)

- **RFC 010** — Body match semantics clarified and extended. `null` semantics for
  `Exists`/`Absent` are now documented and tested: a field present with value `null`
  satisfies `Exists`; `Absent` requires the path to be truly missing. New `EqualInteger`
  operator uses `i64` arithmetic to avoid `f64` precision loss for integers above 2^53
  (e.g. snowflake IDs). `BodyOp::EqualInteger` added to `apimock-config`.
  (`apimock-routing`, `apimock-config`)

- **RFC 011** — `RoundRobin` strategy. Cycles through matching rules in list order, one
  per request. State is kept in an `Arc<AtomicUsize>` per rule set (lock-free, shared
  across concurrent handler tasks). Counter resets on server reload. `ServiceStrategy`
  editing accepts `"round_robin"`. (`apimock-routing`, `apimock-config`)

- **RFC 012** — Config-driven `FileTreeFilter`. New `[file_tree_view]` TOML section
  persists filter preferences (`show_hidden`, `builtin_excludes`, `extra_excludes`,
  `include`). `Workspace::snapshot()` and `Workspace::list_directory()` now use the
  configured filter automatically. `RootSettingKey` gains four new variants
  (`FileTreeShowHidden`, `FileTreeBuiltinExcludes`, `FileTreeExtraExcludes`,
  `FileTreeInclude`), all returning `SoftReload`. TOML writer emits the section when
  non-default. (`apimock-config`)

- **RFC 013** — `RulePayload` validation: `url_path_op: Some(_)` with `url_path: None`
  is now a `ApplyError::InvalidPayload` instead of silently discarding the operator.
  Applies to both `AddRule` and `UpdateRule`. (`apimock-config`)

### Test count

| Crate | v5.8.0 | v5.9.0 | Delta |
|---|---|---|---|
| apimock (façade) | 10 | 10 | — |
| apimock-config | 40 | 45 | +5 |
| apimock-routing | 62 | 76 | +14 |
| apimock-server | 3 | 5 | +2 |
| **Total** | **111** | **136** | **+25** |



## [5.8.0] - 2026-05-16

### Added (stage-2 GUI editing surface — RFCs 001–008)

- **RFC 001** — `RulePayload.url_path_op: Option<UrlPathOp>` lets the GUI author
  rules with non-equal URL-path operators (`starts_with`, `contains`, `ends_with`,
  `wild_card`, `not_equal`) without hand-editing TOML. `None` defaults to `Equal`
  (backwards-compatible). (`apimock-config`)

- **RFC 002** — `RulePayload.headers: Option<Vec<HeaderConditionPayload>>` and
  `RulePayload.body: Option<Vec<BodyConditionPayload>>` expose header and body
  conditions through the GUI editing API. `None` preserves existing conditions;
  `Some([])` clears them; `Some([…])` replaces them wholesale. (`apimock-config`)

- **RFC 003** — `RootSettingKey` gains seven new variants: `TlsEnabled`,
  `TlsCertFile`, `TlsKeyFile`, `LogLevel`, `LogFile`, `LogFormat`. TLS and
  listener-address changes return `ReloadHint::restart()`; log-level and strategy
  changes return `ReloadHint::reload()`. New `ReloadHint::for_key(key)` helper.
  (`apimock-config`)

- **RFC 004** — `WhenView.has_header_conditions: bool` / `has_body_conditions: bool`
  replaced with `headers: Vec<HeaderConditionView>` and `body: Vec<BodyConditionView>`.
  The GUI can now render the full condition list in one snapshot pass without a second
  query. **Breaking change**: callers that read the old boolean fields must migrate to
  `headers.is_empty()` / `body.is_empty()`. (`apimock-routing`)

- **RFC 005** — `FileTreeView` now applies a default filter: dotfiles/dot-directories
  and known build-output directories (`target`, `node_modules`, `dist`, `build`,
  `__pycache__`, etc.) are hidden. New `FileTreeFilter` struct and
  `build_file_tree_with` / `list_directory_with` APIs allow custom filter overrides.
  Constant `BUILTIN_EXCLUDES` lists the default excluded names. (`apimock-routing`)

- **RFC 006** — `apimock-server::trace` module: `TraceEmitter` (bounded broadcast
  channel), `MatchTraceEvent`, `RequestSummary`, `Outcome`. Emits one structured
  event per request to in-process subscribers. The out-of-process transport layer
  (UDS / TCP) is explicitly stubbed and deferred to a future release.
  (`apimock-server`)

- **RFC 007** — `Strategy` gains three new variants: `UniformRandom { seed }`,
  `WeightedRandom { seed }`, `Priority { tiebreaker }`. `Rule` gains optional
  `weight: Option<u32>` and `priority: Option<i32>` fields used by the respective
  strategies. A seed-based xorshift64 PRNG provides reproducibility for tests.
  (`apimock-routing`)

- **RFC 008** — Body match language extended from 5 operators to 17. New operators:
  `equal_string` (explicit alias for `equal`), `equal_typed` (JSON-type-aware),
  `equal_number` / `greater_than` / `less_than` / `greater_or_equal` / `less_or_equal`
  (numeric), `exists` / `absent` (path presence), `array_length_equal` /
  `array_length_at_least` / `array_contains` (array predicates). The existing
  `equal` operator retains its 5.7.0 string-coercion semantics for backwards
  compatibility. (`apimock-routing`)

### Internal

- Introduced `BodyConditionStatement` type in the routing crate to carry
  `BodyOperator` instead of the shared `ConditionStatement` + `RuleOp`, keeping
  the header and body operator surfaces cleanly separated.
- `apimock-config::workspace::edit::payload` now converts `UrlPathOp`,
  `HeaderOp`, and `BodyOp` from the payload layer to the routing crate's internal
  types at the apply boundary.
- `body_op_name_pub` exported from `apimock-routing::view::build` so `toml_writer`
  can serialise `BodyOperator` values to TOML without importing routing internals.

## [5.7.0] - 2026-04-28

5.7.0 is a cosmetic / documentation-only release. It closes ROADMAP's
last cosmetic item — 5.5.0 round-trip test fixtures used `body.json`
keys that *looked* like canonical JSONPath (`"$.user.name"`,
`"$.action"`) but aren't supported by the routing crate's dotted-path
mini-syntax (`apimock_routing::util::json::json_value_by_jsonpath`).
The fixtures still passed because they only asserted round-trip
preservation; they never invoked `is_match`. 5.7.0 corrects the
fixtures and strengthens the documentation around the path syntax so
future readers don't repeat the mistake. There are **no behavioural
changes**, **no public-API changes**, and **no new tests** — the same
97 workspace tests pass before and after.

### Changed

- **Test fixtures (`apimock_config::toml_writer::tests` and
  `apimock_config::workspace::tests`).** The two 5.5.0 round-trip
  tests `round_trip_rule_with_body_json` and
  `save_preserves_body_through_disk_round_trip` (the test name is
  approximate — the latter lives near the workspace save tests) now
  use `"user.name"` and `"action"` instead of `"$.user.name"` and
  `"$.action"`. Assertion keys updated accordingly. Tests still
  exercise round-trip preservation; semantics unchanged.
- **Code comment in `apimock_config::toml_writer::request_table`.**
  The block comment that previously documented the body-condition
  TOML form using a `"$.path"` example now uses `"<dotted.path>"`
  with `"order.items.0.product_id"` as a concrete example, plus an
  explicit "not canonical JSONPath" cross-reference to the routing
  crate's `util::json` module.

### Documentation

- **`apimock_routing::util::json::json_value_by_jsonpath` rustdoc.**
  Section header retitled from "Why a home-rolled mini-JSONPath
  instead of a crate" to "Why a home-rolled mini-syntax instead of
  canonical JSONPath", and a paragraph added that explicitly states
  **"This is not canonical JSONPath / RFC 9535"** and explains what
  `"$.foo.bar"` actually matches in this resolver (a top-level `$`
  key — almost certainly not what the writer intended).
- **`apimock_routing::rule_set::rule::when::request::body::Body`
  rustdoc.** Previously had only an `is_match` doc comment. Now the
  type itself carries a docblock describing the dotted-path key
  syntax, explicitly contrasting with canonical JSONPath, and
  cross-linking to `crate::util::json::json_value_by_jsonpath` for
  the full contract.
- **`crates/apimock/examples/config/default/apimock-rule-set.toml`.**
  The single commented-out `body.json` example (`"a.b.c" = { value
  = "d", op = "starts_with" }`) was expanded to three commented
  examples covering nested keys, array indexing
  (`"order.items.0.product_id"`), and a different operator
  (`"user.role" = { op = "contains", value = "admin" }`), with a
  block comment heading that explicitly warns "NOT canonical
  JSONPath — do not write `\"$.a.b.c\"`."
- **`docs/src/advanced-topics/rule-set-config-structure/rules/when.md`.**
  Added a "Note: not canonical JSONPath" blockquote under the
  `when.request.body.json` section, linking to RFC 9535. Other docs
  pages were audited (`getting-started/rule-based-routing-2.md`,
  `examples/combining-conditions-2.md`, `faq.md`,
  `advanced-topics/.../rules/README.md`) and confirmed already
  correct — they use the dotted form throughout.

### ROADMAP

- "5.5.0 round-trip test fixtures used non-existent JSONPath syntax"
  moved from deferred to resolved. The only remaining deferred item
  is now hidden / VCS / build-artifact directory filtering in
  `FileTreeView`.

## [5.6.0] - 2026-04-28

5.6.0 closes ROADMAP's second deferred item: routing-crate test
coverage for `Headers::is_match` and `Body::is_match`. 36 dedicated
tests now exercise these matchers and their TOML deserialise surface
directly inside the `apimock-routing` crate. No public-API changes;
no behavioural changes.

### Added

- **`apimock_routing::rule_set::rule::when::request::headers::tests`**
  — 19 tests:
  - 10 covering `is_match` operator variants: default `Equal` (op
    omitted), explicit `Equal`, `NotEqual` (match and no-match),
    `StartsWith` (match and no-match), `Contains` (match and
    no-match), `WildCard` (match).
  - 4 covering `is_match` request-shape edges: missing header key →
    false, multi-condition AND with all matching, multi-condition AND
    with one failing, UTF-8 decode failure → `true` (pinning the
    log-and-allow contract).
  - 2 covering `validate()`: empty `Headers` → false, non-empty →
    true.
  - 3 covering TOML deserialise: value-only shape (op omitted), all
    five op variants in one document, multiple keys.

- **`apimock_routing::rule_set::rule::when::request::body::tests`**
  — 17 tests:
  - 3 covering `is_match` request-shape edges: no body, no `Json`
    `BodyKind`, empty inner map.
  - 4 covering jsonpath hits with operators: `Equal`, `StartsWith`,
    `Contains`, plus a jsonpath-miss path returning false.
  - 2 covering value coercion: `Number` → `"42"`, `Object` →
    compact JSON `{"k":"v"}`.
  - 2 covering multi-jsonpath AND.
  - 3 covering `validate()`: empty outer, empty inner, non-empty.
  - 3 covering TOML deserialise: simple jsonpath, nested jsonpath,
    multiple jsonpaths.

### Changed (internal)

- `headers.rs` and `body.rs` now declare `#[cfg(test)] mod tests;`.
  The corresponding `headers/` and `body/` directories receive the
  test files. (`body/` already existed for `body_kind.rs`; `headers/`
  is new.)

### Findings during this work

A test-fixture issue was discovered in 5.5.0's round-trip tests: they
used `body.json` keys like `"$.user.name"` and `"$.action"`, syntax
that resembles canonical JSONPath but isn't recognised by the routing
crate. The routing crate's path resolver supports only dotted paths
(`a.b.c` for object keys, `items.2.name` for array indexing); the
leading `$.` is treated as a literal object key and never matches a
real request.

The 5.5.0 tests still pass because they only verify that the
*string* round-trips through TOML — they don't call `is_match`. The
fixtures are misleading but not broken. A 5.7.0-or-later release will
rewrite them to use the supported syntax, bundled with broader
documentation work. Recorded in ROADMAP.md.

### Roadmap

ROADMAP.md is updated:
- The "Routing crate test coverage" item is marked resolved.
- A new cosmetic item is recorded for 5.7.0 candidacy: rewriting
  5.5.0's misleading `$.foo` test fixtures to use the routing
  crate's actual dotted-path syntax.

### Tests

`cargo test --workspace --lib` reports **97 passing** (was 61 in
5.5.0; +36 = 97). The new tests all live in `apimock-routing`; other
crates' counts are unchanged.

## [5.5.0] - 2026-04-28

5.5.0 closes the highest-priority deferred item from ROADMAP.md:
header and body match conditions now round-trip cleanly through
`Workspace::save()`. A rule loaded from a TOML file with `[when.request.headers.*]`
or `[when.request.body.json.*]` sections will preserve those clauses
across save → reload, and edits via `EditCommand::UpdateRule` no
longer silently strip them.

### Fixed

- **`toml_writer` now serialises `Headers` and `Body` conditions.**
  In 5.2.0–5.4.0, `request_table` stripped these clauses on save;
  the only public note was a CHANGELOG line and a long comment in
  the writer source. 5.5.0 produces faithful TOML for both:
  - `[when.request.headers.<key>] op = "...", value = "..."`
  - `[when.request.body.json."<jsonpath>"] op = "...", value = "..."`
  Keys are sorted at write time so a save → save sequence produces
  byte-identical output (the underlying `HashMap` deserialise
  doesn't preserve order, so the round-trip text won't either, but
  re-saves after the first will be stable).
- **`UrlPathConfig::Detailed` operator round-trip.** The previous
  writer used `format!("{}", op)` for `RuleOp`, which produced the
  human-readable `Display` form (`" == "`, `" starts with "`).
  Deserialisation expects the snake_case serde form (`equal`,
  `starts_with`). Detail-shape URL-path rules now use the routing
  crate's canonical `op_name` helper.

### Changed (behaviour, not API signature)

- **`EditCommand::UpdateRule` now preserves unspecified fields.** The
  GUI's `RulePayload` carries only `url_path`, `method`, and `respond`.
  In 5.1.0–5.4.0, an `UpdateRule` would silently strip any `headers`
  or `body.json` conditions that lived on the rule but weren't
  surfaced through the payload. 5.5.0 carries those clauses forward:
  the new rule keeps whatever conditions the previous rule had.
  - This is a *behaviour* change but not a *signature* change. The
    `EditCommand` and `RulePayload` shapes are byte-identical to
    5.4.0; consumers that were silently relying on the field-stripping
    behaviour need to add an `UpdateRule` variant that explicitly
    blanks the clauses they want gone (none exist today since the
    payload doesn't carry the relevant fields). In practice, every
    consumer is a beneficiary, not a victim.
  - `EditCommand::UpdateRule`'s rustdoc documents the preservation
    semantics so future readers understand the contract.

### Routing crate API surface

The following modules were promoted to `pub mod` so the round-trip
writer can read them:
- `apimock_routing::rule_set::rule::when::condition_statement`
- `apimock_routing::rule_set::rule::when::request::body`
- `apimock_routing::rule_set::rule::when::request::body::body_kind`
- `apimock_routing::rule_set::rule::when::request::headers`

`apimock_routing::view::build::op_name` was promoted to `pub fn` for
use by `apimock_config::toml_writer`.

These are additive; no existing item changed visibility downward.

### Tests

Added 7 round-trip tests:
- 4 in `apimock_config::toml_writer::tests` — single header, header
  with operator, multiple headers, `body.json` with jsonpath.
- 3 in `apimock_config::workspace::tests` — header preservation
  through Workspace save/reload, body preservation through save/reload,
  in-memory preservation across an `UpdateRule` apply (the
  semantics-change verification).

`cargo test --workspace --lib` reports 61 passing (was 54 in 5.4.0;
+7 = 61).

### Roadmap

ROADMAP.md is updated:
- The "Header / body.json round-trip" item from 5.2.0 is marked
  resolved.
- A new deferred item is recorded for 5.6.0: routing-crate test
  coverage for `Headers::is_match`, `Body::is_match`, and the TOML
  deserialisation surface for both. The 5.5.0 round-trip tests
  exercise these paths indirectly via `apimock-config`, but the
  routing crate has no dedicated tests for them. 5.6.0 will add
  ~28 dedicated tests as a focused routing-crate release.

## [5.4.0] - 2026-04-28

5.4.0 is a refactor with no behavioural change. The
`apimock_config::workspace` module — which had grown to almost 1,900
lines as features stacked up over 5.0–5.3 — splits into nine focused
sibling files. The public API is unchanged; every signature, every
field, every visibility level is the same as in 5.3.0.

### Changed (internal layout only)

- **`crates/apimock-config/src/workspace.rs`** trimmed from 1,847 to
  259 lines. Now contains only:
  - the `Workspace` struct definition
  - `load()` and `seed_ids()`
  - the small accessors (`config`, `root_path`, `list_directory`,
    `config_relative_dir`, `resolve_relative`)
  - module declarations for the submodules below.

- **New submodules under `crates/apimock-config/src/workspace/`:**

  | file | content |
  | --- | --- |
  | `id_index.rs` | `NodeAddress` enum + `IdIndex` map |
  | `snapshot.rs` | `snapshot()`, `root_file_nodes()`, `rule_set_file_view()`, `summarise_respond()` |
  | `validate.rs` | `validate()`, `collect_diagnostics()`, `respond_node_validation()` |
  | `save.rs` | `save()`, `has_unsaved_changes()`, `atomic_write()` |
  | `diff.rs` | `compute_diff_summary()`, `append_per_rule_diff()`, `rule_to_string()` |
  | `edit.rs` | `apply()` dispatch + the eight `cmd_*` handlers |
  | `edit/id_shift.rs` | `shift_rule_sets_down()`, `shift_rules_down()`, `reorder_rule_ids()` |
  | `edit/payload.rs` | `build_rule_from_payload()`, `build_respond_from_payload()`, `value_as_*`, `internal_path_err()` |
  | `path_helpers.rs` | `file_basename()`, `resolve_root()` |
  | `tests.rs` | added explicit imports (was `use super::*;` from a `Workspace` parent that re-imported view types); behaviour unchanged |

  Each file gets a module-level rustdoc explaining what it owns and
  why it's grouped that way.

### Why

The single-file `workspace.rs` had become a navigability problem.
Every new feature (Step 2 apply, Step 3 validate, Step 4 save, Step 5
routing snapshot, per-rule diff) appended to the same file, leaving
nine logical concerns sharing one buffer. Splitting along
responsibility lines makes each file readable on its own and clarifies
the dependency direction (id_index ← edit ← snapshot/validate/save).

### Verified

- `cargo check` (dev + release): clean
- `cargo check --benches --examples`: clean
- `cargo test --workspace --lib`: **54 passed** (same as 5.3.0)
- `cargo doc --no-deps`: clean (one pre-existing warning in
  `apimock-server`, unrelated to this refactor)

### Public API impact

None. The crate's external surface — `Workspace`, `EditCommand`,
`SaveResult`, `DiffItem`, etc. — has the same signatures, the same
field shapes, the same visibility. Existing consumers don't need to
change anything.

## [5.3.0] - 2026-04-27

5.3.0 implements Step 5 of the GUI extension plan: a populated
routing snapshot. The `RouteCatalogSnapshot` returned by
`Workspace::snapshot()` now carries real data instead of an empty
placeholder, plus a depth-1 eager file-tree view of the fallback
respond directory and minimal script-route info for Rhai middlewares.

This release also extends the per-save diff summary (a 5.2.0 carry-
over) to surface per-rule changes, not just per-rule-set changes.

### Added

- **`apimock_routing::view::build`** — public builder functions for
  every view type. The config crate uses these to populate
  `Workspace::snapshot().routes`. Free functions rather than `From`
  impls because the views need contextual data (positional indices)
  that the source types don't carry.
- **Structured `WhenView`** replaces the 5.0–5.2 `RuleView.when_summary:
  String`. Now `RuleView.when: WhenView { url_path, method,
  has_header_conditions, has_body_conditions }` per spec §5.3.
  `RuleView::summary()` reproduces the old string format for callers
  that want a one-line label.
- **`UrlPathView { value, op }`** — URL-path predicate detail with
  the matching operator name in lowercase TOML form (`equal`,
  `starts_with`, etc.).
- **`FileTreeView` + `FileNodeView` + `FileNodeKind`** — depth-1
  eager view of the fallback respond directory. Each top-level entry
  is reported; subdirectories carry `children: Some(Vec::new())` to
  flag them as "expandable but not yet expanded". The embedder calls
  `Workspace::list_directory(path)` to load a subdirectory's
  contents on demand. Files include a `route_hint` (the URL path the
  dyn-route fallback would serve them at, e.g. `/users` for
  `users.json`).
- **`ScriptRouteView { index, source_file, display_name }`** —
  minimal info for each Rhai middleware. Static analysis of "what
  URLs does this script handle" isn't feasible without parsing Rhai;
  the view reports only what we know statically.
- **`Workspace::list_directory(&Path) -> Vec<FileNodeView>`** — the
  on-demand expansion API for file-tree subdirectories. Path-based
  rather than NodeId-based because file-tree entries don't share the
  editable-node ID space (their lifecycle reflects the filesystem,
  not the model).
- **Per-rule diff in `SaveResult.diff_summary`.** When a rule set
  diverges from its baseline, the diff now walks the rules pairwise
  and emits one `DiffItem` per changed rule (`Updated` for content
  changes, `Added` for rules past the baseline length, `Removed`
  for rules dropped from the baseline). Rule-set-level `Added`
  still appears for newly-introduced rule sets.

### Changed

- **`RouteCatalogSnapshot` shape** extended (additive — the type was
  already `#[non_exhaustive]`):
  - `+ file_tree: Option<FileTreeView>`
  - `+ script_routes: Vec<ScriptRouteView>`
- **`RuleView.when_summary: String` removed**, replaced by `when:
  WhenView`. The new `RuleView::summary()` accessor produces the same
  string the old field did, so call sites that just wanted a one-line
  label keep working with a one-character `.summary()` change.

### Project documentation

- **`ROADMAP.md` added** at the workspace root. Currently records
  two deferred items: (1) hidden-folder filtering in `FileTreeView`
  and (2) header / body.json round-trip through `toml_writer`. Both
  have full rationale recorded so the original context isn't lost
  between releases.

### Tests

`cargo test --workspace --lib` reports 54 passing (was 49 in 5.2.0).
The 5 new tests cover the route-catalog populate path, structured
`WhenView` summary formatting, file-tree depth-1 eager population
with on-demand subdirectory expansion, per-rule diff after a rule
edit, and `ScriptRouteView` presence with middlewares configured.

### Status against the spec §13 受け入れ条件

| condition | status |
| --- | --- |
| GUI が TOML を意識せず編集できる | 5.1.0 |
| ルール追加・削除・更新が可能 | 5.1.0 |
| 保存前に検証できる | 5.1.0 |
| 差分が取得できる | 5.2.0 (rule-set granularity), 5.3.0 (per-rule) |
| reload 必要性が判断できる | 5.2.0 |
| 既存サーバー動作が維持される | all releases |

§12 Steps 1–5 all complete.

## [5.2.0] - 2026-04-26

5.2.0 implements Step 4 of the GUI extension plan: `Workspace::save()`
and the per-node diff summary. With this release a GUI can complete
the full edit cycle — load → snapshot → apply → validate → **save** —
without ever touching TOML text.

### Added

- **`Workspace::save() -> Result<SaveResult, SaveError>`.** Writes
  every editable file whose rendered representation has diverged from
  the load-time baseline. Files whose rendered output already matches
  baseline are skipped, so unedited files keep their hand-formatting.
  Atomic writes via `tempfile::NamedTempFile::persist` (POSIX
  `rename(2)` / Windows `MoveFileExW`) so a concurrent reader can't
  observe a half-written TOML file.
- **`Workspace::has_unsaved_changes() -> bool`.** Cheap polling for
  GUIs to drive an "unsaved changes" indicator. No I/O — renders the
  current model and compares to the in-memory baseline.
- **`SaveResult.diff_summary`.** One `DiffItem` per node whose
  rendered representation has changed since load. 5.2.0 emits diffs
  at rule-set granularity (`Updated` / `Added`); per-rule diffs are
  a candidate for stage-5.
- **`SaveResult.requires_reload`.** True for any save that wrote at
  least one file. The server-side `ReloadHint::Restart` distinction
  for listener changes is exposed through the existing
  `apimock_server::ReloadHint` conversions.
- **`apimock_config::toml_writer`** (private module) — hand-rolled
  TOML rendering for the editable subset of `Config` and `RuleSet`.
  Produces `toml::Value::Table` trees and serialises via
  `toml::to_string_pretty`. See the module docstring for the
  rationale on hand-built TOML over `Serialize` derives.

### Dependencies

- `tempfile = "3"` promoted from dev-dep to runtime dep on
  `apimock-config`. `Workspace::save` uses it for atomic writes.

### Compatibility

- TOML round-trip through save **does not preserve comments,
  blank lines, or key ordering**. Spec §6 marks comment preservation
  as best-effort and §11 marks "complete comment preservation" as
  an explicit non-goal; this is the documented trade-off. A polished
  GUI is encouraged to warn users editing files that contain
  comments before they trigger a save.
- Rule-set rules with `headers` or `body.json` match conditions
  parse cleanly into the in-memory model but are not currently
  re-serialised by `toml_writer` (the routing crate's `Headers` /
  `Body` types don't expose their internal map shape outside the
  crate). Editing a rule-set that contains such conditions is
  supported, but saving the file will drop the conditions. Stage-5
  will round-trip these once routing exposes the necessary types.

### Tests

`cargo test --workspace --lib` reports 49 passing (was 44 in
5.1.x): 5 new `Workspace::save` tests covering no-op save,
rule-set edit + round-trip through disk, root edit + reload flag,
post-save TOML parseability, and `has_unsaved_changes` lifecycle.

### Status against the spec §13 受け入れ条件

- ✅ GUI が TOML を意識せず編集できる (NodeId-targeted EditCommand)
- ✅ ルール追加・削除・更新が可能 (5.1.0)
- ✅ 保存前に検証できる (5.1.0)
- ✅ **差分が取得できる (`SaveResult.diff_summary`, 5.2.0)**
- ✅ reload 必要性が判断できる (`SaveResult.requires_reload`, 5.2.0)
- ✅ 既存サーバー動作が維持される

§12 Step 5 (richer routing snapshot) remains for 5.3.0 by prior
agreement.

## [5.1.1] - 2026-04-26

5.1.1 is a project-layout refactor with no behavioural change. Each
member crate of the workspace now lives in its own directory under
`crates/`, including the `apimock` façade itself which was previously
co-located with the workspace root.

### Changed

- **`apimock` façade moved to `crates/apimock/`.** Source files
  (`src/`), benches, examples, and integration tests followed it.
  The workspace root now contains only the workspace definition and
  shared metadata — no individual crate's package data.
- **Workspace root `Cargo.toml` slimmed** to four sections:
  `[workspace]`, `[workspace.package]`, `[workspace.dependencies]`,
  and the `[profile.*]` blocks (which must live at workspace root for
  cargo to apply them). Everything that used to belong to the
  `apimock` package — features, deps, dev-deps, bench registrations —
  moved to `crates/apimock/Cargo.toml`.
- **`apimock-config::workspace` test module extracted** to a sibling
  file `src/workspace/tests.rs`. The implementation file
  (`workspace.rs`) drops from 1,843 to 1,395 lines; behaviour is
  unchanged.

### Why

The 5.0–5.1 series accumulated a layout where the workspace root and
the façade crate were mixed in one `Cargo.toml`. That worked but mixed
two levels of concern in a single file. Splitting them out makes each
file responsible for one thing and brings the façade in line with how
the other member crates are organised. End users (`cargo install
apimock` / `npx apimock`) see no change.

### Tests

`cargo test --workspace --lib` reports 44 passing — same as 5.1.0.

## [5.1.0] - 2026-04-26

5.1.0 implements the GUI-facing extension layer specified in the
project's "GUI 向け機能拡張開発指示書". The headline addition is
`apimock_config::Workspace` — an editable façade that lets a future
GUI manipulate apimock configuration through structured commands
without touching TOML directly.

This is **not a breaking change** for existing users. The CLI binary,
the on-disk file formats, and every pre-5.0 import path that 5.0.0
already preserved continue to work unchanged.

### Added

- **`apimock_config::Workspace`** — the new top-level handle a GUI
  uses to edit a loaded configuration:
  - `Workspace::load(path)` reads `apimock.toml` and every file it
    references (sharing `Config::new`'s loader so the running server
    sees the same data).
  - `Workspace::snapshot()` produces a fully-owned `WorkspaceSnapshot`
    suitable for serialisation, IPC, or rendering.
  - `Workspace::apply(EditCommand)` mutates the in-memory model.
  - `Workspace::validate()` runs per-node validation.
  - `Workspace::save()` is reserved for 5.2.0; the placeholder
    returns `SaveError::Inconsistent` so callers see the gap clearly.

- **Stable per-node identifiers (`NodeId`).** Every editable node is
  assigned a v4 UUID at load time. IDs survive `apply()` calls within
  one Workspace instance, including operations that shift positions
  (Delete / Move). A GUI selection set anchored on NodeIds therefore
  remains valid across edits.

- **Eight `EditCommand` variants** matching the spec exactly:
  - `AddRuleSet { path }` / `RemoveRuleSet { id }`
  - `AddRule { parent, rule }` / `UpdateRule { id, rule }` /
    `DeleteRule { id }` / `MoveRule { id, new_index }`
  - `UpdateRespond { id, respond }`
  - `UpdateRootSetting { key, value }` with a typed
    `RootSettingKey` enum (`ListenerIpAddress`, `ListenerPort`,
    `ServiceFallbackRespondDir`, `ServiceStrategy`)

- **Per-node validation.** `validate()` produces a `ValidationReport`
  whose `diagnostics` carry `NodeId` + file + severity + message.
  Apply-time validation runs the same pass so `ApplyResult.diagnostics`
  reflects the post-mutation state. A GUI can render a red underline
  on the offending node directly from the diagnostic's `node_id`.

- **`uuid = "1"` (with `v4` and `serde` features) is now a direct
  dependency of `apimock-config`.**

### Changed

- `view::ReloadHint` reshape: was a 3-variant enum
  (`None / Reload / Restart`) in 5.0.0; is now a struct
  `{ requires_reload, requires_restart }` per spec §9. The
  `apimock-server::control::ReloadHint` enum mirror remains and the
  bidirectional `From` impls have been updated so existing server-side
  pattern matching still works.

- `view::WorkspaceSnapshot` reshape: now `{ files, routes, diagnostics }`
  per spec §4.2 (was `{ root, rule_sets, middlewares, diagnostics }`).
  Each `ConfigNodeView` now carries the six spec-mandated fields
  (`id`, `source_file`, `toml_path`, `display_name`, `kind`, `validation`).

- `view::EditCommand` reshape: was 4 placeholder variants in 5.0.0; is
  now the 8 spec-defined variants, all targeted by NodeId rather than
  positional path.

### Status against the spec

- ✅ §12 Step 1 (Workspace + snapshot) — implemented.
- ✅ §12 Step 2 (EditCommand + apply) — implemented; all 8 variants.
- ✅ §12 Step 3 (validation + diagnostics) — implemented; per-node.
- ⏳ §12 Step 4 (save + diff) — placeholder; planned for 5.2.0.
- ⏳ §12 Step 5 (richer routing snapshot) — placeholder; planned for 5.2.0.

This matches the (A)=3 scope agreed before implementation.

### Tests

`cargo test --workspace --lib` reports 44 passing (was 31 at 5.0.0):
- 19 in `apimock-config` (6 path_util + 13 Workspace tests covering
  every `EditCommand` variant, ID stability across shift / move,
  validation diagnostics, and the three `ApplyError` paths)
- 15 in `apimock-routing` (unchanged from 5.0.0)
- 10 in `apimock` façade (unchanged from 5.0.0)
- 0 in `apimock-server` (none ported when the crate was carved out)

## [5.0.0] - 2026-04-23

5.0.0 splits apimock into a Cargo workspace with three responsibility-
focused library crates behind a thin façade, in preparation for a GUI
front-end that can depend on a stable config + routing API without
reaching into the HTTP runtime.

This is a **breaking change** for library consumers. CLI behaviour,
config file formats, and on-disk layouts are unchanged — `cargo install
apimock` / `npx apimock` work exactly as before.

### Workspace layout

| crate | responsibility |
| --- | --- |
| `apimock-config` | TOML config model — loading, validation, relative-path resolution, and the stage-1 GUI-facing edit / snapshot API shape. |
| `apimock-routing` | Rule-set definitions, request matching, and the stage-1 read-only view types a future GUI binds against. |
| `apimock-server` | HTTP(S) listener, per-request dispatch, Rhai middleware compilation, response building. |
| `apimock` (façade) | CLI entry point, logger installation, `App` composition. Re-exports the three member crates for backwards-compatible `use apimock::routing::...` paths. |

### Added

- **GUI-facing stage-1 view API.** Every type a future GUI will depend
  on is defined with its field shape + rustdoc — ready to be populated
  in stage 2 without further signature churn. Highlights:
  - Routing: `RouteCatalogSnapshot`, `RuleSetView`, `RuleView`,
    `RespondView`, `RouteMatchView`, `RouteValidation`.
  - Config: `WorkspaceSnapshot`, `ConfigFileView`, `ConfigNodeView`,
    `EditCommand`, `EditTarget`, `EditValue`, `ApplyResult`, `SaveResult`,
    `ReloadHint`, `Diagnostic`.
  - Server: `ServerHandle`, `ServerControl`, `ServerState`, `ReloadHint`.
  - All are `#[non_exhaustive]` so stage-2 fills in fields additively.

- **Per-crate error types.** `apimock_routing::RoutingError`,
  `apimock_config::ConfigError`, `apimock_server::ServerError`. Each
  `#[from]`-wraps the layer below, so `?` propagation works unchanged
  in practice; pattern-matching callers can now see which layer the
  failure originated in.

### Changed (breaking)

- Module paths changed for all library consumers. Map:

  | 4.8.0 path | 5.0.0 path |
  | --- | --- |
  | `apimock::core::config::Config` | `apimock::config::Config` or `apimock_config::Config` |
  | `apimock::core::server::routing::rule_set::RuleSet` | `apimock::routing::RuleSet` or `apimock_routing::RuleSet` |
  | `apimock::core::server::Server` | `apimock::server::Server` or `apimock_server::Server` |
  | `apimock::core::error::AppError` | split into the three per-crate errors above |
  | `apimock::core::app::App` | `apimock::App` |
  | `apimock::core::args::EnvArgs` | `apimock::EnvArgs` |

- `ServiceConfig` no longer carries `Vec<MiddlewareHandler>`.
  Compiled Rhai middlewares are now a separate
  `apimock_server::LoadedMiddlewares` value, built from
  `ServiceConfig::middlewares_file_paths` at server startup. This
  removes a cross-layer leak (a config struct must not hold hyper-
  producing runtime objects).
- `Respond::response(...)` method removed. The equivalent free
  function `apimock_server::respond_response::respond_response(...)`
  replaces it. Rationale: the routing crate must stay free of hyper
  body-construction so a GUI can depend on it cheaply.
- `Server::new(AppState)` → `Server::new(Config)`. `AppState` is now
  internal to `apimock-server` and combines `Config` + `LoadedMiddlewares`.

### Not changed

- Binary behaviour (`apimock`, `npx apimock`, `apimock --init`, `apimock -p`, `-d`, `-c`) is byte-identical to 4.8.0.
- Config file formats (`apimock.toml`, rule-set TOML, middleware `.rhai`) are unchanged.
- Performance: both bench suites produce the same numbers as on 4.8.0 (expected — the same code runs, just organised differently).
- Interactive `--init` flow from 4.8.0: unchanged.
- Existing CHANGELOG entries below.

### Migration note

If you were importing from `apimock::core::...` directly, the simplest
migration is to replace the `core::` prefix with the member crate name
(`config::`, `routing::`, or `server::`). The façade re-exports all
three under those names. If you had a `use apimock::core::error::AppError;`,
pick whichever of `ConfigError` / `RoutingError` / `ServerError`
matches the failure you were catching — or use `anyhow::Error` at your
process boundary (that's what the binary does now).

## [4.8.0] - 2026-04-23

4.8.0 focuses on making the project approachable on first contact and on
giving contributors an honest way to track performance. No runtime
dependency additions, no breaking changes to config files or public
library API.

### Added

- **Interactive `apimock --init`.** Prompts for listener IP, port,
  fallback response directory, whether to include a rule-set file, a
  middleware file, and a TLS section, then writes `apimock.toml`
  (templated from your answers) plus the optional companion files.
  - `--yes` / `-y` skips every prompt and uses the 4.7.0 defaults.
  - When stdin is not a TTY (CI, pipes, Docker build), prompts are
    skipped automatically and defaults are used — 4.7.0 non-interactive
    behaviour is preserved exactly.
  - Idempotent: an existing `apimock.toml` still triggers an early
    warn+exit, so repeatable `apimock --init` scripts keep working.
  - Implemented in ~350 lines of plain `std::io` (no `dialoguer` /
    `inquire` / `requestty`) to respect the project's small-footprint
    promise.
- **Benchmarks via criterion 0.8.**
  - `benches/routing.rs` — pure-CPU `RuleSet::find_matched` microbenches,
    parametrised over rule-set sizes 1 / 10 / 100.
  - `benches/response_latency.rs` — end-to-end HTTP latency across five
    response kinds (text, status-only, file-warm, dyn-route fallback,
    404) using criterion's `async_tokio` support.
  - `examples/bench_load.rs` — standalone load sampler that drives the
    server at a target RPS and emits CSV samples of RSS, CPU ticks,
    in-flight requests, and latency every 100 ms. Covers the
    memory / CPU axes that criterion doesn't measure. Reads
    `/proc/self/{status,stat}` directly (no new crates); Linux-only for
    those two columns, latency/throughput columns work everywhere.

### Changed

- `EnvArgs::default()` now reads `--yes` / `-y` in addition to the
  existing flags. No source-compatible breakage — the flag is purely
  additive.
- `Cargo.toml` dev-deps gain `criterion`, `tempfile`, and `log` with the
  `std` feature (needed by benches / example only; release builds
  unaffected).

## [4.7.0] - 2026-04-23

4.7.0 is an internal-cleanup release. Public CLI behaviour and config file
formats are unchanged — but a few library-level signatures changed for
contributors:

- **Typed errors.** Startup-time failures (missing config, unreadable TLS,
  malformed rule set, Rhai compile error, …) now flow through a single
  `apimock::core::error::AppError` enum (powered by `thiserror`) instead of
  `String` / `panic!`. `App::new`, `apimock::new`, `Config::new`,
  `RuleSet::new`, `Server::new`, `MiddlewareHandler::new`,
  `load_certs` / `load_private_key` and `EnvArgs::default` now return
  `AppResult<T>`. The binary entry point uses `anyhow::Result` at the
  process boundary so typos in user config produce a single-line error
  instead of a backtrace.
- **No more `unwrap` / `expect` on hot paths.** The HTTP `accept` loops,
  TLS handshakes, `ParsedRequest` construction, and per-request logging
  all degrade gracefully instead of panicking.
- **Lower nesting.** `Respond::response`, `ParsedRequest::from` and
  `dyn_route_content` are shallower now, with early returns and
  `let … else` in place of long `if let … else` chains.
- **Richer rustdoc.** Key modules and functions now document *why* they
  exist and *why* they're shaped the way they are, not just what they do.

