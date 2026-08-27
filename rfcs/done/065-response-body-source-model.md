# RFC 065 — The response body-source model: say what you serve

**Status.** Implemented (v6.0.0). Accepted — owner approved 2026-08-27.
**Tracks.** v6 response correctness. **Blocking for 6.0.0.**
**Touches.** `crates/apimock-routing/src/rule_set/rule/respond.rs`,
`crates/apimock-server/src/respond_response.rs`,
`crates/apimock-server/src/response/{json_response,file_response,error_response}.rs`,
`crates/apimock-config` (load-time validation),
`crates/apimock/src/cmd/set.rs`, `docs/src/reference/rule-set-schema.md`.
**Depends on.** [RFC 045](../done/045-configuration-accepted-but-ignored.md),
[RFC 057](./057-set-command.md), [RFC 062](./062-v6-threat-model.md).

## Summary

`apimock set --json` produces a mock that serves the right body under
the wrong `content-type`. Investigating that turned up three more
defects in the same subsystem, all sharing one cause: **`respond` has no
model of what kind of body it is serving.** Content-type is decided
ad hoc at four different points, load-time validation does not cover
response bodies, and one error path echoes an absolute filesystem path
to the client.

This RFC replaces the ad-hoc handling with an explicit **body source**,
derives content-type from it, makes the override rule uniform, moves
body validation to load time, and stops the error path leaking paths.

All four defects measured 2026-08-27 against a binary built from
`main` @ `37c4957`.

## Motivation

### Defect 1 — `--json` serves `text/plain`

```
$ apimock set rule --path /api/user --status 200 --json '{"id":1,"name":"ada"}'
Applied:                                                        [exit 0]
$ curl -D - http://127.0.0.1:3399/api/user
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
{"id":1,"name":"ada"}
```

Body right, header wrong. Nothing looks broken until a client calls
`.json()` under a strict library, or the mock is compared against the
API it imitates.

**This is the headline flow.** `set` is v6's flagship command and
`--json` is the obvious flag for mocking a JSON API. The CLI's primary
user is an agent, defined by failing silently — and this fails silently
at exit 0.

**The cause is a lost fact, not a missing check.** `set --json` *already
validates* its input:

```
$ apimock set rule … --json 'this is not json at all'
apimock set rule: --json is not valid JSON: expected ident at line 1 column 2   [exit 2]
```

It knows the value is JSON, then stores it as `respond.text`
(`respond.rs:19` has no JSON field), and `respond_response.rs:97` hands
`text` to `text_response(text, None, …)` — content type `None`, so
`text/plain`. **The type information exists at the moment of writing and
is discarded by the schema.**

### Defect 2 — an explicit `content-type` is silently overwritten on the JSON path

RFC 045 Defect 1b established the precedence rule — an explicit
`respond.headers.content-type` must win over an inferred default — and
fixed `text_response` accordingly, applying custom headers *after*
`with_text`. `json_response` still applies them *before*
`with_json_body`, so `with_json_body` overwrites them:

| Rule | Declared | Served |
|---|---|---|
| `text` + `headers.content-type = "application/json"` | `application/json` | `application/json` ✅ |
| `file_path = "data.json"` + `headers.content-type = "application/vnd.custom+json"` | `application/vnd.custom+json` | **`application/json`** ❌ |

The operator wrote a header and apimock silently ignored it. Anyone
mocking a vendor media type (`application/vnd.…+json`, JSON:API,
`application/problem+json`) cannot.

RFC 045 fixed one of two symmetric call sites. This is the other.

### Defect 3 — `validate` passes on a malformed JSON response file

```
$ apimock validate -c ./cfg.toml
Validation passed (1 rules across 1 rule set(s)).               [exit 0]
```

The rule serves a `.json` file whose contents are `{"a": ,,,BROKEN`.
`Respond::validate` checks that the file *exists*; nothing parses it.
The failure surfaces at request time as a 500.

`validate` exists to answer "will this config serve correctly". Passing
a config that cannot serve is the failure mode RFC 045 was written
about, on the response side rather than the config side.

### Defect 4 — the 500 body leaks an absolute filesystem path

The same request returns:

