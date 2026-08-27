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
| `connection` | `keep-alive` |
| `x-content-type-options` | `nosniff` |

Source: `DEFAULT_RESPONSE_HEADERS` in
`crates/apimock-server/src/constant.rs`. A `date` header also appears
on every response, but nothing in `apimock-server` sets it explicitly
— it's added by the underlying HTTP transport layer, not application
code.

## CORS — origin and credentials

`access-control-allow-origin`, `vary`, and (conditionally)
`access-control-allow-credentials` depend on whether the request looks
authenticated — defined as carrying a `cookie` or `authorization`
header:

| Request has `cookie`/`authorization`? | `access-control-allow-origin` | `vary` | `access-control-allow-credentials` |
|---|---|---|---|
| Yes | The request's own `origin` value, reflected back | `Origin` | `true` |
| No | `*` | `*` | *(absent)* |

Source: `default_response_headers` /
`is_likely_authenticated_request` in
`crates/apimock-server/src/response_handler.rs`.

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
