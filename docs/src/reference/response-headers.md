# Response headers

Every response — including a 404 — carries a fixed set of default
headers. Some vary by request; none of this is configurable globally.

## Always present

| Header | Value |
|---|---|
| `access-control-allow-headers` | `*` |
| `access-control-allow-methods` | `GET, POST, PUT, DELETE, OPTIONS` |
| `access-control-max-age` | `86400` |
| `cache-control` | `no-store` |
| `x-content-type-options` | `nosniff` |

Source: `DEFAULT_RESPONSE_HEADERS` in
`crates/apimock-server/src/constant.rs`. A `date` header also appears
on every response, but nothing in `apimock-server` sets it explicitly
— it's added by the underlying HTTP transport layer, not application
code.

## `connection: keep-alive` — HTTP/1.1 only

`apimock-server` sets `connection: keep-alive` on every response it
builds, alongside the headers above — but unlike them, it isn't always
present on the wire. `Connection` is a hop-by-hop header defined for
HTTP/1.1's own connection-management model; HTTP/2 multiplexes many
requests over one connection and has no equivalent concept, so RFC 9113
§ 8.2.2 requires an intermediary to strip it, and hyper does so
correctly before this project's own `DEFAULT_RESPONSE_HEADERS` value
ever reaches the wire. This is a transport-layer removal, not something
`apimock-server`'s own code special-cases per protocol.

Verified against a running server, both protocols, same request:

```
$ curl -s -i --http1.1 http://127.0.0.1:3011/hello.json | grep -i connection
connection: keep-alive

$ curl -s -i -k --http2 https://127.0.0.1:3012/hello.json | grep -i connection
$ # (no output — the header is genuinely absent, not empty)
```

If you're asserting on this header in a test against apimock, either
force HTTP/1.1 or don't assert on it at all when HTTP/2 is in play.

## CORS — origin and credentials

`access-control-allow-origin`, `vary`, and (conditionally)
`access-control-allow-credentials` depend on whether the request looks
authenticated — defined as carrying a `cookie` or `authorization`
header — **and**, if so, whether the request's `origin` is allowed
credentialed reflection (RFC 067):

| Request | `access-control-allow-origin` | `vary` | `access-control-allow-credentials` |
|---|---|---|---|
| No `cookie`/`authorization` | `*` | `*` | *(absent)* |
| Credentialed, origin **allowed** | The request's own `origin` value, reflected back | `Origin` | `true` |
| Credentialed, origin **not allowed** | `*` | `*` | *(absent)* |

An origin is "allowed" if it's `http://localhost:*` or
`http://127.0.0.1:*` (implicitly, always — no configuration needed), or
appears exactly in `[service].cors_allow_credentials_origins` (empty by
default). An unlisted, non-loopback origin gets the same response as a
request with no credentials at all — the response is still served, but
without the headers a browser needs to expose it to a credentialed
cross-origin read. See
[the threat model](threat-model.md#deliberate-allowances-with-reasons)
for why.

Source: `default_response_headers` /
`is_likely_authenticated_request` / `is_credentialed_reflection_allowed`
in `crates/apimock-server/src/response_handler.rs`.

## `OPTIONS` requests

Handled before anything else in the request pipeline — before
middleware, before rule matching, before parsing the body. Every
`OPTIONS` request gets:

- Status **`204 No Content`** (not `200`).
- `content-length: 0`.
- The full default header set above, including the CORS headers.

Source: `handle_options` in `crates/apimock-server/src/server.rs`. See
[Matching order and precedence](../how-it-works/matching-order-and-precedence.md)
for where this sits in the overall request flow.

## Custom headers via `respond.headers`

`respond.headers` adds or overrides headers on a per-rule basis,
**uniformly** across every `respond` shape — `file_path` (JSON, JSON5,
CSV, binary, or plain text), `text`, `json`, and `status`-only, with or
without a custom status code. An explicit `content-type` in
`respond.headers` always overrides whatever content-type the response
would otherwise derive: from a file's extension, from `text`'s
`text/plain; charset=utf-8` default, or from `json`'s
`application/json` default.

This section used to carry a per-shape table of exceptions — several
`respond` shapes silently dropped custom headers entirely (RFC 045),
and every shape that *did* honour a custom `content-type` still had it
overwritten by the derived default immediately afterward (RFC 065).
Both are now fixed by routing every response-building call site
through one shared step (`ResponseHandler::with_custom_headers`,
applied only after the body — and its derived content-type — is
already set), so there's no longer a shape-by-shape exception to list:
if you set `respond.headers`, including `content-type`, it's honoured,
on every shape.
