# Body path syntax

`when.request.body.json` conditions and `respond.csv_records_key` both
address a value inside a JSON body using apimock's own dotted-path
mini-syntax. **This is not JSONPath** (RFC 9535) — see
[Design notes](../how-it-works/design-notes.md#why-dotted-paths-not-jsonpath)
for why.

## The rule

A path is a sequence of segments joined by `.`:

- A segment against a JSON object is a key lookup.
- A segment that parses as a non-negative integer, against a JSON
  array, indexes into that array.
- Anything that doesn't resolve — a missing key, an out-of-range
  index, indexing into a non-array — makes the path resolve to nothing,
  which is a non-match for every operator except `absent`.

Implementation: `json_value_by_jsonpath` in
`crates/apimock-routing/src/util/json.rs`, which walks the path with
`str::split('.')`, folding into the JSON value one segment at a time.

## Examples

Given this request body:

```json
{
  "customer": { "tier": "gold" },
  "items": [
    { "sku": "WIDGET-42", "qty": 3 },
    { "sku": "GADGET-7", "qty": 1 }
  ]
}
```

| Path | Resolves to |
|---|---|
| `"customer.tier"` | `"gold"` |
| `"items.0.sku"` | `"WIDGET-42"` — `0` indexes the first array element |
| `"items.1.qty"` | `1` |
| `"items.2.sku"` | nothing — index `2` is out of range |
| `"customer.email"` | nothing — key doesn't exist |

## What this is not

`"$.customer.tier"` does **not** work the way it would in JSONPath. The
leading `$` has no special meaning here — it's treated as a literal
object key, which almost never exists, so the condition silently never
matches. There's no `[0]` bracket-array syntax either; array indexing
is a plain numeric path segment, as in the table above.

This distinction matters enough to repeat: a condition written with
`$.`-prefixed pseudo-JSONPath doesn't error — it just never matches,
and a rule that never matches is easy to miss in testing. See
[Dry-run a rule](../guides/dry-run-a-rule.md) for a way to check a
condition actually matches before relying on it.
