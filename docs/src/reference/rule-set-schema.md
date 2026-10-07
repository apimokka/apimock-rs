# Rule-set schema

A rule-set file — one of the paths listed in `service.rule_sets` — has
five possible top-level tables/keys, only one of which (`[[rules]]`) is
required.

```toml
strategy = "round_robin"          # optional: overrides service.strategy, this file only

[prefix]
url_path = "/api/v2"
respond_dir = "responses"

[default]
delay_response_milliseconds = 1000   # applies to rules that set no delay of their own

[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
when.request.headers.x-api-key = { op = "exists" }
when.request.body.json."customer.tier" = { op = "equal", value = "gold" }
respond = { file_path = "vip-order.json" }
priority = 10
weight = 3
```

## `strategy` (top-level, optional)

**From 6.4.0, name the strategy:** `strategy = "first_match"`,
`"round_robin"`, `"uniform_random"`, `"weighted_random"` or `"priority"`.
A bare name is that strategy with every option at its default. **Add a
table only to set an option:** `strategy = { weighted_random = { seed = 7 } }`,
or `strategy = { priority = { tiebreaker = "uniform_random" } }`. A table
with no options, `{ weighted_random = {} }`, means the same as the bare
name. **On 6.3.0 and earlier, only `"first_match"` and `"round_robin"` load
as bare names; the other three must be written as a table**, even with no
options (`{ weighted_random = {} }`), or the config fails to load. The table
form works on every release. Overrides `service.strategy` for this rule set
only. A name that is none of the five is refused, with the five listed.

> **Corrected 2026-10-06.** This page previously listed
> `"uniform_random"` and `"weighted_random"` as bare strings. **That was
> false:** written that way, the config fails to load with *"invalid
> type: unit variant, expected struct variant"*. Both have taken options
> (a `seed`) since at least 5.18.0, so the bare form has never worked.
> The guide below always showed the table form correctly.

> **Changed in 6.4.0 (RFC 085, 2026-10-07).** The bare names now load.
> What the note above describes is true of 6.3.0 and earlier: there,
> `strategy = "weighted_random"` failed to load, and only the table form
> worked. From 6.4.0 both spellings load to the same strategy, and the
> table form is needed only to set a `seed` or a `tiebreaker`. Nothing
> that loaded before stops loading.

See [Vary the response for one path](../guides/vary-the-response-for-one-path.md)
for the full syntax of all five.

## `[prefix]`

| Field | Meaning |
|---|---|
| `url_path` | Stripped from the front of the request path before this rule set's rules are matched — a rule's own `when.request.url_path` only needs to name what comes after it |
| `respond_dir` | Prepended to every `respond.file_path` in this rule set |

`url_path` matches at a segment boundary, not as a raw prefix: `/api`
matches `/api` and `/api/x`, never `/apixyz` or `/apix`. A rule set
scoped to `/api` never claims a request to an unrelated, similarly-
spelled path.

## `[default]`

The only field is `delay_response_milliseconds`. It sets a delay for
**every rule in this file that does not set one itself** — a per-rule
`respond.delay_response_milliseconds` always wins, including when it is
`0`, which cancels the default for that one rule.

> **Corrected 2026-10-06.** This page previously said this field "has no
> effect on any response". **That was false.** It was implemented by
> RFC 045 (Defect 2) and has worked since; the documentation described a
> limitation that the fix had already removed. Verified against a
> running server: with `[default] delay_response_milliseconds = 1500`, a
> rule that sets no delay answers in 1.502s, and a rule setting
> `respond.delay_response_milliseconds = 0` answers in 0.001s.

See [Simulate slow or flaky backends](../guides/simulate-slow-or-flaky-backends.md).

## `[guard]` — deprecated, do not use

**Deprecated as of 2026-10-06 and scheduled for removal.** It is no
longer shown in the example above, because it never did anything and
this page should not teach it.

A zero-field table — there is nothing to put inside it, and a `[guard]`
block with any content fails to parse. It carries a `// todo:` in the
source for a rule-set-wide condition that was never implemented, and
nothing reads it.

**If you have `[guard]` in a rule-set file, delete the line.** It has no
effect, so removing it cannot change how your rules behave. Loading a
file that contains it prints a warning on stderr naming the file; the
file still loads, and the command still exits 0. A future release will
reject the key outright, at which point a file still carrying it will
fail to load with the key named.

## `[[rules]]`

