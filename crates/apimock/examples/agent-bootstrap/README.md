# Agent bootstrap

RFC 048's own acceptance test for `get`/`set` (the RFC calls it "W7"):
build a mock, from nothing, in a clean directory, with no editor and no
hand-written TOML — the exact shape an agent's own tool use looks like.
Every command below was run for real; there's no `apimock.toml` or
`apimock-rule-set.toml` checked in here, because the walkthrough is
what creates them.

`cd` into an empty directory before following along.

## 1. Add the specific rule first

`set rule` always **appends** — a rule's address (rule-set file +
0-based index) has to stay stable across invocations, so nothing ever
reorders existing rules. `FirstMatch` (the default strategy) picks the
first rule in file order whose conditions hold, with no specificity
tie-break. That means order matters: the header-gated rule has to be
added **before** the general fallback, or the fallback — being tried
first — would answer every request regardless of the header.

```sh
$ apimock set rule --path /users/1 --header "x-api-key: k1" --status 403
Applied:
  rule set: apimock-rule-set.toml (new rule)
  Added: rule set `./apimock-rule-set.toml` — rule set #1 (apimock-rule-set.toml): rules=1
  Updated: root config — apimock.toml: listener / log / service
```

Neither `apimock.toml` nor `apimock-rule-set.toml` existed before this;
`set` created both with a minimal starting shape, no `--init` needed.

## 2. Add the general fallback second

```sh
$ apimock set rule --path /users/1 --status 200 --json '{"id":1}'
Applied:
  rule set: apimock-rule-set.toml (new rule)
  Added: rule set `./apimock-rule-set.toml`, rule #1 — added rule #2 in rule set #1
```

`--json` here — not `--text` — matters: it writes `respond.json` and
gets served as `application/json`. `--text '{"id":1}'` would write the
identical-looking body but serve it as `text/plain`, since `text` never
inspects its own content to guess a content-type (see
[Add or change a rule](https://apimokka.github.io/apimock-rs/guides/add-or-change-a-rule.html)).

## 3. Check it — without a server

```sh
$ apimock get /users/1
GET /users/1

Status: 200
Headers:
  content-length: 8
  ...
  content-type: application/json
Body:
{"id":1}

Answered: rule set #1, rule #2
```

("Answered: rule set #1, rule #2" — the *second* rule, since this
request has no `x-api-key` header, so the first (specific) rule's
condition fails and `get` falls through to the general one. `...`
elides the default header set, unchanged from request to request.)

## 4. Check the header-gated path too

```sh
$ apimock get /users/1 --header "x-api-key: k1"
GET /users/1

Status: 403
...
Body:


Answered: rule set #1, rule #1
```

Same URL path, different outcome — the header satisfies the first
rule's condition, so `FirstMatch` never reaches the second one.

## 5. Validate the result

```sh
$ apimock validate -c apimock.toml
Validation passed (2 rules across 1 rule set(s)).
```

A bare relative `-c` (no `./` prefix) resolves correctly here — worth
calling out explicitly, since this exact command used to fail even
though the file existed right there (fixed across every subcommand and
the root parser at once).

## What got written

```toml
[[rules]]

[rules.respond]
status = 403

[rules.when.request]
url_path = "/users/1"

[rules.when.request.headers.x-api-key]
op = "equal"
value = "k1"

[[rules]]

[rules.respond]
json = '{"id":1}'
status = 200

[rules.when]

[rules.when.request]
url_path = "/users/1"
```

Five commands, no editor, no server started until you choose to run
`apimock` for real against the file this walkthrough produced.

## Further reading

- [Add or change a rule](https://apimokka.github.io/apimock-rs/guides/add-or-change-a-rule.html) — the full `set` guide
- [Check what a request returns](https://apimokka.github.io/apimock-rs/guides/check-what-a-request-returns.html) — the full `get` guide
- [CLI reference](https://apimokka.github.io/apimock-rs/reference/cli-reference.html) — every flag, every exit code
