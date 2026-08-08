# Match on the request body

```toml
[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
[rules.when.request.body.json]
"customer.tier" = { op = "equal", value = "gold" }
[rules.respond]
text = "VIP customer order"

[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
[rules.when.request.body.json]
"items.0.sku" = { op = "contains", value = "WIDGET" }
[rules.respond]
text = "widget order"
```

`when.request.body.json` keys are **dotted paths, not JSONPath** — see
[Body path syntax](../reference/body-path-syntax.md) for the exact
resolution rules and why `"$.a.b"`-style paths don't work. `"customer.tier"`
walks into a nested object; `"items.0.sku"` indexes into an array with
a numeric segment.

The 25 `BodyOperator` variants cover far more than string equality —
numeric comparison (`greater_than`, `less_than`), presence
(`exists`/`absent`), array checks (`array_contains`,
`array_length_at_least`), typed equality, and structural matching
against a JSON object shape (`structural_contains`). Full list in the
[Operator reference](../reference/operator-reference.md#bodyjson--bodyoperator-25).

A worked, verified example layering nested-object, array-index, and
numeric-comparison conditions by specificity:
[`crates/apimock/examples/match-headers-and-body/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/match-headers-and-body).
