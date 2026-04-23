# Configuration reference

Everything below describes the TOML config files. Run `npx apimock --init` (or
`apimock --init`) once to scaffold `apimock.toml` and `apimock-rule-set.toml`
into the current directory, then tweak to taste.

## `apimock.toml` — top-level file

```toml
[listener]
ip_address = "127.0.0.1"   # interface to bind
port       = 3001          # plaintext HTTP port

[listener.tls]             # optional — enables HTTPS
cert = "./cert.pem"
key  = "./key.pem"
# port = 3002              # optional — if omitted, HTTPS replaces HTTP on the port above

[log]
verbose = { header = true, body = true }

[service]
strategy             = "first_match"                # only value supported today
rule_sets            = ["apimock-rule-set.toml"]    # paths, resolved relative to this file
middlewares          = []                           # Rhai scripts, resolved relative to this file
fallback_respond_dir = "."                          # folder served when no rule matches
```

| Section / field | Required | Default | Notes |
| --- | --- | --- | --- |
| `listener.ip_address` | no | `127.0.0.1` | Bind address. `0.0.0.0` to accept from other machines. |
| `listener.port` | no | `3001` | Plaintext HTTP port. |
| `listener.tls.cert` / `.key` | yes if `[listener.tls]` set | — | PEM files. Any key format `rustls-pki-types` accepts (PKCS#8, PKCS#1, SEC1). |
| `listener.tls.port` | no | same as `listener.port` | When omitted, the single port serves HTTPS only — no plaintext listener is started. |
| `log.verbose.header` | no | `false` | Log each request's headers. |
| `log.verbose.body` | no | `false` | Log each request's URL query and JSON body (pretty-printed). |
| `service.strategy` | no | `first_match` | Currently the only strategy — the first rule that matches wins. |
| `service.rule_sets` | no | `[]` | See **Rule-set schema** below. Paths relative to this file. |
| `service.middlewares` | no | `[]` | Rhai script paths, relative to this file. |
| `service.fallback_respond_dir` | no | `.` | Folder served when nothing in `rule_sets` or `middlewares` handled the request. |

## Rule-set schema — `apimock-rule-set.toml`

```toml
[prefix]                                       # optional
url_path    = "/api/v1/"                       # prepended to every rule's url_path
respond_dir = "apimock-rule-sets/@respond-dir" # prepended to every rule's file_path

[default]                                      # optional
delay_response_milliseconds = 0                # applied to every rule in this set unless overridden

[[rules]]
when.request.url_path = "complicated"
when.request.method   = "POST"
when.request.headers.authorization = { value = "Bearer eyJhb", op = "starts_with" }
when.request.headers.user          = { value = "user1" }                 # op defaults to "equal"
when.request.body.json."user.id"   = { value = "42" }
respond = { text = "Strictly authorized !" }

[[rules]]
when.request.url_path = { value = "/public/", op = "starts_with" }
respond = { file_path = "public/index.json", csv_records_key = "records" }

[[rules]]
when.request.url_path = ""
respond = { status = 403 }
```

### `when.request` — match conditions (AND-ed together)

| Field | Accepts | Purpose |
| --- | --- | --- |
| `url_path` | string *or* `{ value, op }` | URL path match. String form is shorthand for `op = "equal"`. |
| `method` | `"GET"` / `"POST"` / `"PUT"` / `"DELETE"` | HTTP method. Omit to match any method. |
| `headers` | `{ header-name = { value, op } }` map | Every listed header must match. Header names are matched case-insensitively per HTTP. |
| `body.json` | `{ json-path = { value, op } }` map | JSON body match — see **JSONPath form** below. |

### `respond` — response shape

Exactly one of `file_path` / `text` / `status` must be present. (`text` may be combined with `status` to set the response code.)

| Field | Type | Purpose |
| --- | --- | --- |
| `file_path` | string | File under `prefix.respond_dir` (or the working dir if no prefix). Extension decides content type; `.json` / `.json5` / `.csv` served as JSON, other text types served verbatim with an inferred `Content-Type`, binary types (images, audio, video, `.pdf`, `.zip`) served with the corresponding binary `Content-Type`. |
| `text` | string | Return this string verbatim. Content type `text/plain; charset=utf-8`. |
| `status` | integer (100–999) | HTTP status code. With `text`, sets the status; alone, returns an empty body with that status. Cannot be combined with `file_path`. |
| `headers` | `{ name = value }` map | Extra response headers. |
| `delay_response_milliseconds` | integer | Sleep this long before responding — useful for simulating slow backends in UI tests. |
| `csv_records_key` | string | Only meaningful when `file_path` points at a `.csv`. The JSON response wraps the rows as `{ "<key>": [...] }` using this value; defaults to `"records"`. |

### `op` — comparison operators

Every `{ value, op }` structure accepts these `op` values. If `op` is omitted it defaults to `equal`.

| `op` | Meaning | Example |
| --- | --- | --- |
| `equal` (default) | Exact string match | `{ value = "user1" }` matches `user1` only |
| `not_equal` | Anything *but* an exact match | `{ value = "guest", op = "not_equal" }` matches everything except `guest` |
| `starts_with` | String starts with value | `{ value = "Bearer ", op = "starts_with" }` matches any Bearer token |
| `contains` | Value appears anywhere inside the target | `{ value = "admin", op = "contains" }` matches `superadmin`, `admin-1`, etc. |
| `wild_card` | Glob match — see below | `{ value = "/api/v*/users/?", op = "wild_card" }` |

#### Wildcard / glob syntax (`wild_card`)

The glob matcher supports exactly two metacharacters, matching the common case without the complexity of a full regex engine:

| Token | Matches |
| --- | --- |
| `?` | Exactly one character |
| `*` | Zero or more characters |

Examples:

| Pattern | Matches | Doesn't match |
| --- | --- | --- |
| `hello` | `hello` | `hell`, `hello!` |
| `file*.txt` | `file.txt`, `file123.txt`, `files/doc.txt` | `file.csv` |
| `file?.txt` | `file1.txt`, `fileX.txt` | `file.txt`, `file12.txt` |
| `a*b?c` | `axyzbxc`, `ab1c` | `abc` |
| `*test*` | `my_test_file`, `test` | `tes` |
| `こんにち?世界` | `こんにちは世界` | Unicode is respected per code point. |

This is NOT a regex. Sequences such as `.`, `[…]`, `(…)`, `+`, `^`, `$` are matched literally.

#### JSONPath form (`body.json` and `csv_records_key`)

The JSONPath support is deliberately minimal — only the "object key" and "array index" forms, joined by `.`. This covers every real-world body-match need without exposing a full jsonpath engine that users would then have to learn.

| Input | Meaning |
| --- | --- |
| `user` | Top-level key `user` |
| `user.id` | `user` → `id` (nested object) |
| `items.0` | First element of array `items` |
| `orders.2.total` | Array index 2 of `orders`, then key `total` |
| `a.b.c` | Three-level nested object lookup |

If any segment is missing or has the wrong shape (e.g. a numeric segment against an object), the rule simply does not match — no error, because "missing" is a useful matchable condition.

### `prefix`

| Field | Effect |
| --- | --- |
| `url_path` | Prepended to every rule's `when.request.url_path` before matching. Leading/trailing slashes are normalised. The `contains` operator deliberately ignores the prefix — it matches against the raw `url_path` value — because "contains" usually means "substring anywhere in the full request path", and re-applying a prefix there would be surprising. |
| `respond_dir` | Prepended to every rule's `respond.file_path`. Resolved relative to the rule-set file's directory, not the working dir — so configs travel cleanly between machines. |

### `default`

| Field | Effect |
| --- | --- |
| `delay_response_milliseconds` | Default delay for rules in this set that don't specify their own. |
