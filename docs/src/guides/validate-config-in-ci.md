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

**Use `./apimock.toml`, not a bare `apimock.toml`** — a relative
`--config` path with no directory separator currently fails to
resolve even though the file exists. See the
[CLI reference](../reference/cli-reference.md#running-the-server) for
this and every other flag.

`--json` emits the diagnostics array instead, for machine consumption:

```sh
$ apimock validate --config ./apimock.toml --json
[]
Validation passed (2 rules across 1 rule set(s)).
```

Exit codes: `0` clean, `1` at least one error (or, with `--strict`,
any warning), `2` the config couldn't even be loaded — any of these is
enough to fail a CI step.

A worked, verified example (also covering
[`apimock match-test`](./dry-run-a-rule.md)) is
[`crates/apimock/examples/validate-in-ci/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/validate-in-ci).
