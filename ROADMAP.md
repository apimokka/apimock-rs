# Roadmap

This file records design questions that have been identified during
development but intentionally postponed to a later release. Items
here are *not* bugs — they're follow-on work whose right shape is
easier to decide after some related primary feature has shipped.
Recording the rationale here prevents the original context from being
lost between releases.

## Deferred items

### Hidden / VCS / build-artifact directory filtering in `FileTreeView`

**Identified during:** 5.3.0 design discussion (after `FileTreeView`
was added as part of routing snapshot enrichment, spec §5.5).

**Status:** Deferred. No release scheduled. Pick up when there's
concrete user feedback on what clutters the GUI tree view.

**Description.** `Workspace::snapshot()` produces a `FileTreeView` of
the fallback respond directory. As of 5.3.0, the depth-1 eager
population strategy enumerates the top-level directory verbatim — no
entries are filtered. A `.git`, `node_modules`, `target`, or
`.DS_Store` entry that happens to live at the top level of the
fallback respond dir appears alongside the user's mock data.

**Why this is deferred.**

- *Performance is not affected.* depth-1 eager only lists the top
  level; subdirectory contents are loaded on demand when a GUI
  explicitly calls `Workspace::list_directory(parent_id)`. A `.git`
  entry has the same display cost as any other single directory
  entry — the heavy contents are never enumerated unless the user
  clicks to expand. The "hidden folder + lazy expansion" combination
  doesn't compound into a pathological case.
- *No safety risk.* apimock is a mock-server development tool, so a
  rendered `.git/config` doesn't escalate to anything more serious
  than a noisy GUI. The runtime `dyn_route` fallback that serves
  files matching incoming URL paths has always been
  filter-agnostic, and retroactively filtering it would break
  legitimate uses such as serving `/.well-known/security.txt`.
- *No agreed shape.* Candidate filtering strategies — dotfile prefix,
  hardcoded denylist, `.gitignore` parsing, user-configurable
  patterns — each have trade-offs that are easier to evaluate once
  there's a real GUI built against `FileTreeView` and concrete
  feedback on what users find annoying.

**Suggested approach when picked up.**

A two-step plan that doesn't lock in long-term policy:

1. Apply a minimal default filter (dotfile prefix only — entries
   whose `file_name()` starts with `.`) to `FileTreeView` only.
   Leave `dyn_route` untouched.
2. Make the filter override-able via `Workspace::load_with_options`
   or a builder, so a GUI that wants to show dotfiles can opt out.

This belongs in the routing crate, ideally co-located with the
existing file-tree builder.

### Header / body.json round-trip through `toml_writer`

**Status:** ✅ **Resolved in 5.5.0.** The headers / body / condition_statement / body_kind modules in the routing crate were promoted to `pub mod`, exposing the existing public-field `Headers`, `Body`, `ConditionStatement`, and `BodyKind` types. `toml_writer::request_table` now round-trips these conditions, and `EditCommand::UpdateRule` preserves them when the GUI's `RulePayload` (which doesn't surface these fields) calls back into the apply layer.

### Routing crate test coverage for `Headers::is_match` and `Body::is_match`

**Identified during:** 5.5.0 design discussion.

**Status:** Deferred to **5.6.0**. Tracked as a separate routing-crate-focused release.

**Description.** Routing-crate test coverage today is concentrated in `RuleOp::is_match` (5 tests across the `Equal` / `NotEqual` / `StartsWith` / `Contains` / `WildCard` variants) and `util::glob`. The broader matching pipeline that wires those operators into request evaluation — `Headers::is_match`, `Body::is_match`, and the TOML deserialisation surface for both — has *no* dedicated tests in the routing crate. The 5.5.0 round-trip tests in `apimock-config` exercise the deserialise → serialise path indirectly, but the routing crate alone has no contract tests for these types.

**Why this is deferred.** The 5.5.0 release scope was "fix the round-trip bug surfaced in 5.2.0"; broadening into a routing-crate test campaign would have mixed two concerns into one CHANGELOG entry. 5.6.0 will add ~28 dedicated tests covering:

- `Headers::is_match`: every operator variant (with and without match), missing-key behaviour, the UTF-8 decode failure path that returns `true` after logging, and multi-condition AND evaluation.
- `Body::is_match`: non-JSON body, jsonpath miss, jsonpath hits with each operator, multi-jsonpath AND, and value coercion (`Number` / `Object` → string for matching).
- TOML deserialise: simple and detailed shapes, nested jsonpath keys, the `op` default (`Equal`) when omitted.

**Suggested approach.** Keep tests inside the routing crate's existing `tests` submodule pattern (`headers/tests.rs`, `body/tests.rs`). Use `toml::from_str` to construct fixtures rather than building `Headers` / `Body` values by hand — this verifies the serde wiring at the same time. A `RuleSet::new` test path may also be useful to cover the prefix-resolution + validation flow with these conditions present.
