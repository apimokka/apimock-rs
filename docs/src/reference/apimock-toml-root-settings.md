# `apimock.toml` root settings

The root config file's four top-level tables. All are optional — an
empty or missing `apimock.toml` is valid, and falls back to
zero-config, port-`3001`, serve-`./`-by-path behaviour.

```toml
[listener]
ip_address = "127.0.0.1"
port = 3001

[listener.tls]
cert = "./cert.pem"
key = "./key.pem"
# port = 3002   # omit to serve HTTPS-only on `listener.port`
# handshake_timeout_seconds = 10
# max_connections = 256

[log]
verbose = { header = true, body = true }

[service]
strategy = "first_match"
rule_sets = ["apimock-rule-set.toml"]
middlewares = ["apimock-middleware.rhai"]
fallback_respond_dir = "."
# cors_allow_credentials_origins = ["https://app.example.com"]
# max_request_body_bytes = 33554432
# middleware_max_operations = 10000000

[file_tree_view]
show_hidden = false
builtin_excludes = true
extra_excludes = ["*.bak"]
include = []
respect_gitignore = false
```

## `[listener]`

| Field | Type | Default | Meaning |
|---|---|---|---|
| `ip_address` | string | `"127.0.0.1"` | Bind address |
| `port` | integer | `3001` | Bind port |

`ip_address` accepts any address your OS can bind to, IPv4 or IPv6:

| `ip_address` | Binds to |
|---|---|
| `127.0.0.1` / `::1` | Loopback only (the default) |
| A LAN address, e.g. `192.168.1.10` | That interface |
| `0.0.0.0` / `::` | Every interface — reachable from outside the machine |

Binding to `0.0.0.0`/`::` or a LAN address exposes the mock server
beyond localhost — fine on a trusted network, a real exposure on
anything else.

## `[listener.tls]`

Enables HTTPS. Both `cert` and `key` must point at files that exist —
checked at startup, not lazily.

| Field | Type | Default | Meaning |
|---|---|---|---|
| `cert` | string | — | Path to the certificate PEM file |
| `key` | string | — | Path to the private key PEM file |
| `port` | integer, optional | — | If set, HTTPS listens here and plain HTTP continues on `listener.port`. If omitted, `listener.port` itself becomes HTTPS-only — no plaintext HTTP listener starts at all |
| `handshake_timeout_seconds` | integer | `10` | An incomplete TLS handshake is dropped after this long |
| `max_connections` | integer | `256` | Maximum concurrent HTTPS connections. Beyond this, a new connection waits for a slot rather than being refused — the server recovers as soon as one closes |

**Relative `cert`/`key` paths resolve against the process's current
directory, not against `apimock.toml`'s own location** — unlike
`rule_sets` and `fallback_respond_dir` below, which do resolve
relative to the config file. Run `apimock` from the directory
containing the cert/key files, or use absolute paths. See
[Serve over HTTPS](../guides/serve-over-https.md) for a full working
example.

**A cert/key that exists but fails to parse stops startup — the server
never binds any listener, HTTP included.** Before this was fixed, a
malformed PEM silently fell back to HTTP-only, which is worse than a
loud failure: an operator who configured HTTPS would not otherwise know
they didn't get it.

## `[log]`

| Field | Type | Default | Meaning |
|---|---|---|---|
| `verbose.header` | bool | `false` | Log request headers. Credential-bearing headers (`authorization`, `cookie`, `set-cookie`, `proxy-authorization`, `x-api-key`) print as `[redacted]` — same policy, same defaults, as the trace channel (RFC 040, RFC 051) |
| `verbose.body` | bool | `false` | Log request bodies. **Not redacted** — a JSON body's field values print verbatim, including any credentials it happens to carry. Name-based redaction doesn't transfer to body content (there are no header names to match against), and value-scanning bodies for secrets is out of scope for the same reason it is for the trace channel |

## `[service]`

| Field | Type | Default | Meaning |
|---|---|---|---|
| `strategy` | string or table | `"first_match"` | Default response strategy — see [Vary the response for one path](../guides/vary-the-response-for-one-path.md) for all five and their syntax |
| `rule_sets` | array of strings, optional | — | Rule-set files, checked in this order — see [Rule-set schema](./rule-set-schema.md) |
| `middlewares` | array of strings, optional | — | Rhai middleware files, checked in this order before any rule set — see [Script with Rhai middleware](../guides/script-with-rhai-middleware.md) |
| `fallback_respond_dir` | string | `"."` | Directory served by URL path when nothing above matches |
| `cors_allow_credentials_origins` | array of strings, optional | `[]` | Exact origins (beyond the always-allowed `http://localhost:*` / `http://127.0.0.1:*`) allowed credentialed CORS reflection — see [Response headers](./response-headers.md#cors--origin-and-credentials) |
| `max_request_body_bytes` | integer | `33554432` (32 MiB) | A request body over this size is refused with `413`, before it is buffered |
| `middleware_max_operations` | integer | `10000000` | Rhai operations one middleware evaluation may perform before it's aborted — see [Script with Rhai middleware](../guides/script-with-rhai-middleware.md) |

Relative `rule_sets` and `fallback_respond_dir` paths resolve against
`apimock.toml`'s own directory, regardless of the process's current
directory when `apimock` was started.

## `[file_tree_view]`

**This does not affect what the running server serves over HTTP.** It
filters the file tree shown by the `Workspace` config-editing API
(consumed by GUI tooling, and incidentally by `apimock validate`'s
internal rule/rule-set count) — not the `fallback_respond_dir` request
path. A file inside `node_modules`, `.git`, or any excluded pattern
here is still served over HTTP if a client requests its exact path.

| Field | Type | Default | Meaning |
|---|---|---|---|
| `show_hidden` | bool | `false` | Show dotfiles and dot-directories in the editor's file-tree view |
| `builtin_excludes` | bool | `true` | Apply the built-in exclude list (below) |
| `extra_excludes` | array of glob strings | `[]` | Additional excludes, matched against each entry's bare filename (not its full path) |
| `include` | array of glob strings | `[]` | An allow-list — only applies to files, never to directories |
| `respect_gitignore` | bool | `false` | Also exclude anything a `.gitignore` (found by walking up from the listed directory, stopping at the first `.git`) would ignore |

**Built-in excludes** (when `builtin_excludes = true`, matched by exact
bare name): `target`, `node_modules`, `dist`, `build`, `out`,
`__pycache__`, `.venv`, `vendor`, `.cargo`, `.gradle`, `.idea`,
`.vscode`. Note `.git` itself is *not* in this list — it's hidden by
the separate dotfile filter when `show_hidden = false`, but would
reappear in the editor's view if `show_hidden = true` and `.git` isn't
added to `extra_excludes` explicitly.

`extra_excludes` and `include` both use standard glob syntax (`*`,
`?`, `[…]`) via the `globset` crate. Filter order, each one able to
reject an entry outright: dotfile filter → built-in excludes →
`extra_excludes` → `.gitignore` → `include` (files only).
