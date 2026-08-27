# Add or change a rule

`apimock set rule` writes a rule to disk — no editor, no hand-authored
TOML. Adds a new rule by default; pass `--rule <n>` to change an
existing one instead. Either way it keeps the target file's comments
and formatting, and the config/rule-set files don't need to exist yet —
a fresh directory gets a minimal starting pair.

```sh
$ apimock set rule --path /orders/1 --status 200 \
    --json '{"id":1,"status":"shipped"}'
Applied:
  rule set: apimock-rule-set.toml (new rule)
  Added: rule set `./apimock-rule-set.toml` — rule set #1 (apimock-rule-set.toml): rules=1
  Updated: root config — apimock.toml: listener / log / service
```

That wrote:

```toml
[[rules]]

[rules.respond]
json = '{"id":1,"status":"shipped"}'
status = 200

[rules.when.request]
url_path = "/orders/1"
```

## Where it writes, and how it decides add vs. change

`--config`/`-c` (default `./apimock.toml`) and `--rule-set` (default
`./apimock-rule-set.toml`) name the two files; both are created if
missing. Without `--rule`, the command **adds** a new rule — always
appended, never inserted, since the address every `set` invocation
uses to find a rule again (`--rule-set` path + 0-based index) has to
stay stable across runs. With `--rule <n>`, it **changes** the rule
already at that index instead — give it whatever flags actually
differ; anything you don't pass is left as it was.

```sh
$ apimock set rule --rule 0 --status 404 \
    --json '{"error":"not found"}' --dry-run
Would apply (--dry-run, nothing written):
  rule set: apimock-rule-set.toml, rule #0
  Updated: rule set `./apimock-rule-set.toml`, rule #0 — rule #1 in rule set #1
```

`--dry-run` previews the exact change and writes nothing, full stop —
not even a bootstrap file, if the workspace didn't exist yet. Drop it
to actually apply.

## `--json`, not `--text`, for a JSON body

`--json <value>` validates the value as JSON, writes it to
`respond.json`, and it's served as `application/json`. `--text <value>`
writes `respond.text` and is always `text/plain`, even if the value
happens to look like JSON — the two are mutually exclusive on purpose,
so a body's content-type is a declared choice, not a guess (see
[Rule-set schema](../reference/rule-set-schema.md#respond)). Get this
wrong and the body is right but the header is not, exactly the kind of
mismatch a strict client library rejects.

## `--format json`

```sh
$ apimock set rule --path /orders/2 --status 201 \
    --json '{"id":2,"status":"pending"}' --format json
{
  "apimock": "5.19.0",
  "result": {
    "changed_files": [
      "./apimock-rule-set.toml"
    ],
    "changes": [
      {
        "kind": "Added",
        "summary": "added rule #2 in rule set #1",
        "target": "rule set `./apimock-rule-set.toml`, rule #1"
      }
    ],
    "dry_run": false,
    "requires_reload": true,
    "rule_set": "apimock-rule-set.toml"
  },
  "schema": 1
}
```

`requires_reload` tells a caller whether a running server needs a
restart or reload to pick up the change — currently always `true` when
anything changed, since apimock doesn't yet reload on its own. A
failure carries `error.kind` instead of `result` — see the
[response envelope](../reference/cli-reference.md#the-response-envelope---format-json).

## Exit codes

`0` applied (or, under `--dry-run`, would apply). `2` a bad invocation
— an unknown or dangling flag, a target outside the confined directory
— writes nothing at all, verified by asserting file contents unchanged
rather than just reading the exit code. `1` the rule was loaded and
addressed successfully but the save itself failed (a conflicting
external edit, or an I/O error). Full flag list and every exit code on
the [CLI reference](../reference/cli-reference.md#apimock-set).

A worked, verified example —
[`crates/apimock/examples/agent-bootstrap/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/agent-bootstrap) —
walks through bootstrapping a workspace from nothing with `set`,
checking it with [`get`](./check-what-a-request-returns.md), and
validating it, in the order an agent would actually run them.