Each rule is `when` (what has to be true of the request) plus `respond`
(what to send back), plus two optional strategy-specific fields.

### `when.request`

At least one of the following is required; multiple conditions within
one rule are ANDed.

| Field | Shape |
|---|---|
| `url_path` | A bare string (implies `op = "equal"`), or `{ value = "...", op = "..." }` |
| `method` | A bare HTTP method string, spelled in uppercase: `"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, or `"PATCH"`. Config is case-sensitive, so `"patch"` is refused; a request's method on the wire still matches case-insensitively. Two methods are deliberately not matchable — see below |
| `headers.<name>` | `{ value = "...", op = "..." }` per header, ANDed; header names match case-insensitively |
| `body.json."<dotted.path>"` | `{ value = "...", op = "..." }` per path, ANDed — see [Body path syntax](./body-path-syntax.md) |

Every operator for `url_path`/`headers`/`body.json` is listed in the
[Operator reference](./operator-reference.md).

#### Which methods a rule can match, and why not all of them

A rule's `method` accepts `GET`, `POST`, `PUT`, `DELETE`, and `PATCH`.
Any other method is refused when the config loads, and the refusal says
why when there is a reason:

```
unknown variant `OPTIONS`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`
  — OPTIONS is answered by the built-in CORS preflight handler before rule sets are consulted, so it cannot be matched by a rule
```

- **`OPTIONS` cannot be matched.** Every `OPTIONS` request is answered by
  apimock's CORS preflight handler before rule sets are consulted, so a
  rule naming it would validate and then never run. Preflight stays
  authoritative. A rule cannot override it.
- **`HEAD` is not matchable yet.** A rule that answers `HEAD` could return
  a response body, which HTTP forbids for `HEAD`. Matching `HEAD` needs
  body suppression first, and that is a separate change.
- **`TRACE` and `CONNECT` are not supported.** `TRACE` is commonly disabled
  as a security measure and has no meaning for a mock server. `CONNECT` is a
  proxy mechanism with no meaning here.

Config spelling is case-sensitive, so `method = "patch"` is refused, the
same as `"get"` always was. The refusal names the correct spelling
(``did you mean `PATCH`?``), or, for a method that is excluded on purpose, gives
its reason instead, since the right case would still not match. A request's
method on the wire is matched case-insensitively, so a client sending `patch`
still matches a `PATCH` rule.

### `respond`

At least one of `file_path`, `text`, `json`, or `status` is required.

| Field | Meaning |
|---|---|
| `file_path` | Serve this file's content — extension decides JSON/JSON5/CSV/binary/text handling |
| `text` | A literal response body, always served as `text/plain; charset=utf-8` (unless overridden by `headers`) — including when its content happens to look like JSON. A body that looks like JSON is not a JSON body; use `json` for that |
| `json` | A literal response body, declared as JSON — served as `application/json` (unless overridden by `headers`). Validated at load time: must parse, and loading fails otherwise (see below) |
| `status` | The HTTP status code |
| `headers` | Custom headers, honoured uniformly on every shape above — see [Response headers](./response-headers.md) |
| `delay_response_milliseconds` | Sleep this long before responding. Overrides the rule set's `[default]` value, including `0` to cancel it |
| `csv_records_key` | For a CSV `file_path`, the dotted path under which the parsed rows are nested in the JSON response (default key: `records`) |

**Content-type is derived from which field is set** — `file_path` from
its extension, `text` always `text/plain; charset=utf-8`, `json`
always `application/json` — and an explicit `headers.content-type`
always overrides that default, on every one of the three.

Validity rules: `file_path`, `text` and `json` are **mutually
exclusive** — exactly one may be set. `file_path` combined with
`status` is rejected — a custom status code is only available with
`text` or `json`. `text`/`json` combined with `status` is allowed (a
custom-status message body). `file_path` must resolve to a file that
exists under the rule set's `respond_dir`/`prefix.respond_dir` — and
if its extension is `.json`/`.json5`, its content must parse as JSON
too. Both checks run at startup (`apimock validate`, and loading a
config to run the server), not per-request: a rule that couldn't be
served either way now fails to load, naming the file and the parse
position, instead of loading and returning `500` on the first request
that reached it. `json`'s own inline value is validated the same way,
naming the rule.

### `priority` and `weight`

Per-rule fields, read only when the governing strategy needs them:
`priority` (integer, for the `priority` strategy) and `weight`
(unsigned integer, default `1`, for `weighted_random`). Both are
ignored under every other strategy.
