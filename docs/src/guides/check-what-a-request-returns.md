# Check what a request returns

`apimock get <path>` answers *what would the server return for this
request* — status, headers, body — by reading configuration from disk.
No server has to be running. Unlike
[`match-test`](./dry-run-a-rule.md), it answers from the **whole
workspace**: `OPTIONS` handling, every rule set in order, and the
fallback directory if nothing matched — the same dispatch order, in
the same code paths, the running server actually uses.

```sh
$ apimock get /orders/1
GET /orders/1

Status: 200
Headers:
  content-length: 27
  ...
  content-type: application/json
Body:
{"id":1,"status":"shipped"}

Answered: rule set #1, rule #1
```

(`...` above elides the default header set — CORS, cache-control, and
so on, the same on every response — see
[Response headers](../reference/response-headers.md) for the full
list.)

A request nothing matches is still a normal answer, not an error:

```sh
$ apimock get /nope
GET /nope

Status: 404
...
Body:


Answered: fallback directory (no rule set matched)
```

That's deliberate — a 404 is a legitimate thing to ask about, and `get`
exits `0` for it. `match-test` is the one that exits `1` on no match,
because its whole purpose is checking whether a rule matches; the two
commands answer similar-sounding questions on purpose with different
exit semantics — see the
[CLI reference](../reference/cli-reference.md#apimock-get) for exactly
why.

## `--why`

Add `--why` to see which rule matched, and — more usefully — why the
rules that *didn't* match, didn't:

```sh
$ apimock get /orders/1 --why
...
-- Why --
Answered from a rule set.

Rule set #1 (./apimock-rule-set.toml):
  Rule #1: MATCH
    ✓  url_path equal "/orders/1" (actual: /orders/1)
  Rule #2: NO MATCH
    ✗  url_path equal "/orders/2" (actual: /orders/1)
```

## `--format json`

```sh
$ apimock get /orders/1 --format json
{
  "apimock": "5.19.0",
  "result": {
    "matched": { "rule_index": 0, "rule_set_file": "./apimock-rule-set.toml", "rule_set_index": 0 },
    "request": { "method": "GET", "path": "/orders/1" },
    "response": {
      "body": "{\"id\":1,\"status\":\"shipped\"}",
      "headers": [
        { "name": "content-type", "value": "application/json" }
      ],
      "status": 200
    },
    "source": {
      "config": "/abs/path/to/apimock.toml",
      "rule_sets": ["/abs/path/to/apimock-rule-set.toml"]
    },
    "stage": "rule_set"
  },
  "schema": 1
}
```

(`headers` is trimmed above to the one line that changes per request;
a real response repeats the same default set shown in the text example.)

**`--why` is included by default under `--format json`**, even without
passing the flag — an agent reading structured output gets the
explanation without a second round trip. It stays off by default in
text, so a quick human check isn't buried in it.

`source` gives the absolute, resolved paths of the config and every
rule set consulted — provenance for an answer that came from files on
disk, not from a running process. `matched.rule_set_file` is the same
path [`set --rule-set`](./add-or-change-a-rule.md) accepts, so an
address `get` reports can be handed straight to `set` to change the
rule that produced it, no translation needed.

## Two honest limits

A rule set using a randomised or round-robin strategy can answer
differently from what a *running* server would return next — `get`
loads its rule sets fresh each run, with no way to see how far a live
server's own selector has advanced. `strategy = "first_match"` (the
default) is unaffected; it's deterministic from the request alone. And
a non-UTF-8 response body is shown with replacement characters rather
than round-tripping exactly, in both formats. Neither is expected to
matter often — see the
[CLI reference](../reference/cli-reference.md#apimock-get) for the
full detail.

## Exit codes

`0` answered — including a 404 or "no rule matched"; that's a result,
not a failure. `2` a bad invocation, or the configuration couldn't be
loaded. `get` never exits `1`. Full flag list on the
[CLI reference](../reference/cli-reference.md#apimock-get).

A worked, verified example —
[`crates/apimock/examples/agent-bootstrap/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/agent-bootstrap) —
uses `get` to check a rule right after
[`set`](./add-or-change-a-rule.md) writes it, before ever starting a
server.
