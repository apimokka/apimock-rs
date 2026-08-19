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

These apply across the whole CLI, `match-test`, `validate` and `get`
included (each also documents its own diagnostic-specific codes below —
`get` in particular reuses `0`/`2` only, never `1`; see its own section):

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
apimock validate --config <path> [--strict] [--quiet] [--json] [--format text|json]
```

Loads the whole workspace — root config and every rule set it
references — and reports diagnostics, without binding a port.

| Flag | Meaning |
|---|---|
| `--config`, `-c <path>` | Required. The root config to validate. **Prefix a bare relative path with `./`** — unlike the top-level `-c` above, `validate` parses its own `--config` separately and a bare filename (no directory separator) currently fails to resolve even though the file exists; `--config ./apimock.toml` works, `--config apimock.toml` does not |
| `--strict` | Documented to treat warnings as failures (exit `1`). **Not reachable today** — see the note below the exit-codes table |
| `--quiet` | Suppress non-error output |
| `--json` | **Deprecated, removed in 6.0.0.** Emits the same bare diagnostics array 5.18.0 and earlier did, byte-identical — an existing parser reading stdout is unaffected. Using it prints a one-line warning to stderr, once, naming `--format json` as the replacement |
| `--format text` | Default. Today's plain-text summary — unchanged whether written explicitly or left implicit |
| `--format json` | The [RFC 053 response envelope](#the-response-envelope---format-json): an object with `schema`, `apimock`, and exactly one of `result`/`error`, instead of a bare array |

`--json` and `--format` may not be combined — that is a usage error,
exit `2`, not a silent precedence rule between the two.

Exit codes: `0` clean, `2` the config couldn't be loaded at all, or the
invocation itself was invalid (e.g. `--json --format json` together, or
`--format` given a value other than `text`/`json`).

**Exit `1` ("at least one error") is documented but not reachable
today, and neither is `--strict`'s effect.** `Workspace::load` — which
`validate` calls before it ever produces a diagnostic — already checks,
identically, every condition that could otherwise appear in the
diagnostics report (a `respond` block that's empty or has conflicting
fields, a `respond.file_path` that doesn't exist, a missing
`fallback_respond_dir`) and fails to load if any of them is present.
So a config either loads with zero diagnostics (exit `0`) or fails to
load (exit `2`) before reaching the exit-`1` path at all; nothing
anywhere constructs a `Severity::Warning` diagnostic either, so
`--strict` (which only promotes warnings to failures) has nothing to
act on even in principle. Documented as-is rather than fixed — a real
fix changes config-load validation shared with server startup, larger
than this page's scope.

### The response envelope (`--format json`)

Introduced in 5.19.0 (RFC 054), ahead of 6.0.0's `get`/`set` — `get`
(below) is the first of the two to actually use it. A successful
validation:

```json
{
  "schema": 1,
  "apimock": "5.19.0",
  "result": {
    "diagnostics": [
      { "severity": "error", "message": "…", "node_id": "…", "file": "…" }
    ],
    "summary": { "errors": 0, "warnings": 0, "rule_sets": 1, "rules": 2 }
  }
}
```

A config that failed to *load* (`validate` never got as far as
producing diagnostics):

```json
{
  "schema": 1,
  "apimock": "5.19.0",
  "error": { "kind": "config_invalid", "message": "…" }
}
```

`error.kind` is one of `usage`, `config_invalid`, `config_unreadable`,
`io`, `conflict`, `internal` — a closed, stable set; treat an
unrecognised value as a generic failure rather than erroring on it, since
new kinds may be added later without a `schema` bump. **A validation
that ran and found problems is still a `result`, not an `error`** — the
envelope's top-level shape answers "did this command run", not "is the
config valid"; check `result.summary.errors` for the latter. `schema`
starts at `1`; a later, incompatible change to this shape increments it.

## `apimock get`

```
apimock get <path> [-c <config>] [-m <METHOD>] [-H "Name: value"]... \
  [-b <json> | --body-file <path>] [--why] [--format text|json]
