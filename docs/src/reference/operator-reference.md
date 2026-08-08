# Operator reference

Every operator apimock-rs supports for matching, across the three
places conditions appear: `when.request.url_path`, `when.request.headers`,
and `when.request.body.json`. 49 variants total, generated from the
enum source directly rather than transcribed — see
[How this table is generated](#how-this-table-is-generated) for the
method, so you can re-run it yourself against any future version.

## `url_path` — `RuleOp` (11)

`when.request.url_path = { value = "...", op = "..." }`. The default
when `op` is omitted is `equal`.

| `op` | Matches when |
|---|---|
| `equal` | The path equals `value` exactly (default) |
| `not_equal` | The path does not equal `value` |
| `starts_with` | The path starts with `value` |
| `not_starts_with` | The path does not start with `value` |
| `ends_with` | The path ends with `value` |
| `not_ends_with` | The path does not end with `value` |
| `contains` | The path contains `value` as a substring |
| `not_contains` | The path does not contain `value` |
| `wild_card` | The path matches `value` as a glob pattern (`*`, `?`) |
| `regex` | The path matches `value` as a regular expression, compiled per request |
| `not_regex` | The path does not match `value` as a regular expression |

Source: `crates/apimock-routing/src/rule_set/rule/when/request/rule_op.rs`.

## `headers` — `HeaderOperator` (13)

`when.request.headers.<name> = { value = "...", op = "..." }`. The 11
value operators above, plus two presence-only operators where `value`
is ignored:

| `op` | Matches when |
|---|---|
| `equal` | The header's value equals `value` exactly (default) |
| `not_equal` | The header's value does not equal `value` |
| `starts_with` | The header's value starts with `value` |
| `not_starts_with` | The header's value does not start with `value` |
| `ends_with` | The header's value ends with `value` |
| `not_ends_with` | The header's value does not end with `value` |
| `contains` | The header's value contains `value` as a substring |
| `not_contains` | The header's value does not contain `value` |
| `wild_card` | The header's value matches `value` as a glob pattern |
| `regex` | The header's value matches `value` as a regular expression |
| `not_regex` | The header's value does not match `value` as a regular expression |
| `exists` | The header key is present, regardless of value (`value` ignored) |
| `absent` | The header key is not present (`value` ignored) |

Header names are matched case-insensitively (HTTP semantics). Source:
`crates/apimock-routing/src/rule_set/rule/when/request/headers/header_operator.rs`.

## `body.json` — `BodyOperator` (25)

`when.request.body.json."<dotted.path>" = { value = "...", op = "..." }`
— see [Body path syntax](./body-path-syntax.md) for how the path
resolves. The default `op` is `equal`.

| `op` | Matches when |
|---|---|
| `equal` | String-coerced equality (both sides converted to string) — the default, kept for backwards compatibility |
| `equal_string` | Explicit alias for `equal` |
| `contains` | The (string-coerced) value contains `value` as a substring |
| `not_contains` | The (string-coerced) value does not contain `value` |
| `starts_with` | The (string-coerced) value starts with `value` |
| `not_starts_with` | The (string-coerced) value does not start with `value` |
| `ends_with` | The (string-coerced) value ends with `value` |
| `not_ends_with` | The (string-coerced) value does not end with `value` |
| `regex` | The (string-coerced) value matches `value` as a regular expression |
| `not_regex` | The (string-coerced) value does not match `value` as a regular expression |
| `equal_typed` | Exact JSON-type-and-value equality — distinguishes `42` (number) from `"42"` (string); `value` is parsed as JSON |
| `equal_number` | Numeric equality; both sides coerced to `f64` |
| `greater_than` | Numeric greater-than |
| `less_than` | Numeric less-than |
| `greater_or_equal` | Numeric greater-than-or-equal |
| `less_or_equal` | Numeric less-than-or-equal |
| `exists` | The path resolves to any value, including `null` (`value` ignored) |
| `absent` | The path does not resolve to anything (`value` ignored) |
| `array_length_equal` | The value at the path is an array whose length equals `value` |
| `array_length_at_least` | The value at the path is an array whose length is ≥ `value` |
| `array_contains` | The value at the path is an array containing an element equal to `value` (typed JSON comparison) |
| `equal_integer` | Exact `i64` integer equality — avoids the precision loss `equal_number`'s `f64` coercion has above 2^53 |
| `map_has_key` | The value at the path is a JSON object containing the key named by `value` |
| `map_does_not_have_key` | The value at the path is a JSON object that does not contain the key named by `value` |
| `structural_contains` | The value at the path is an array containing at least one element that is a *superset* of the JSON object in `value` — every key in `value` present with an equal value; extra keys on the element are fine |

Source: `crates/apimock-routing/src/rule_set/rule/when/request/body/body_operator.rs`.

## How this table is generated

Every variant above is pulled directly from each enum's source, not
hand-copied — re-run this yourself against any checkout to confirm the
table above is current:

```sh
for f in \
  crates/apimock-routing/src/rule_set/rule/when/request/rule_op.rs:RuleOp \
  crates/apimock-routing/src/rule_set/rule/when/request/headers/header_operator.rs:HeaderOperator \
  crates/apimock-routing/src/rule_set/rule/when/request/body/body_operator.rs:BodyOperator
do
  file="${f%%:*}"; enum="${f##*:}"
  echo "=== $enum ==="
  awk -v enum="$enum" '
    $0 ~ "pub enum " enum " *\\{" { in_enum=1; next }
    in_enum && /^}/ { in_enum=0 }
    in_enum {
      line = $0; gsub(/^[[:space:]]+/, "", line)
      if (line ~ /^[A-Z][A-Za-z0-9]*,?$/) { gsub(/,$/, "", line); print line }
    }
  ' "$file" | sed -E 's/([a-z0-9])([A-Z])/\1_\2/g' | tr '[:upper:]' '[:lower:]'
done
```

Counts: **11** `RuleOp`, **13** `HeaderOperator`, **25** `BodyOperator`
— 49 total, matching every table above exactly.