```
HTTP/1.1 500 Internal Server Error
content-type: text/plain; charset=utf-8

/home/<user>/…/scratch/badjson.XXXX/bad.json: invalid json content
```

**The absolute path is echoed to the client** — directory layout, and
the username with it. `internal_server_error_response` is called with a
message built from the file path (`json_response.rs:31`).

Security-relevant and in scope for the threat model RFC 062 established:
RFC 062 and RFC 063 confined which files may be *read*; this discloses
where they *are*. Bounded by the `127.0.0.1` default bind and by needing
a malformed response file — but "bounded" is not "fixed", and a diagnostic
belongs in the server log, not in an HTTP body.

### The common cause

`respond` describes a body by *elimination* — `file_path`, else `text`,
else status-only — and each branch decides its own content-type in its
own way:

| Where | Decides |
|---|---|
| `respond_response.rs:97` | `text` → `text_response(…, None, …)` |
| `response/util.rs:22` | file extension → `text/*` families |
| `response/util.rs:43` | file extension → binary types |
| `json_response.rs:29` | `.json` file → `application/json`, overwriting headers |

Four sites, three precedence behaviours, and no place where "what kind
of body is this?" is stated once. Every defect above is a consequence.

## Goals

1. `respond` states its body source explicitly; content-type is
   **derived** from it, in one place.
2. An explicit `respond.headers.content-type` wins **on every path** —
   one rule, no exceptions.
3. `validate` rejects a response body that cannot be served, including
   malformed JSON, inline or referenced.
4. No error response contains a filesystem path.
5. `set --json` produces a mock that serves `application/json`.

## Non-goals

- Content negotiation, or varying the response by `Accept`.
- A general templating or transformation layer.
- Changing how rules *match*. This is entirely the respond side.
- Reworking `csv_records_key`, beyond fitting it into the model.

## Design

### The body source

`respond` declares **exactly one** body source. This is not a new
constraint invented here — `Respond::validate` already rejects
`file_path` + `text`, and `file_path` + `status`. It makes the existing
rule explicit and total.

| Source | Field | Derived content-type |
|---|---|---|
| File | `file_path` | From extension, as today |
| Plain text | `text` | `text/plain; charset=utf-8` |
| **JSON** | **`json`** *(new)* | `application/json` |
| None | `status` only | none set |

`json` is a new `Option<String>` on `Respond`. `Respond` is
`#[non_exhaustive]` (RFC 041) and every field is `Option`, so this is
**purely additive** — no existing config changes meaning, and RFC 039's
additive-only gate is satisfied.

Why a field rather than a `content_type` string: it carries a *type*,
not a *label*. It is what lets load-time validation parse the value
(Goal 3), what lets `set --json` stop discarding what it already
verified (Defect 1), and what makes the wrong content-type
unrepresentable rather than merely discouraged. A bare `content_type`
field would let `text = "{"` + `content_type = "application/json"`
declare JSON that is not JSON — precisely today's bug with extra steps.

### Content-type derivation, in one place

A single function maps body source → default content-type, replacing the
four ad-hoc sites. Then, uniformly and last:

> **An explicit `respond.headers.content-type` overrides the derived
> default. Always, on every source.**

This is RFC 045's rule, applied everywhere rather than on one path.
`json_response` must apply custom headers **after** `with_json_body`,
matching `text_response`.

`headers` remains the escape hatch for anything the model does not name
— vendor media types, `application/problem+json`, a deliberate
`text/plain` for a JSON body. **The model sets the default; the operator
keeps the last word.** Nothing expressible today becomes inexpressible.

### Validation moves to load time

`Respond::validate` gains:

- **Exactly one body source.** `json` + `text`, `json` + `file_path`,
  etc. are all rejected, extending the pairwise checks already there.
- **Inline `json` must parse**, using the same JSON5 parser
  `json_response` uses at request time, so load and serve agree by
  construction rather than by coincidence.
- **A referenced `.json` file must parse.** Today only its existence is
  checked. This is Defect 3.

Validation reads the file at load; that is already true for existence.

> **Deliberate consequence, stated rather than buried:** a config that
> serves today can fail to load after this change, if it references a
> malformed `.json` file. That is the point — it cannot serve that rule
> today either; it 500s. Moving the failure from request time to load
> time is the change. It is a **breaking change for broken configs
> only**, appropriate to a major version, and belongs in the migration
> guide.

