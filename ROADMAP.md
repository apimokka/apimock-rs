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

**Status:** ✅ **Resolved in 5.6.0.** Added 36 dedicated tests in the routing crate covering every operator variant of `Headers::is_match`, the request-shape edge cases (key missing, UTF-8 decode failure), `Body::is_match` across jsonpath hits / misses / value-type coercion, multi-condition AND for both Headers and Body, `validate()` for both, and the TOML deserialise surface. Tests live in `headers/tests.rs` and `body/tests.rs` to follow the existing routing-crate convention.

### 5.5.0 round-trip test fixtures used non-existent JSONPath syntax

**Identified during:** 5.6.0 routing-crate test work.

**Status:** Cosmetic issue. **5.7.0 candidate.**

**Description.** The 5.5.0 round-trip tests in `apimock_config::toml_writer::tests` and `apimock_config::workspace::tests` used `body.json` keys like `"$.user.name"` and `"$.action"` — syntax that *looks* like canonical JSONPath but isn't supported by the routing crate. The routing crate's path resolver (`apimock_routing::util::json::json_value_by_jsonpath`) only understands a dotted-path mini-syntax: `a.b.c` for nested object keys and `items.2.name` for array indexing. The leading `$.` form is not recognised — the resolver treats `$` as a literal object key, which never matches.

**Why the 5.5.0 tests still pass.** The 5.5.0 tests verify *round-trip* (load → save → reload preserves the string), and arbitrary strings round-trip just fine. They never call `is_match`, so the syntax error has no observable effect.

**Why this is deferred.** Fixing the test fixtures is purely cosmetic — they pass, just with misleading example paths. Rewriting them in a 5.7.0-or-later release lets us bundle the change with broader documentation work (e.g. the `apimock` example TOML files could carry richer `body.json` examples too). 5.6.0's tests use the correct dotted-path format, so the documentation truth is now in the codebase.

**Suggested approach.** When eventually addressed: rewrite the 5.5.0 fixtures to use forms like `"action"`, `"user.name"`, etc. Add a brief note to `apimock_routing::util::json` rustdoc and `apimock_config::toml_writer` reminding writers that `body.json` keys are dotted paths, not canonical JSONPath.
