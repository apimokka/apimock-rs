# CLI reference

## Running the server

```
apimock [-p <port>] [-d <dir>] [-c <config>] [--init [--yes] [--middleware]]
```

| Flag | Result |
|---|---|
| *(no flags)* | Zero-config: serves `./` by URL path, port `3001` |
| `-p`, `--port <port>` | Listen on a custom port |
| `-d <dir>` | Serve a custom fallback directory instead of `./` |
| `-c`, `--config <path>` | Load a config file. **Prefix relative paths with `./`** — a bare filename (no directory separator) currently fails to resolve even though the file exists; `-c ./apimock.toml` works, `-c apimock.toml` does not |

## `--init`

Scaffolds a starting config in the current directory. Never overwrites
an existing `./apimock.toml`.

| Flag | Result |
|---|---|
| `--init` | Interactive: prompts for port, IP, fallback dir, whether to scaffold a rule-set file, a middleware file, and a TLS section. Writes `apimock.toml`, plus whichever of `apimock-rule-set.toml` / `apimock-middleware.rhai` you opted into |
| `--init --yes` | Non-interactive: writes the same defaults every prompt above defaults to (`127.0.0.1:3001`, rule-set file included, TLS commented out), no prompts |
| `--init --middleware` | Also scaffold `apimock-middleware.rhai`. Combines with `--yes` |

When stdin isn't a TTY (piped, CI, a Docker build), `--init` silently
falls back to the same defaults `--yes` would produce, even without
`--yes` explicitly passed.

## `apimock validate`

```
apimock validate --config <path> [--strict] [--quiet] [--json]
```

Loads the whole workspace — root config and every rule set it
references — and reports diagnostics, without binding a port.

| Flag | Meaning |
|---|---|
| `--config`, `-c <path>` | Required. The root config to validate |
| `--strict` | Treat warnings as failures too (exit `1`, not just `0`) |
| `--quiet` | Suppress non-error output |
| `--json` | Emit diagnostics as a JSON array instead of the plain-text summary |

Exit codes: `0` clean, `1` at least one error (or any warning under
`--strict`), `2` the config couldn't be loaded at all.

## `apimock match-test`

```
apimock match-test --rule-set <path> [--rule <n>] [--path <url_path>] \
  [--method <METHOD>] [--header "Name: value"]... \
  [--body <json> | --body-file <path>] [--quiet]
```

Builds a synthetic request from the flags below and checks it against
a rule set directly — no server, no network request. Prints a
per-condition breakdown for every rule (or just the one named by
`--rule`), then a final `Result: MATCH (rule #N)` or `Result: NO MATCH`
line.

| Flag | Meaning |
|---|---|
| `--rule-set`, `-r <path>` | Required. The rule-set file to check against |
| `--rule <n>` | Check only this rule, 1-based |
| `--path`, `-p <url_path>` | The synthetic request's URL path |
| `--method`, `-m <METHOD>` | The synthetic request's HTTP method |
| `--header`, `-H "Name: value"` | Add a header; repeatable |
| `--body`, `-b <json>` | The synthetic request's JSON body, inline |
| `--body-file <path>` | The synthetic request's JSON body, from a file |
| `--quiet`, `-q` | Suppress the per-condition breakdown, print only the result |

Exit codes: `0` matched, `1` no rule matched, `2` an argument or input
error (bad flag, file not found, invalid JSON body).

See [Validate config in CI](../guides/validate-config-in-ci.md) and
[Dry-run a rule](../guides/dry-run-a-rule.md) for worked examples of
both commands, including their exact output.
