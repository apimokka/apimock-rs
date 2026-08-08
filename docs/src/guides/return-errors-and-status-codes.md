# Return errors and status codes

```toml
[[rules]]
when.request.url_path = "/widgets/999"
respond = { text = "widget not found", status = 404 }

[[rules]]
when.request.url_path = "/widgets/rate-limited"
respond = { text = "rate limit exceeded, retry after 30s", status = 429 }

[[rules]]
when.request.url_path = "/widgets/2"
when.request.method = "DELETE"
respond.status = 204
```

`respond.status` alone is an empty body with just that status code —
useful for `204`, or any response where the status is the whole
answer. `respond = { text = "...", status = N }` pairs a status with a
message body. Either way, `status` accepts any HTTP status code.

**Custom headers on a status response are unreliable** — see
[Response headers](../reference/response-headers.md#custom-headers-via-respondheaders--uneven-support)
before relying on `respond.headers` alongside `status`; in particular,
a `3xx` redirect's `Location` header currently can't be set this way.

A worked, verified example covering the common REST-error range (400,
401, 403, 404, 429, 500) plus a bare `204`:
[`crates/apimock/examples/status-codes-and-errors/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/status-codes-and-errors).
