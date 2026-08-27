# Validate config in CI

Two commands that check a config or a rule set without starting a
server - the right shape for a CI step or a pre-commit hook. This set
has no `curl` walkthrough; both commands are the thing being
demonstrated, and their exit code and output are what a CI job checks.

## `apimock validate`

Loads the whole workspace (root config + every rule set it references)
and reports diagnostics without binding a port.

```sh
$ cd crates/apimock/examples/validate-in-ci
$ apimock validate --config ./apimock.toml
Validation passed (2 rules across 1 rule set(s)).
$ echo $?
0
```

`--json` emits the diagnostics array instead (empty here, since
there's nothing to report). **Deprecated as of 5.19.0** — still works,
byte-identical, but now prints a one-line warning to stderr; `--format
json` (the [CLI reference](../../../../docs/src/reference/cli-reference.md#the-response-envelope---format-json))
is the replacement for new CI steps:

```sh
$ apimock validate --config ./apimock.toml --json
[]
Validation passed (2 rules across 1 rule set(s)).
```

Exit codes: `0` clean, `2` the config couldn't even be loaded. **Exit
`1` and `--strict` are documented but not reachable through this
command** — the load step above already rejects, identically, every
condition that would otherwise become a reported diagnostic, so a
problem fails validation by failing to *load* (exit `2`), before
`--strict` or an exit-`1` "N error(s)" outcome ever come into play. See
the [CLI reference](../../../../docs/src/reference/cli-reference.md#apimock-validate)
for the detail.

## `apimock match-test`

Builds a synthetic request from CLI flags and checks it against a rule
set directly - no server, no `curl`. Useful for asking "which rule
would this request hit, and why" while authoring a rule set.

A gold-tier order matches the first, more specific rule:

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

A non-gold order falls through to the second, more general rule - note
rule #1 is shown failing on exactly the condition that excluded it:

```sh
$ apimock match-test --rule-set apimock-rule-set.toml \
    --path /orders --method POST --body '{"customer":{"tier":"silver"}}'
Rule #1: /orders  NO MATCH
  ✓  url_path equal "/orders"
  ✓  method POST (actual: POST)
  ✗  body.json "customer.tier"  ==  "gold"  (actual: "silver")

Rule #2: /orders  MATCH ★
  ✓  url_path equal "/orders"
  ✓  method POST (actual: POST)

Result: MATCH (rule #2)
```

A `GET` matches neither rule (both require `POST`) - exit code `1`,
the "no match" signal a CI step would check for:

```sh
$ apimock match-test --rule-set apimock-rule-set.toml --path /orders --method GET
Rule #1: /orders  NO MATCH
  ✓  url_path equal "/orders"
  ✗  method POST (actual: GET)
  ✗  body (request has no JSON body)

Rule #2: /orders  NO MATCH
  ✓  url_path equal "/orders"
  ✗  method POST (actual: GET)

Result: NO MATCH
$ echo $?
1
```

Exit codes: `0` matched, `1` no rule matched, `2` an argument or input
error (bad flag, file not found, invalid JSON body).
