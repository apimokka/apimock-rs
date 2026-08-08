# Rule-set schema

A rule-set file — one of the paths listed in `service.rule_sets` — has
five possible top-level tables/keys, only one of which (`[[rules]]`) is
required.

```toml
strategy = "round_robin"          # optional: overrides service.strategy, this file only

[prefix]
url_path = "/api/v2"
respond_dir = "responses"

[default]
delay_response_milliseconds = 1000   # currently has no effect — see below

[guard]                              # currently has no effect at all — see below

[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
when.request.headers.x-api-key = { op = "exists" }
when.request.body.json."customer.tier" = { op = "equal", value = "gold" }
respond = { file_path = "vip-order.json" }
priority = 10
weight = 3
```

## `strategy` (top-level, optional)

A bare string for a unit strategy (`"first_match"`, `"round_robin"`,
`"uniform_random"`, `"weighted_random"`), or a table for `priority`
(which always needs its own table, even to accept default settings —
`priority = "..."` is a parse error). Overrides `service.strategy` for
this rule set only. See
[Vary the response for one path](../guides/vary-the-response-for-one-path.md)
for the full syntax of all five.

## `[prefix]`

| Field | Meaning |
|---|---|
| `url_path` | Stripped from the front of the request path before this rule set's rules are matched — a rule's own `when.request.url_path` only needs to name what comes after it |
| `respond_dir` | Prepended to every `respond.file_path` in this rule set |

## `[default]`

The only field is `delay_response_milliseconds`. **It currently has no
effect on any response** — it's parsed and printed in the startup log,
but nothing applies it. The per-rule
`respond.delay_response_milliseconds` (below) works correctly; this
rule-set-wide equivalent does not. See
[Simulate slow or flaky backends](../guides/simulate-slow-or-flaky-backends.md).

## `[guard]`

A zero-field table today — there is nothing to put inside it, and a
`[guard]` block with any content fails to parse. It carries a `// todo:`
comment in the source for a rule-set-wide condition that was never
implemented. Don't configure it expecting it to gate anything; nothing
reads it beyond printing an empty line in the startup log.

## `[[rules]]`

Each rule is `when` (what has to be true of the request) plus `respond`
(what to send back), plus two optional strategy-specific fields.

### `when.request`

At least one of the following is required; multiple conditions within
one rule are ANDed.

| Field | Shape |
|---|---|
| `url_path` | A bare string (implies `op = "equal"`), or `{ value = "...", op = "..." }` |
| `method` | A bare HTTP method string: `"GET"`, `"POST"`, `"PUT"`, or `"DELETE"` |
| `headers.<name>` | `{ value = "...", op = "..." }` per header, ANDed; header names match case-insensitively |
| `body.json."<dotted.path>"` | `{ value = "...", op = "..." }` per path, ANDed — see [Body path syntax](./body-path-syntax.md) |

Every operator for `url_path`/`headers`/`body.json` is listed in the
[Operator reference](./operator-reference.md).

### `respond`

At least one of `file_path`, `text`, or `status` is required.

| Field | Meaning |
|---|---|
| `file_path` | Serve this file's content — extension decides JSON/JSON5/CSV/binary/text handling |
| `text` | A literal response body (plain text unless combined with `status`, see caveats) |
| `status` | The HTTP status code |
| `headers` | Custom headers — support is uneven; see [Response headers](./response-headers.md) for exactly which combinations honour it |
| `delay_response_milliseconds` | Sleep this long before responding — works correctly at the per-rule level |
| `csv_records_key` | For a CSV `file_path`, the dotted path under which the parsed rows are nested in the JSON response (default key: `records`) |

Validity rules: `file_path` and `text` are mutually exclusive.
`file_path` combined with `status` is rejected — status override is
only available with `text`. `text` combined with `status` is allowed
(a custom-status message body). `file_path` must resolve to a file
that exists under the rule set's `respond_dir`/`prefix.respond_dir` —
checked at startup, not per-request.

### `priority` and `weight`

Per-rule fields, read only when the governing strategy needs them:
`priority` (integer, for the `priority` strategy) and `weight`
(unsigned integer, default `1`, for `weighted_random`). Both are
ignored under every other strategy.