```

Answers *what would the server return for this request* — status,
headers, body — from configuration on disk, with no server running.
Unlike `match-test`, it answers from the whole workspace (`apimock.toml`
and everything it references), and covers **every** dispatch stage the
server does, in the same order: `OPTIONS` → rule sets → the fallback
directory. A zero-config workspace (no rule sets at all) is answered
correctly, because the fallback-directory stage is where zero-config
mode's answers come from — a `get` that stopped at rule sets would be
wrong for that case, which is most of them.

| Flag | Meaning |
|---|---|
| `--config`, `-c <path>` | The root config to answer from. Default: `./apimock.toml` if it exists, otherwise zero-config — same resolution the server itself uses |
| `--method`, `-m <METHOD>` | The request's HTTP method (default: `GET`) |
| `--header`, `-H "Name: value"` | Add a header; repeatable |
| `--body`, `-b <json>` | The request's JSON body, inline |
| `--body-file <path>` | The request's JSON body, from a file |
| `--why` | Explain which rule set and rule decided the answer, and for a near-miss, which specific condition failed. Off by default in text, **on by default with `--format json`** |
| `--format text` | Default. Human-readable |
| `--format json` | The [RFC 053 response envelope](#the-response-envelope---format-json), including provenance (the absolute paths of the config and rule sets that answered) |

**`--format json`'s `matched` object also carries `rule_set_file`**
alongside `rule_set_index`/`rule_index` — the same rule-set path
`--why` reports (see below), added so the address can be handed to
[`apimock set`](#apimock-set)'s `--rule-set`/`--rule` unmodified,
without a second `--why` round trip just to learn the path.

**Middleware is never executed.** If any is configured, the answer says
so explicitly (`middleware.configured`/`middleware.note` in JSON, a
console note in text) and proceeds anyway — the response may be wrong if
a middleware would have intercepted the request, and the answer is
marked incomplete rather than silently omitting that risk. There is no
flag to run middleware; that would mean executing Rhai scripts as a side
effect of a read command, which this project's stated preference for the
safer option rules out.

**Exit codes deliberately differ from `match-test`'s.** `get` exits `0`
even when nothing matched — a 404, or "no rule matched", is a legitimate
answer to a legitimate question (RFC 053: this is a `result`, not an
`error`). `match-test` still exits `1` on no match; the two commands
answer similar-sounding questions with different exit semantics on
purpose, documented here rather than aligned, since changing
`match-test`'s exit code now would be an unannounced breaking change.

Exit codes: `0` answered (including a 404 or no match), `2` a bad
invocation or the configuration couldn't be loaded.

**Two honest limits, both narrow.** A `[[rules]] strategy = "round_robin"`
(or `uniform_random`/`weighted_random`, or `priority` with a
`uniform_random` tiebreak) rule set can answer differently from what a
*running* server would return next. `get` loads its own rule sets fresh
from disk each run, with their own round-robin counter starting at `0`
and their own random draw — it has no way to observe how far a live
server's selector has already advanced, or to reproduce an unseeded
draw, so its answer is one legitimate possibility, not a prediction of
the server's next response. There's no fix for this: it's the same
drift a static answer always risks against live state, which is exactly
what [provenance](#the-response-envelope---format-json) exists to name
rather than hide. `strategy = "first_match"` (the default) and
`priority` with the default `first_match` tiebreak are unaffected — both
are deterministic from the request alone. Separately, a response body
that isn't valid UTF-8 is shown with replacement characters rather than
round-tripping exactly, in both `--format text` and `--format json` — a
mock server's bodies are expected to be JSON or text, so this is
believed to be a narrow gap rather than a common one.

### `--why`'s JSON shape

```json
"why": {
  "note": "Answered from a rule set.",
  "rule_sets": [
    {
      "rule_set_index": 0,
      "rule_set_file": "/abs/path/apimock-rule-set.toml",
      "rules": [
        {
          "rule_index": 0,
          "matched": false,
          "conditions": [
            { "name": "url_path", "expectation": "equal \"/orders\"", "actual": "/orders", "matched": true },
            { "name": "body.json:customer.tier", "expectation": "equal \"gold\"", "actual": "\"silver\"", "matched": false }
          ]
        }
      ]
    }
  ]
}
```

Only the rule sets dispatch actually consulted are listed — if an
earlier one answered, later ones were never reached by the server
either, so they aren't listed here. `actual` is always present, even
for conditions whose text-format output never showed it historically
(`url_path`, headers) — the JSON shape is not constrained to match
`match-test`'s older, narrower text output.

## `apimock set`

```
apimock set rule [-c <config>] [--rule-set <path>] [--rule <n>] \
  [--path <url_path>] [--method <METHOD>] [-H "Name: value"]... \
  [--status <code>] [--json <value> | --text <value>] [--file <path>] \
  [--delay <ms>] [--dry-run] [--format text|json]
