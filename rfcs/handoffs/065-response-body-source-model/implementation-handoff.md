# Implementation Handoff — RFC 065, the response body-source model

**Governing RFC.** [RFC 065](../../accepted/065-response-body-source-model.md)
**Companion.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0, **blocking**.
**Baseline.** `main` @ `bfd01db`.

**Self-contained.** Everything binding is restated here. You do not need
to read RFC 045, 057 or 062 to do this work.

---

## 1. Four defects, one cause

All measured against a binary built from `main` @ `37c4957`. Reproduce
each before changing anything — they are the acceptance criteria.

### D1 — `--json` serves `text/plain`

```
$ apimock set rule --path /api/user --status 200 --json '{"id":1,"name":"ada"}'
Applied:                                                     [exit 0]
$ curl -D - http://127.0.0.1:PORT/api/user
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
{"id":1,"name":"ada"}
```

Body right, header wrong.

**The cause is a discarded fact, not a missing check.** `set --json`
already validates:

```
$ apimock set rule … --json 'this is not json at all'
apimock set rule: --json is not valid JSON: expected ident at line 1 column 2   [exit 2]
```

It proves the value is JSON, then writes it to `respond.text`
(`rule_set/rule/respond.rs` has no JSON field), and
`respond_response.rs:97` calls `text_response(text, None, …)` — content
type `None` → `text/plain`.

### D2 — an explicit `content-type` is overwritten on the JSON path

| Rule | Declared | Served |
|---|---|---|
| `text` + `headers.content-type = "application/json"` | `application/json` | `application/json` ✅ |
| `file_path = "data.json"` + `headers.content-type = "application/vnd.custom+json"` | `application/vnd.custom+json` | **`application/json`** ❌ |

`text_response` applies custom headers **after** `with_text` (RFC 045
fixed that). `json_response.rs:22-29` applies them **before**
`with_json_body`, which then overwrites. Same bug, the other call site.

### D3 — `validate` passes on a malformed JSON response file

A rule serving a `.json` file containing `{"a": ,,,BROKEN`:

```
$ apimock validate -c ./cfg.toml
Validation passed (1 rules across 1 rule set(s)).            [exit 0]
```

`Respond::validate` checks the file **exists**; nothing parses it.

### D4 — the 500 body leaks an absolute filesystem path

The same request:

```
HTTP/1.1 500 Internal Server Error
/home/<user>/…/bad.json: invalid json content
```

The client is told the server's directory layout and the username.
Built at `json_response.rs:31`, passed to
`internal_server_error_response`.

**Treat D4 as a class, not a call site.** Audit every
`internal_server_error_response` / error-response construction for a
path or other host detail in the client-facing message.

### The cause

`respond` describes a body by elimination — `file_path`, else `text`,
else status-only — and four sites each decide content-type their own
way (`respond_response.rs:97`, `response/util.rs:22`,
`response/util.rs:43`, `json_response.rs:29`), with three different
precedence behaviours. Every defect above follows from that.

## 2. The model to build

`respond` declares **exactly one body source**:

| Source | Field | Derived content-type |
|---|---|---|
| File | `file_path` | from extension, as today |
| Plain text | `text` | `text/plain; charset=utf-8` |
| **JSON** | **`json`** *(new `Option<String>`)* | `application/json` |
| None | `status` only | none set |

Then, **uniformly and last**:

> **An explicit `respond.headers.content-type` overrides the derived
> default. Always, on every source.**

That is RFC 045's rule applied everywhere instead of on one path. Fix
`json_response` to apply custom headers **after** `with_json_body`.

Put the source → default-content-type mapping in **one function** and
route all four sites through it. If you finish and content-type is still
decided in more than one place, the RFC's goal is not met.

### Why a `json` field and not a `content_type` string

Do not substitute a general `content_type` field. It would allow
`text = "{"` + `content_type = "application/json"` — JSON that is not
JSON, which is today's bug with extra steps. A field carries the *type*,
so load-time validation can parse it and the wrong content-type becomes
unrepresentable. `headers` remains the escape hatch for anything the
model does not name.

