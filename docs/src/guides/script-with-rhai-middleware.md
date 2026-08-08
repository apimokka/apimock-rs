# Script with Rhai middleware

```toml
[service]
middlewares = ["apimock-middleware.rhai"]
```

Middleware scripts run before any rule set, in the order listed,
before matching order and precedence's second stage — see
[Matching order and precedence](../how-it-works/matching-order-and-precedence.md).
The first script that returns a value answers the request; if none do,
the request falls through to the rule sets.

```rhai
//! pre-defined variables are available:
//! - url_path: request url path
//! - body: request body json value, defined only when the request has one

if url_path == "/profile" {
    return "data/profile.json";              // shorthand for #{ "file_path": ... }
}
else if url_path == "/profile/json" {
    return #{ "json": "{\"plan\": \"pro\"}" };
}
else if url_path == "/profile/text" {
    return #{ "text": "plan: pro" };
}

if is_def_var("body") {
    if url_path == "/orders" && body.priority == "rush" {
        return #{ "text": "expedited: this order jumps the queue" };
    }
}

return;   // falls through to the rule sets
```

## Return shapes

| Script returns | Response |
|---|---|
| a plain string | serves that file path |
| `#{ "file_path": "..." }` | same, spelled out |
| `#{ "json": "..." }` | literal JSON response body |
| `#{ "text": "..." }` | literal plain-text response body |
| nothing (falls off the end) | declines — falls through to the rule sets |

A relative file path is resolved against the middleware script's own
directory, not the process's current directory.

`is_def_var("body")` is only true when the request actually carries a
JSON body — checking it before reading `body.*` avoids a script error
on a GET or a bodyless request.

A worked, verified example exercising all four return shapes plus
body-driven branching:
[`crates/apimock/examples/scripting-with-middleware/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/scripting-with-middleware).
