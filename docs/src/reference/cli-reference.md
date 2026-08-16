# CLI reference

## `--version` and `--help`

```
apimock --version
apimock --help
apimock <subcommand> --help
```

Both short-circuit before anything else — before a config file is read
and before any listener binds. They work with no config file present
and with a deliberately broken one; that's deliberate, not incidental:
"what version am I running" is the question asked precisely when
something is wrong. `--help` (or `-h`) is reachable per subcommand too —
`apimock match-test --help` and `apimock validate --help` print that
subcommand's own usage, not the top-level one.

Output goes to stdout; exit code `0`.

## Unrecognised arguments

Anything starting with `-` that isn't one of the flags documented on
this page is an error, not silently ignored:

```
$ apimock --prot 4000
apimock: unknown option '--prot'; did you mean '--port'?
```

A near-match suggestion appears where one exists. The message goes to
stderr, exit code `2`, and no server is started — a typo used to start a
server on a port nobody asked for; now it doesn't start anything.

## Exit codes

These apply across the whole CLI, `match-test` and `validate` included
(each also documents its own diagnostic-specific codes below):

| Code | Meaning |
|---|---|
| `0` | Success, including `--version` / `--help` |
| `2` | Usage error — an unrecognised option, or a known option given a value that doesn't parse (e.g. `--port notanumber`) |
| `1` | Everything else, including a known option given **no** value at all (e.g. `-c` with nothing after it) — the same code as a referenced file not existing |

A flag given with no value is indistinguishable, at the point the
argument list is scanned, from a boolean flag's mere presence (`--init`
takes no value; `-c` normally does) — telling them apart would mean
changing that scan, which every other flag's exact behaviour depends on
staying untouched. So `-c` with nothing after it isn't caught as a
usage error; it falls through and fails later, the same way it always
has, as exit `1`.

## Running the server

```
apimock [-p <port>] [-d <dir>] [-c <config>] [--init [--yes] [--middleware]]
```

| Flag | Result |
|---|---|
| *(no flags)* | Zero-config: serves `./` by URL path, port `3001` |
| `-p`, `--port <port>` | Listen on a custom port |
| `-d <dir>` | Serve a custom fallback directory instead of `./` |
| `-c`, `--config <path>` | Load a config file. A bare relative path resolves the same as one prefixed with `./` — `-c apimock.toml` and `-c ./apimock.toml` are equivalent |

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
| `--config`, `-c <path>` | Required. The root config to validate. **Prefix a bare relative path with `./`** — unlike the top-level `-c` above, `validate` parses its own `--config` separately and a bare filename (no directory separator) currently fails to resolve even though the file exists; `--config ./apimock.toml` works, `--config apimock.toml` does not |
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
