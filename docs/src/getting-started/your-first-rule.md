# Your first rule

`--init` scaffolds `apimock-rule-set.toml`, referenced from
`apimock.toml`'s `service.rule_sets`. Each `[[rules]]` block is one
condition-plus-response pair.

## Match on the path alone

```toml
[[rules]]
when.request.url_path = ""
respond.text = "home"

[[rules]]
when.request.url_path = "home"
respond.file_path = "home.json"
```

```sh
curl http://localhost:3001/
# --> home
curl http://localhost:3001/home
# --> (content of home.json)
```

`when.request.url_path` as a bare string is an exact match.
`respond.text` returns a literal string; `respond.file_path` serves a
file, with its content type inferred from the extension.

## Match on method too

```toml
[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
respond = { text = "order created", status = 201 }

[[rules]]
when.request.method = "GET"
when.request.url_path = "/orders"
respond.text = "order list"
```

Multiple conditions in one rule are ANDed — both `method` and
`url_path` have to match. `when.request.method` is always a bare
string: `"GET"`, `"POST"`, `"PUT"`, or `"DELETE"`.

## Match on a header

```toml
[[rules]]
when.request.url_path = "/private"
[rules.when.request.headers]
authorization = { value = "Bearer eyJhb", op = "starts_with" }
respond.text = "authenticated"
```

`op` picks the comparison — `starts_with` here, `equal` if omitted.
Header names match case-insensitively.

## Match on the request body

```toml
[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
[rules.when.request.body.json]
"customer.tier" = { value = "gold" }
respond.text = "VIP order"
```

`"customer.tier"` is apimock's own dotted-path syntax for reaching into
a JSON body — **not** JSONPath. See
[Body path syntax](../reference/body-path-syntax.md) before writing
anything more complex than one flat key.

## What's next

This is enough to build most mock APIs. Once you outgrow it:
[Match on URL path and method](../guides/match-on-url-path-and-method.md),
[Match on headers](../guides/match-on-headers.md), and
[Match on the request body](../guides/match-on-the-request-body.md)
cover every operator available for each; the
[Guides](../guides/) index covers everything else, including
scripting, strategies, TLS, and CI validation.
