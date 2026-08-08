# Status codes and error responses

A small "widgets" API demonstrating `respond.status`, alone and paired
with a message body, across the status codes a mock API is most often
asked to simulate: validation failure, missing auth, forbidden,
not-found, rate limiting, and a server error - plus a bare no-content
response.

`[prefix] url_path = "/widgets"` strips that segment before rules are
matched, so each rule's `when.request.url_path` only needs to name what
comes after it.

## Run it

```sh
cd crates/apimock/examples/status-codes-and-errors
apimock
```

## Try it

```sh
$ curl -i -X POST http://127.0.0.1:3001/widgets/create
HTTP/1.1 400 Bad Request
missing field: name

$ curl -i http://127.0.0.1:3001/widgets/private
HTTP/1.1 401 Unauthorized
authentication required

$ curl -i -X DELETE http://127.0.0.1:3001/widgets/1
HTTP/1.1 403 Forbidden
insufficient permissions

$ curl -i http://127.0.0.1:3001/widgets/999
HTTP/1.1 404 Not Found
widget not found

$ curl -i http://127.0.0.1:3001/widgets/rate-limited
HTTP/1.1 429 Too Many Requests
rate limit exceeded, retry after 30s

$ curl -i http://127.0.0.1:3001/widgets/boom
HTTP/1.1 500 Internal Server Error
internal error, try again later

$ curl -i -X DELETE http://127.0.0.1:3001/widgets/2
HTTP/1.1 204 No Content
```

The last one uses `respond.status` with no `text` at all - an empty
body, just the status line.
