# apimock-rs (API Mock)

[![npm](https://img.shields.io/npm/v/apimock-rs)](https://www.npmjs.com/package/apimock-rs)
[![crates.io](https://img.shields.io/crates/v/apimock?label=rust)](https://crates.io/crates/apimock)
[![License](https://img.shields.io/github/license/apimokka/apimock-rs)](https://github.com/apimokka/apimock-rs/blob/main/LICENSE)

[![Rust Documentation](https://docs.rs/apimock/badge.svg?version=latest)](https://docs.rs/apimock)
[![Dependency Status](https://deps.rs/crate/apimock/latest/status.svg)](https://deps.rs/crate/apimock)
[![Releases Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/release-executable.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/release-executable.yaml)
[![App Docs Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/docs.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/docs.yaml)

![logo](docs/src/assets/logo.png)

Build a working REST API in seconds — without a backend.    
Frontend blocked by an unfinished backend ?
Need stable API responses for UI tests or offline development ?    
Drop JSON files into a folder and your API immediately exists.

## Mock APIs easily 🎈 — just JSON and go

If you’re building or testing APIs, this tool makes mocking painless. It’s super fast, efficient, and flexible when you need it to be.
All you have to do to start up is just use folders and JSON without any config set.

- ❄️ Zero-config start.
- 🌬️ Fast to boot, light on memory.
- 🪄 File-based and rule-based matching. Scripting supported.

### Why `apimock-rs` ?

- The backend is not ready yet.
- You need stable API responses for UI testing.
- You want offline development.
- CI tests require a predictable API.
- Your mock data is becoming large.

### Handles real project scale

As your project grows, your mock API grows, too. Large mock datasets often cause problems:

- Slow startup
- High memory usage
- Crashes during UI testing
- Unstable CI runs

apimock-rs does not preload responses. Each response file is read only when a request arrives using non-blocking I/O. This keeps:

- Startup nearly instant
- Memory usage minimal
- Stable behavior under repeated requests

as validated with k6 load testing.
You can run UI development and automated tests continuously without worrying about server instability.

### 📖 Documentation

For more details, **🧭 check out [the docs](https://apimokka.github.io/apimock-rs/)**.

---

## Quick start

Easy to start with [npm package](https://www.npmjs.com/package/apimock-rs).

```sh
# install into your app project
npm install -D apimock-rs
# and go
npx apimock
```

```sh
# just use folders and JSON
mkdir -p api/v1/
echo '{"hello": "world"}' > api/v1/hello.json
npx apimock

# response
curl http://localhost:3001/api/v1/hello
# --> {"hello":"world"}
```

You may also check it out with browser to visit http://localhost:3001/api/v1/hello .

You now have a running REST endpoint.

### Vite project integration

An example of **scripts** section in **package.json** is as below.

**concurrently** is used to run the Vite and API mock servers simultaneously, while **cross-env** enables terminal output coloring. Before starting, ensure you run:

```sh
npm install -D concurrently cross-env
```

Edit package.json:

```json
  "scripts": {
    "apimock": "npx apimock",
    "dev": "cross-env CLICOLOR_FORCE=1 concurrently \"vite\" \"npm run apimock\""
  }
```

Run:

```sh
npm run dev
```

### `npx apimock` variation

| command | result |
| --- | --- |
| `npx apimock` | Run with all default parameters. |
| `npx apimock -p 4000` | Run with custom port. |
| `npx apimock -d tests/apimock-dyn-route` | Run with custom root dir on server response. |
| `npx apimock -c apimock.toml` | Run with config file giving rich features. Running `npx apimock --init` beforehand is required. |

---

## Configuration reference

Everything below describes the TOML config files. Run `npx apimock --init` (or
`apimock --init`) once to scaffold `apimock.toml` and `apimock-rule-set.toml`
into the current directory, then tweak to taste.

### `apimock.toml` — top-level file

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

### Rule-set schema — `apimock-rule-set.toml`

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

#### `when.request` — match conditions (AND-ed together)

| Field | Accepts | Purpose |
| --- | --- | --- |
| `url_path` | string *or* `{ value, op }` | URL path match. String form is shorthand for `op = "equal"`. |
| `method` | `"GET"` / `"POST"` / `"PUT"` / `"DELETE"` | HTTP method. Omit to match any method. |
| `headers` | `{ header-name = { value, op } }` map | Every listed header must match. Header names are matched case-insensitively per HTTP. |
| `body.json` | `{ json-path = { value, op } }` map | JSON body match — see **JSONPath form** below. |

#### `respond` — response shape

Exactly one of `file_path` / `text` / `status` must be present. (`text` may be combined with `status` to set the response code.)

| Field | Type | Purpose |
| --- | --- | --- |
| `file_path` | string | File under `prefix.respond_dir` (or the working dir if no prefix). Extension decides content type; `.json` / `.json5` / `.csv` served as JSON, other text types served verbatim with an inferred `Content-Type`, binary types (images, audio, video, `.pdf`, `.zip`) served with the corresponding binary `Content-Type`. |
| `text` | string | Return this string verbatim. Content type `text/plain; charset=utf-8`. |
| `status` | integer (100–999) | HTTP status code. With `text`, sets the status; alone, returns an empty body with that status. Cannot be combined with `file_path`. |
| `headers` | `{ name = value }` map | Extra response headers. |
| `delay_response_milliseconds` | integer | Sleep this long before responding — useful for simulating slow backends in UI tests. |
| `csv_records_key` | string | Only meaningful when `file_path` points at a `.csv`. The JSON response wraps the rows as `{ "<key>": [...] }` using this value; defaults to `"records"`. |

#### `op` — comparison operators

Every `{ value, op }` structure accepts these `op` values. If `op` is omitted it defaults to `equal`.

| `op` | Meaning | Example |
| --- | --- | --- |
| `equal` (default) | Exact string match | `{ value = "user1" }` matches `user1` only |
| `not_equal` | Anything *but* an exact match | `{ value = "guest", op = "not_equal" }` matches everything except `guest` |
| `starts_with` | String starts with value | `{ value = "Bearer ", op = "starts_with" }` matches any Bearer token |
| `contains` | Value appears anywhere inside the target | `{ value = "admin", op = "contains" }` matches `superadmin`, `admin-1`, etc. |
| `wild_card` | Glob match — see below | `{ value = "/api/v*/users/?", op = "wild_card" }` |

##### Wildcard / glob syntax (`wild_card`)

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

##### JSONPath form (`body.json` and `csv_records_key`)

The JSONPath support is deliberately minimal — only the "object key" and "array index" forms, joined by `.`. This covers every real-world body-match need without exposing a full jsonpath engine that users would then have to learn.

| Input | Meaning |
| --- | --- |
| `user` | Top-level key `user` |
| `user.id` | `user` → `id` (nested object) |
| `items.0` | First element of array `items` |
| `orders.2.total` | Array index 2 of `orders`, then key `total` |
| `a.b.c` | Three-level nested object lookup |

If any segment is missing or has the wrong shape (e.g. a numeric segment against an object), the rule simply does not match — no error, because "missing" is a useful matchable condition.

#### `prefix`

| Field | Effect |
| --- | --- |
| `url_path` | Prepended to every rule's `when.request.url_path` before matching. Leading/trailing slashes are normalised. The `contains` operator deliberately ignores the prefix — it matches against the raw `url_path` value — because "contains" usually means "substring anywhere in the full request path", and re-applying a prefix there would be surprising. |
| `respond_dir` | Prepended to every rule's `respond.file_path`. Resolved relative to the rule-set file's directory, not the working dir — so configs travel cleanly between machines. |

#### `default`

| Field | Effect |
| --- | --- |
| `delay_response_milliseconds` | Default delay for rules in this set that don't specify their own. |

---

## Open-source, with care

This project is lovingly built and maintained by volunteers.  
We hope it helps streamline your API development.  
Please understand that the project has its own direction — while we welcome feedback, it might not fit every edge case 🌱

## Acknowledgements

Depends on [tokio](https://github.com/tokio-rs/tokio) / [hyper](https://hyper.rs/) / [toml](https://github.com/toml-rs/toml) / [serde](https://serde.rs/) / [serde_json](https://github.com/serde-rs/json) / [json5](https://github.com/callum-oakley/json5-rs) / [console](https://github.com/console-rs/console) / [rhai](https://github.com/rhaiscript/rhai) / [thiserror](https://crates.io/crates/thiserror) / [anyhow](https://crates.io/crates/anyhow). In addition, [mdbook](https://github.com/rust-lang/mdBook) (as to workflows).
