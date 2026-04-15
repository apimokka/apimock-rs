# `when` data

Each `[[rules]]` entry contains a `when` table (e.g., `when.request.url_path`, `when.request.headers`, `when.request.body.json`). This table defines the conditions that an incoming HTTP request must meet for this rule to be triggered.

## `when.request.url_path`

Matches the request's URL path.

```toml
# apimock-rule-set.toml
[[rules]]
when.request.url_path = "/greetings"
```

## `when.request.headers`

Matches specific HTTP headers. You can specify multiple header keys.

- **Note:** According to [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html), HTTP header field names are case-insensitive. Your mock server will handle this automatically.

```toml
# apimock-rule-set.toml
[[rules]]
when.request.headers.user = { value = "user1" }
```

## `when.request.body.json`

Matches content within the request body. Currently, this supports matching specific keys and values within **JSON request bodies**.

- **JSON Body Matching:** You define the target key using **dot-notation paths** (e.g., `request.body.json.order.items.0.product_id`). For array fields, use the 0-based index number (e.g., `.0`).

```toml
# apimock-rule-set.toml
[[rules]]
when.request.body.json.order.items.0.product_id = { value = "123" }
```

## Multiple conditions strategy

**Important:** If you define multiple conditions (e.g., a path, a header, and a body match) within a single `[[rules]]` block, they are all evaluated using **AND logic**. All conditions must be met for the rule to match the incoming request.
