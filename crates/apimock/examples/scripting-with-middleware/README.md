# Script a response with middleware

Middleware (Rhai) runs before rule sets, once per request. A returned
value serves the response directly; returning nothing falls through to
`apimock-rule-set.toml`. This set demonstrates all three response
shapes a middleware script can return, plus inspecting the request
body to decide whether to handle a request itself or defer.

## Run it

```sh
cd crates/apimock/examples/scripting-with-middleware
apimock
```

## Try it

Three ways to answer the same question, each returning `data/profile.json`'s
content one way or another:

```sh
$ curl http://127.0.0.1:3001/profile/file-path
{"plan":"pro","source":"middleware-file"}

$ curl http://127.0.0.1:3001/profile/json
{"plan":"pro","source":"middleware-json"}

$ curl http://127.0.0.1:3001/profile/text
plan: pro (middleware-text)

$ curl http://127.0.0.1:3001/profile
{"plan":"pro","source":"middleware-file"}
```

`/profile` uses the shorthand form - a bare returned string is the same
as `#{ "file_path": ... }`.

Body inspection: the middleware only handles `/orders` itself when the
body says `priority: "rush"`; anything else falls through to the rule
set below.

```sh
$ curl -X POST http://127.0.0.1:3001/orders \
    -H 'Content-Type: application/json' -d '{"priority":"rush"}'
expedited: this order jumps the queue

$ curl -X POST http://127.0.0.1:3001/orders \
    -H 'Content-Type: application/json' -d '{"priority":"normal"}'
standard: order queued normally

$ curl http://127.0.0.1:3001/orders
standard: order queued normally
```

The last two both reach `apimock-rule-set.toml` - one because the body
doesn't ask for rush handling, the other because there's no body at
all (`is_def_var("body")` is only true when the request actually
carries one).

## Return shapes, summarised

| Script returns | Response |
|---|---|
| a plain string | serves that file path |
| `#{ "file_path": "..." }` | same, spelled out |
| `#{ "json": "..." }` | literal JSON response body |
| `#{ "text": "..." }` | literal plain-text response body |
| nothing (falls off the end) | declines - falls through to `rule_sets` |
