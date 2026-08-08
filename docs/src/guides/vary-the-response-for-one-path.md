# Vary the response for one path

When more than one rule matches the same request, the rule set's
`strategy` decides which one answers. Five exist.

## `first_match` (the default)

No configuration needed — the first matching rule in file order wins,
every time. Deterministic.

## `priority`

```toml
[strategy]
priority = { tiebreaker = "first_match" }

[[rules]]
when.request.url_path = "/widgets"
respond.text = "general response"
priority = 1

[[rules]]
when.request.url_path = "/widgets"
respond.text = "special response (higher priority)"
priority = 10
```

Among matching rules, the highest `priority` wins — deterministically,
regardless of file order. `tiebreaker` (`first_match` or
`uniform_random`) decides what happens when two matching rules share
the top priority. Note `priority` always needs its own table, even for
a default tiebreaker — `strategy = "priority"` as a bare string is a
parse error, unlike the other four.

## `weighted_random`

```toml
[strategy]
weighted_random = { seed = 7 }   # omit `seed` for real randomness

[[rules]]
when.request.url_path = "/weighted"
respond.text = "variant-a"
weight = 3

[[rules]]
when.request.url_path = "/weighted"
respond.text = "variant-b"
weight = 1
```

Random among matches, weighted by `weight` (default `1` if omitted) —
`variant-a` above is picked roughly 3 times as often as `variant-b`.

**`seed`, if set, makes the pick fully deterministic** — not a fixed
*sequence*, but the *same* result every single request, since a fresh
RNG is seeded per call. That's useful for a reproducible test; it is
not a way to preview a realistic distribution. Omit `seed` entirely to
see genuine variation across requests.

## `uniform_random`

Same shape as `weighted_random` (an optional `seed`), but every match
has equal probability — `weight` is not consulted.

## `round_robin`

```toml
strategy = "round_robin"

[[rules]]
when.request.url_path = "/round-robin"
respond.text = "server-a"

[[rules]]
when.request.url_path = "/round-robin"
respond.text = "server-b"
```

Cycles through matches in file order, one per request:
`server-a`, `server-b`, `server-a`, `server-b`, ... Deterministic, and
doesn't need `weight` or `priority` set on any rule.

## Where `strategy` goes

`service.strategy` sets the default for every rule set. A rule set's
own top-level `strategy` field overrides that default for itself only
— which is how you can run several strategies side by side, each
scoped to its own rule set (and, typically, its own `[prefix]`). See
[Rule-set schema](../reference/rule-set-schema.md#strategy-top-level-optional).

A worked, verified example running all three of `priority`,
`weighted_random`, and `round_robin` from one server, each in its own
rule set:
[`crates/apimock/examples/vary-response-by-strategy/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/vary-response-by-strategy).