```

Adds a rule (the default), or changes an existing one when `--rule` is
given, and writes it to the rule-set file — keeping that file's
comments and formatting (RFC 056).
Neither the root config nor the rule-set file need to exist yet — a
fresh directory gets a minimal starting pair of files, not the
example-filled scaffold `--init` writes.

**Addressing is by natural key, never a process ID.** Every
`apimock set` invocation is a new process, so a new load of the
config — anything keyed by a process-local ID would be meaningless to
the next invocation. `set` addresses a rule by `(rule-set file path,
0-based rule index)` instead — the same shape [`get`'s `--format json`
`matched`/`--why`](#apimock-get) already reports. An address printed
by `get` can be passed to `--rule-set`/`--rule` unmodified.

| Flag | Meaning |
|---|---|
| `--config`, `-c <path>` | The root config to edit. Default: `./apimock.toml`, created if absent |
| `--rule-set <path>` | The rule-set file to add to, or edit within. Default: `./apimock-rule-set.toml`, created if absent |
| `--rule <n>` | Edit the existing rule at this **0-based** index, instead of adding a new one |
| `--path <url_path>` | The rule's `url_path` condition |
| `--method <METHOD>` | The rule's method condition |
| `--header`, `-H "Name: value"` | Add a header condition; repeatable. With `--rule`, layers onto the existing rule's conditions rather than replacing them |
| `--status <code>` | The response status code |
| `--json <value>` | The response body, as JSON (validated at parse time, stored as text) |
| `--text <value>` | The response body, as plain text — mutually exclusive with `--json` |
| `--file <path>` | The response body, served from a file |
| `--delay <ms>` | Delay the response by this many milliseconds |
| `--dry-run` | Show what would change, without writing anything |
| `--format text` | Default. Human-readable |
| `--format json` | The [RFC 053 response envelope](#the-response-envelope---format-json) |

**`--rule`'s index is 0-based**, matching `get`'s JSON contract rather
than its 1-based text display — the machine-readable convention, since
that is the one meant to compose. Addressing a rule set by a path not
in `service.rule_sets` when `--rule` is also given, or an out-of-range
rule index, is a `usage` error — not a panic, and not a silent no-op.

**A file changed on disk since it was loaded is refused, not
overwritten** (RFC 056) —
`error.kind: "conflict"`, distinguished from an unrelated read failure
(`"io"`). No file is modified when either happens.

**`--dry-run` never reports a `NodeId`.** Its preview resolves every
changed node back to the same natural-key address `set` accepts, the
same way a successful save's own `changes` array does — nothing
process-local ever appears in `set`'s output, on any path, success or
error.

**Scope of this cut.** `service.middlewares` is never added, changed
or removed by any `set` invocation — existing entries pass through
untouched (RFC 048 § 9 T2, deferred rather than refused). `DeleteRule`,
`MoveRule` and `RemoveRuleSet` aren't reachable from `set` yet — those
renumber existing rules, which would break the positional address this
command's whole design depends on staying stable across invocations.
One rule change per invocation; there is no batch flag.

Exit codes: `0` applied (or, under `--dry-run`, would apply), `1`
loaded and addressed successfully but the save failed (conflict, io,
or an internal error), `2` a bad invocation or the configuration
couldn't be loaded.

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
