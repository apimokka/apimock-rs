# Quickstart

What `apimock --init` scaffolds, and what every release archive ships
alongside the binary. Deliberately small - a starting point to keep,
not a feature tour. For a specific feature, see the sibling
directories under `crates/apimock/examples/`.

## Run it

```sh
cd crates/apimock/examples/config/default
apimock
```

(or, from a source checkout: `cargo run -p apimock -- --config crates/apimock/examples/config/default/apimock.toml`)

## Try it

```sh
$ curl http://127.0.0.1:3001/health
ok

$ curl http://127.0.0.1:3001/greet
Hello, world.

$ curl http://127.0.0.1:3001/hello
Hello from middleware!
```

`/health` and `/greet` come from `apimock-rule-set.toml`; `/hello` is
answered by `apimock-middleware.rhai` before the rule set is even
consulted - middleware runs first, and a returned value serves the
response directly.
