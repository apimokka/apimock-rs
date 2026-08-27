# Validate config in CI

`apimock validate` loads a whole workspace — the root config and every
rule set it references — and reports diagnostics, without binding a
port. The right shape for a CI step or a pre-commit hook.

```sh
$ apimock validate --config ./apimock.toml
Validation passed (2 rules across 1 rule set(s)).
$ echo $?
0
```

A bare relative `--config apimock.toml` (no `./` prefix) resolves the
same way — see the
[CLI reference](../reference/cli-reference.md#running-the-server) for
this and every other flag.

`--format json` emits a machine-readable response instead — an object
with `schema`, `apimock`, and a `result` carrying the diagnostics array
plus a summary:

```sh
$ apimock validate --config ./apimock.toml --format json
{
  "schema": 1,
  "apimock": "5.19.0",
  "result": {
    "diagnostics": [],
    "summary": { "errors": 0, "warnings": 0, "rule_sets": 1, "rules": 2 }
  }
}
```

**`--json` (the bare diagnostics array, no envelope) was removed in
6.0.0.** Using it now fails with exit `2` and a message naming
`--format json` as the replacement, instead of the array a script might
still be parsing — see the
[migration guide](./migrating-to-6-0.md#cli-apimock-validate---json-is-removed)
for the exact error text. `--format json` is the one shape to build
new CI steps against; see the
[CLI reference](../reference/cli-reference.md#the-response-envelope---format-json)
for the full envelope shape and `error.kind` values.

Exit codes: `0` clean, `2` the config couldn't even be loaded — either
is enough to fail a CI step. **Exit `1` ("at least one error") and
`--strict` are documented but not reachable today**: every condition
that would produce a diagnostic is already checked, identically, at
load time, so a config with a problem fails to load (exit `2`) before
`validate` ever gets to report it as a diagnostic instead. See the
[CLI reference](../reference/cli-reference.md#apimock-validate) for the
detail.

A worked, verified example (also covering
[`apimock match-test`](./dry-run-a-rule.md)) is
[`crates/apimock/examples/validate-in-ci/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/validate-in-ci).
