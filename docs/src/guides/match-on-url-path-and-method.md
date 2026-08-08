# Match on URL path and method

Once file-based serving isn't enough — you need different responses
for the same-looking request, or you want to react to the HTTP method —
add a rule set.

```toml
[service]
rule_sets = ["apimock-rule-set.toml"]
```

```toml
[[rules]]
when.request.url_path = "/health"
respond.text = "ok"

[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
respond = { text = "order created", status = 201 }
```

`when.request.url_path` on its own is a bare string, equivalent to
`{ value = "...", op = "equal" }`. Any of the eleven `RuleOp`
operators can apply here too — `starts_with`, `contains`, `wild_card`,
`regex`, and their negations — see the full list in the
[Operator reference](../reference/operator-reference.md#url_path--ruleop-11).

`when.request.method` only ever needs a bare string: `"GET"`,
`"POST"`, `"PUT"`, or `"DELETE"`. Combining `method` with `url_path` in
one rule ANDs them — both have to match.

Multiple rules matching the same request are resolved by the rule
set's [strategy](./vary-the-response-for-one-path.md) — by default,
the first one listed that matches. Worked, verified examples covering
status codes by path: [`crates/apimock/examples/status-codes-and-errors/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/status-codes-and-errors).
