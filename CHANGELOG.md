# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [5.4.0] - 2026-04-27

5.4.0 is a refactor-only release. The behaviour of the library and
the CLI is byte-identical to 5.3.0; the source tree of
`apimock-config::workspace` is reorganised so the implementation file
no longer exceeds 1,800 lines.

### Changed

- **`crates/apimock-config/src/workspace.rs` split into a parent
  module + 9 sibling files.** The parent file now holds only the
  `Workspace` struct definition, the `load` / `seed_ids` lifecycle,
  and the small public accessors (`config`, `root_path`,
  `list_directory`, plus the `config_relative_dir` /
  `resolve_relative` helpers used by sibling modules). Every other
  concern moves to a dedicated file:

  | file | content |
  | --- | --- |
  | `workspace/id_index.rs` | `NodeAddress` + `IdIndex` machinery |
  | `workspace/snapshot.rs` | `Workspace::snapshot()` + per-file view builders |
  | `workspace/edit.rs` | `Workspace::apply()` + the eight `cmd_*` handlers |
  | `workspace/edit/id_shift.rs` | `shift_rule_sets_down`, `shift_rules_down`, `reorder_rule_ids` |
  | `workspace/edit/payload.rs` | `EditCommand` payload → routing-model converters |
  | `workspace/validate.rs` | `Workspace::validate()` + `respond_node_validation` |
  | `workspace/save.rs` | `Workspace::save()` + `has_unsaved_changes()` + `atomic_write` |
  | `workspace/diff.rs` | `compute_diff_summary` + per-rule diff walker |
  | `workspace/path_helpers.rs` | `file_basename`, `resolve_root` |

  `workspace/tests.rs` is unchanged in content; only its import lines
  were updated to name `crate::view::*` items explicitly now that
  `super::*` resolves to the slimmed parent module.

- **Field visibility on `Workspace`** changed from private to
  `pub(super)` so sibling modules under `workspace/` can read
  `config`, `root_path`, `ids`, `diagnostics`, and `baseline_files`
  directly. The struct itself remains `pub`; nothing changes for
  external consumers.

- **`apimock_config::toml_writer::rule_table`** stays at `pub(crate)`
  (already the case in 5.3.0). No change to the `toml_writer`
  surface.

- **One dead helper removed**: `routing_to_config` was marked
  `#[allow(dead_code)]` in 5.3.0 with no callers; deleted.

### Why this refactor

A single 1,847-line implementation file made navigation and code
review increasingly costly. Splitting along responsibility lines —
*identity*, *reading*, *mutating*, *persisting*, *validating* — keeps
each file focused on one concern and lets readers jump to the right
file by name. Per-module `//!` docs explain the why for each
grouping.

### No behavioural change

- `cargo test --workspace --lib`: 54/54 pass (same as 5.3.0).
- `cargo check --release`: clean.
- `cargo check --benches --examples`: clean.
- `cargo doc --no-deps`: clean.
- Public API surface: identical. The only signature-level change is
  that internal helper methods on `Workspace` (`collect_diagnostics`,
  `shift_rule_sets_down`, `shift_rules_down`, `reorder_rule_ids`,
  `compute_diff_summary`) now carry `pub(super)` visibility instead
  of plain private. They remain unreachable from outside the
  workspace module.

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

