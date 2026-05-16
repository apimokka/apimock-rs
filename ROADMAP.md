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

**Status:** ✅ **Resolved in 5.7.0.** The 5.5.0 round-trip tests in `apimock_config::toml_writer::tests` and `apimock_config::workspace::tests` used `body.json` keys like `"$.user.name"` and `"$.action"` — syntax that *looks* like canonical JSONPath but isn't supported by the routing crate's dotted-path mini-syntax (`apimock_routing::util::json::json_value_by_jsonpath`). The tests still passed because they only verified round-trip preservation, never calling `is_match`. 5.7.0 rewrote the fixtures to use the correct dotted form (`"user.name"`, `"action"`), strengthened the rustdoc on `apimock_routing::util::json`, `apimock_routing::rule_set::rule::when::request::body::Body`, and `apimock_config::toml_writer::request_table` with explicit "not canonical JSONPath / RFC 9535" warnings, expanded the `apimock` example TOML's `body.json` block with realistic dotted-path examples, and added a JSONPath-mismatch note to `docs/src/advanced-topics/rule-set-config-structure/rules/when.md`.