### Error responses carry no paths

`internal_server_error_response` gets a client-facing message with no
filesystem path. The path goes to `log::error!` instead, where it is
useful to the operator and invisible to the client.

Audit every `internal_server_error_response` and
`*_response("…{}", path)` call for the same shape — Defect 4 is one
instance and this RFC should close the class, not the instance.

### The CLI

`set --json <value>` writes `respond.json`, not `respond.text`. Its
existing input validation is unchanged — it already rejects non-JSON;
now the verified value reaches a field that preserves what was verified.

`set --text` is unchanged and still writes `respond.text`.

## Compatibility

| Config | Before | After |
|---|---|---|
| `respond.text` with a JSON string | `text/plain` | **`text/plain`** — unchanged. A body that happens to be JSON is not a JSON body |
| `respond.text` + explicit `content-type` | honoured | honoured |
| `.json` `file_path` | `application/json` | unchanged |
| `.json` `file_path` + explicit `content-type` | **ignored** | **honoured** (Defect 2) |
| `.json` `file_path`, malformed | loads; 500s per request | **fails to load** (Defect 3) |
| Rules written by `set --json` before 6.0.0 | `text/plain` | `text/plain` — they are `respond.text`, and stay so until rewritten |

Only the last row is a silent carry-over: configs generated by an
earlier `set --json` keep serving `text/plain`. **Do not auto-migrate
them** — rewriting a user's config on load is a larger and more
surprising action than the bug warrants. The migration guide should say
how to fix them, and `set` overwrites the rule correctly when next run
against it.

## Testing and verification

- `set rule --json '{…}'` → served with `content-type: application/json`.
  **Assert on the served header via a real request**, not on the written
  TOML — the written TOML was right before and wrong after; only the
  wire tells the truth.
- Every body source × {no explicit header, explicit header} — the
  override wins in all cases. A table, one row per combination.
- `.json` file + `application/vnd.custom+json` is served as declared.
- Malformed inline `json` fails `validate`, exit 2, naming the rule.
- Malformed referenced `.json` fails `validate`, exit 2.
- Two body sources on one rule fails `validate`.
- **No error response body contains `/` path-like content** — assert
  across every error path, not just the JSON one.
- The 500 diagnostic still reaches the log with its path.
- Existing rule sets in `examples/` serve byte-identically.
- Full suite, `fmt`, `clippy -D warnings`, `cargo audit`, `mdbook build`.

## Risks

| Risk | Mitigation |
|---|---|
| A previously-loading config now fails to load | Only configs that already could not serve the rule. Deliberate; migration guide entry; the error names the file and the parse position |
| Load-time JSON parsing slows startup | Bounded by the number of referenced `.json` files, parsed once at load rather than on every request — likely a net win |
| Adding a field to `#[non_exhaustive] Respond` | Purely additive; every field is `Option`; RFC 039's gate satisfied |
| The one-body-source rule breaks a config combining fields | Those combinations are already rejected today for `file_path`+`text` and `file_path`+`status`. This completes the set rather than starting it |
| Scope creep into a content-negotiation feature | Explicit non-goal. `headers` stays the escape hatch |

## Unresolved questions

1. ~~**Should `text` with an explicit `application/json` content-type be
   *warned* about at load?**~~ ✅ **Resolved 2026-08-27 — no warning;
   the migration guide covers it.** Owner decision. Building a warning
   would mean building the warning mechanism first: nothing in apimock
   constructs a `Severity::Warning` today, which is why `--strict` has
   nothing to act on (`docs/src/reference/cli-reference.md`). That is a
   larger change than the case warrants, and it would fire on every
   config an earlier `set --json` wrote — configs that are legal, still
   work, and are exactly what the migration guide is for. The prose is
   written and lives in the handoff, to land with the implementation
   rather than describe a field `main` does not yet have.
2. **Does `csv_records_key` become a fifth body source?** It currently
   modifies a `file_path` response rather than replacing it. Fitting it
   in is a non-goal here, but the model should not make it harder to fit
   later — worth a paragraph in the design once implemented.
