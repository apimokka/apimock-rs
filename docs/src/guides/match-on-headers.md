# Match on headers

```toml
[[rules]]
when.request.url_path = "/orders"
[rules.when.request.headers]
x-api-key = { op = "absent" }
[rules.respond]
text = "missing x-api-key header"
status = 401

[[rules]]
when.request.url_path = "/orders"
[rules.when.request.headers]
x-api-key = { op = "exists" }
[rules.respond]
text = "order created"
status = 201
```

Header names match case-insensitively. Multiple headers in one
`[rules.when.request.headers]` table are ANDed, same as any other
combination of conditions in a rule.

`exists`/`absent` check only whether the header key is present —
`value` is ignored for both. Every other operator compares the
header's actual value: `equal`, `contains`, `starts_with`, `regex`, and
their negations — same set as `url_path`'s, listed in full in the
[Operator reference](../reference/operator-reference.md#headers--headeroperator-13).

A worked, verified example gating a whole endpoint on an API key and
falling through to more specific rules once authenticated:
[`crates/apimock/examples/match-headers-and-body/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/match-headers-and-body).