`Respond` is `#[non_exhaustive]` and every field is `Option`, so adding
`json` is **purely additive**.

## 3. Validation moves to load time

`Respond::validate` gains:

- **Exactly one body source.** `json`+`text`, `json`+`file_path`, etc.
  all rejected. Extends the pairwise checks already there
  (`file_path`+`text`, `file_path`+`status`).
- **Inline `json` must parse** — use the **same JSON5 parser**
  `json_response` uses at request time (`json5::from_str::<Value>`), so
  load and serve agree by construction, not coincidence.
- **A referenced `.json` file must parse** (D3). Today only existence is
  checked.

> **Deliberate break, approved:** a config referencing a malformed
> `.json` file will now **fail to load** where it previously loaded and
> 500'd per request. It could not serve that rule either way; the change
> is when it says so. The error must name the file **and the parse
> position** — this is the migration's sharpest edge, so make the
> message good.

## 4. The CLI

`set --json <value>` writes `respond.json`, not `respond.text`. Its
input validation is unchanged — it already rejects non-JSON. `set
--text` is unchanged.

## 5. Migration guide — prose supplied, place it

Owner resolved RFC 065's open question: **no load-time warning** for
`text` carrying a JSON content-type. Building one would mean building
the warning mechanism first (nothing constructs a `Severity::Warning`
today, which is why `--strict` has nothing to act on), and it would fire
on every config an earlier `set --json` wrote — legal configs that still
work.

Add to `docs/src/guides/migrating-to-6-0.md`. Adjust wording to fit the
page, but keep every fact:

> ### `respond.json`, and rules written by an earlier `apimock set --json`
>
> `respond` now names what kind of body it serves. Alongside
> `file_path` and `text` there is **`json`**, and a rule that uses it is
> served as `application/json`:
>
> ```toml
> [rules.respond]
> json = '{"id":1,"name":"ada"}'
> ```
>
> A rule declares **exactly one** of `file_path`, `text` and `json`.
> Content-type is derived from that choice, and an explicit
> `respond.headers.content-type` still overrides it — on every one of
> them, which was not previously true for `.json` files.
>
> **`text` is unchanged and stays `text/plain`,** including when its
> content happens to be JSON. That is deliberate: a body that looks like
> JSON is not a JSON body.
>
> **This matters if you used `apimock set --json` before 6.0.0.** It
> wrote `respond.text`, so those rules serve `text/plain; charset=utf-8`
> — the body is correct, the header is not, and a client calling
> `.json()` under a strict library may reject it. **Existing configs are
> not rewritten automatically**, because silently editing your config on
> load is more surprising than the problem it fixes. To fix a rule,
> either rename the field:
>
> ```toml
> # before                        # after
> [rules.respond]                 [rules.respond]
> text = '{"id":1}'               json = '{"id":1}'
> ```
>
> or re-run `apimock set --json` against it, which now writes `json`.
>
> **Also new:** a rule serving a `.json` file whose contents are not
> valid JSON now **fails `apimock validate` and fails to load**, instead
> of loading and returning `500` on every request. If a config that
> worked before now refuses to load, this is the likely reason — the
> error names the file and the position. Such a rule could never serve;
> apimock now says so at load time rather than per request.

## 6. Not in scope

- Content negotiation, or varying by `Accept`.
- Templating or body transformation.
- Anything on the matching side. This is entirely the respond side.
- Reworking `csv_records_key` beyond leaving room for it (RFC 065
  Unresolved 2) — it modifies a `file_path` response today; do not make
  it harder to model later, but do not model it now.
- Auto-migrating existing `respond.text` rules.

## 7. Report back

`.git-exclude/review-request/065-response-body-source-model/`, entry
point document, including:

- [ ] **The served `content-type` for each body source**, captured from
      a real request — not from the written TOML.
- [ ] Every place you found a path or host detail in a client-facing
      error message (D4 as a class), and what you did about each.
- [ ] Whether consolidating content-type derivation changed any
      *existing* served response. If it did, say which and why.
- [ ] Anything in § 2 you found reason to disagree with — say so rather
      than implementing around it.
