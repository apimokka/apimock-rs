# Roadmap

This file records design questions that have been identified during
development but intentionally postponed to a later release. Items
here are *not* bugs — they're follow-on work whose right shape is
easier to decide after some related primary feature has shipped.
Recording the rationale here prevents the original context from being
lost between releases.

## Deferred items

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
