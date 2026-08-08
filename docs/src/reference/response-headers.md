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

## Custom headers via `respond.headers` — uneven support

`respond.headers` is meant to add or override headers on a per-rule
basis. Whether it actually applies depends on which other `respond`
fields are set, and the exact behaviour differs by case:

| `respond` shape | Custom headers applied? |
|---|---|
| `file_path` → JSON, JSON5, or CSV | Yes, except a custom `content-type` is still overwritten by the default afterward |
| `file_path` → binary (non-UTF-8) | Yes, same `content-type` caveat |
| `file_path` → plain text (`.txt`, `.html`, anything else UTF-8) | **No — every custom header is dropped, not only `content-type`** |
| `text` alone (no `status`) | Yes, except a custom `content-type` is overwritten by the default afterward |
| `text` + `status` | **No — every custom header is dropped** |
| `status` alone | **No — every custom header is dropped** |

In short: a non-`content-type` custom header only reliably survives on
a `file_path` response to a JSON/JSON5/CSV/binary file, or on a
`text`-only response. A custom `content-type` override doesn't survive
anywhere. This is a product defect, not documented behaviour to design
around — if you need a specific `content-type` today, its file
extension deciding the served type (via `file_path`) is the only path
that reliably sets it.
