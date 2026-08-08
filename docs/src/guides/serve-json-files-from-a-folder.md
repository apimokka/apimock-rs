# Serve JSON files from a folder

The headline feature: drop JSON files into a folder and they're
immediately reachable as an API — no rules, no rule set, nothing to
author.

```sh
mkdir -p api/v1/
echo '{"hello": "world"}' > api/v1/hello.json
apimock

curl http://localhost:3001/api/v1/hello
# --> {"hello":"world"}
```

The URL path maps directly to a file path under `service.fallback_respond_dir`
(`.` by default — the current directory). The extension is optional in
the request, and `.json`, `.json5`, and `.csv` are all recognised —
a `.csv` file is converted to a JSON array of objects, one per row,
keyed by column header.

A full worked example — collection and member endpoints, plus a CSV
file — is [`crates/apimock/examples/serve-json-resources/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/serve-json-resources),
runnable and automatically verified; its
[README](https://github.com/apimokka/apimock-rs/blob/main/crates/apimock/examples/serve-json-resources/README.md)
walks through it with real `curl` output.

This is the last stage in the request pipeline — see
[Matching order and precedence](../how-it-works/matching-order-and-precedence.md).
Once you need conditional responses (different output depending on
headers, method, or body content), move on to
[Match on URL path and method](./match-on-url-path-and-method.md).
