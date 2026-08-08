# Dry-run a rule

`apimock match-test` builds a synthetic request from CLI flags and
checks it against a rule set directly — no server, no `curl`. The
right tool for "which rule would this request hit, and why" while
you're authoring a rule set.

```sh
$ apimock match-test --rule-set apimock-rule-set.toml \
    --path /orders --method POST --body '{"customer":{"tier":"gold"}}'
Rule #1: /orders  MATCH ★
  ✓  url_path equal "/orders"
  ✓  method POST (actual: POST)
  ✓  body.json "customer.tier"  ==  "gold"  (actual: "gold")

Rule #2: /orders  MATCH
  ✓  url_path equal "/orders"
  ✓  method POST (actual: POST)

Result: MATCH (rule #1)
```

Every rule is checked, and each one shows exactly which conditions
passed and which didn't — including the rule that *didn't* win, which
is often more useful than the one that did:

```sh
$ apimock match-test --rule-set apimock-rule-set.toml \
    --path /orders --method GET
Rule #1: /orders  NO MATCH
  ✓  url_path equal "/orders"
  ✗  method POST (actual: GET)
  ✗  body (request has no JSON body)

Result: NO MATCH
```

Exit codes: `0` matched, `1` no rule matched, `2` an argument or input
error. Unlike `apimock validate`, a bare relative `--rule-set` path
works fine here — this command doesn't hit the path-resolution issue
noted on the [CLI reference](../reference/cli-reference.md#apimock-match-test)
page.

Full flag list on the [CLI reference](../reference/cli-reference.md#apimock-match-test).
A worked, verified example (also covering
[`apimock validate`](./validate-config-in-ci.md)) is
[`crates/apimock/examples/validate-in-ci/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/validate-in-ci).
