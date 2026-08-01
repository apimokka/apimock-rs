# RFC 021 — Negated value operators for url_path, headers, and body

**Status.** Implemented (v5.12.0)
**Tracks.** Operator surface completeness — adding negated forms of
every string-style value operator across all three matching surfaces.
**Touches.** `apimock-routing` (`RuleOp`, `HeaderOperator`,
`BodyOperator`, match logic, tests), `apimock-config` (`UrlPathOp`,
`HeaderOp`, `BodyOp`, payload converters), documentation.

## Summary

Every string-style value operator currently has a positive form only
(`contains`, `starts_with`, `ends_with`, `regex`). Their negated
counterparts — "does NOT contain", "does NOT start with", etc. — are
useful for carve-out rules. This RFC adds `not_contains`,
`not_starts_with`, `not_ends_with`, and `not_regex` to `RuleOp`,
`HeaderOperator`, and `BodyOperator` simultaneously, keeping all
three surfaces symmetric.

## Motivation

- "Match any path that does NOT start with `/internal/`" — today
  requires careful rule ordering; with `not_starts_with`: one rule.
- "Reject requests whose `Content-Type` does NOT contain `json`" —
  today not expressible without Rhai middleware.
- "Match body `action` field that does NOT start with `admin_`" —
  today not expressible.

## Reference-level explanation

### `RuleOp` (routing crate)

```rust
// new variants:
NotContains, NotStartsWith, NotEndsWith, NotRegex,
```

`is_match`:
- `NotContains`   → `!text.contains(checker)`
- `NotStartsWith` → `!text.starts_with(checker)`
- `NotEndsWith`   → `!text.ends_with(checker)`
- `NotRegex`      → `!Regex::new(checker).map(|r| r.is_match(text)).unwrap_or(false)`

`NotRegex` with an invalid pattern returns `false` (non-matching),
identical to `Regex`'s invalid-pattern behaviour.

### `HeaderOperator` and `BodyOperator` (routing crate)

Same four variants added to each. `HeaderOperator::to_rule_op` maps
them to the corresponding `RuleOp` variants.

### Payload enums (config crate)

`UrlPathOp`, `HeaderOp`, and `BodyOp` each gain the four variants.
Payload-to-routing converters updated exhaustively.

### Missing-key semantics for headers

If a header key is absent and the operator is `NotContains`, the rule
does NOT match. This is symmetric with `Contains` on a missing key —
both require the key to be present before comparing values.

## Drawbacks

Operator enum sizes grow (RuleOp 7→11, HeaderOperator 9→13,
BodyOperator 18→22). Each new variant is a simple negation of an
existing one, so the maintenance cost is proportional but not complex.

## Unresolved questions

None — all design decisions follow RFC 017's established patterns.
