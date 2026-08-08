# Vary the response for one path

Three ways to pick among several rules that match the same request,
each in its own rule set and its own URL prefix so all three can run
from one server. A rule-set file's own top-level `strategy` overrides
`service.strategy` for that rule set only (RFC 025) - `service.strategy`
itself is left unset here, so each rule set's override is what governs
it.

## Run it

```sh
cd crates/apimock/examples/vary-response-by-strategy
apimock
```

## Try it

**`priority.toml`** - the highest `priority` among matching rules wins,
every time:

```sh
$ curl http://127.0.0.1:3001/priority
special response (higher priority)
$ curl http://127.0.0.1:3001/priority
special response (higher priority)
```

**`round-robin.toml`** - cycles through matches in order, one per
request:

```sh
$ for i in 1 2 3 4; do curl http://127.0.0.1:3001/round-robin; echo; done
server-a
server-b
server-c
server-a
```

**`weighted.toml`** - random, weighted by each rule's `weight`
(3:1 here). Genuinely random - run it enough times and the ratio
settles near 3:1, but any single request could return either:

```sh
$ for i in $(seq 1 30); do curl -s http://127.0.0.1:3001/weighted; echo; done | sort | uniq -c
     24 variant-a
      6 variant-b
```

## On `seed`

`WeightedRandom` and `UniformRandom` both accept an optional `seed`.
Set, it makes the pick **fully deterministic** - the same request
always returns the same response, useful for a reproducible test.
Omitted, as in `weighted.toml` here, every request is genuinely
random. A demo meant to be watched varying needs it omitted; the
verification harness for this set instead asserts the statistical
property ("both variants appear over N requests") rather than an exact
response, since the response is not fixed.
