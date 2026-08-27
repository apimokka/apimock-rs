# Acceptance / QA Checklist — RFC 065

**Governing RFC.** [RFC 065](../../accepted/065-response-body-source-model.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Every row is a claim a reviewer will re-run. Tick only what you ran.

---

## 0. How to test this correctly

> **Assert on the wire, not on the file.** Every defect here is a
> mismatch between what the config *says* and what the server *sends*.
> The written TOML was already correct before this RFC and wrong after
> the bug — only a real HTTP response tells the truth.
>
> Start a server, `curl -D -`, assert on the header. A test that reads
> the rule set and checks `json = …` proves nothing about D1.

- [ ] Response assertions come from a real request against a running
      server, not from parsing the config.
- [ ] Exit codes captured directly, never through a pipe (`… | head`
      reports `head`'s status).

## 1. D1 — `--json` serves `application/json`

- [ ] `set rule --path /api/user --status 200 --json '{"id":1}'` then
      `GET /api/user` → **`content-type: application/json`**
- [ ] The body is byte-identical to what was declared
- [ ] The rule is written as `respond.json`, not `respond.text`
- [ ] `set --text` still writes `respond.text` and still serves
      `text/plain; charset=utf-8`
- [ ] `set --json` still rejects non-JSON, message unchanged

## 2. D2 — the override rule, uniformly

A table, one row per **(body source × header present/absent)**. Not
hand-picked examples.

- [ ] `file_path` (`.json`) + `content-type: application/vnd.custom+json`
      → **served as declared** (this is the defect)
- [ ] `file_path` (`.json`), no explicit header → `application/json`
- [ ] `file_path` (`.html`, `.css`, `.png`) → unchanged from today
- [ ] `text`, no header → `text/plain; charset=utf-8`
- [ ] `text` + explicit header → served as declared (already true; pin it)
- [ ] `json`, no header → `application/json`
- [ ] `json` + explicit header → **served as declared**
- [ ] `status`-only + explicit header → served as declared
- [ ] A deliberate inversion works: `json` + `content-type: text/plain`
      serves `text/plain`. The operator keeps the last word

## 3. D3 — validation at load

- [ ] Malformed **inline** `json` → `validate` exit 2, naming the rule
- [ ] Malformed **referenced** `.json` file → `validate` exit 2, naming
      the file **and the parse position**
- [ ] The server also refuses to start on both
- [ ] Valid JSON in both forms still loads and serves
- [ ] The same JSON5 parser is used at load and at request time — a
      value accepted by one is accepted by the other. **State how you
      established this**, since "they agree" is the whole point
- [ ] Two body sources on one rule (`json`+`text`, `json`+`file_path`,
      and the pre-existing `file_path`+`text`, `file_path`+`status`)
      → all rejected at load

## 4. D4 — no host detail in any client-facing error

- [ ] The malformed-`.json` 500 no longer exists (D3 stops it loading) —
      **and** any remaining error path is checked
- [ ] **Sweep every error response construction**, not just the JSON
      one. List what you found in the review package
- [ ] No error response body contains an absolute path, a username, or a
      directory listing. Assert mechanically — e.g. the body contains no
      `/`-prefixed path-like substring — rather than eyeballing
- [ ] The diagnostic **still reaches the server log**, with its path
      intact. The information moves; it is not lost

## 5. Non-regression

- [ ] Every rule set under `examples/` serves **byte-identical**
      responses, headers included, before and after
- [ ] Every existing test passes; note any you had to change and why
- [ ] `csv_records_key` responses unchanged
- [ ] Middleware response paths (`middleware_response.rs`) unchanged
- [ ] A config with `respond.text` containing JSON still serves
      `text/plain` — this is deliberate, pin it so it cannot drift
- [ ] Startup time on a config with many `.json` files is not
      meaningfully worse (parsing moved to load; report the measurement)

## 6. Documentation

- [ ] `docs/src/reference/rule-set-schema.md`: `json` documented
      alongside `file_path`/`text`; the one-body-source rule stated; the
      content-type derivation table given; the override rule stated once
- [ ] `docs/src/guides/migrating-to-6-0.md`: the section supplied in the
      handoff's § 5, placed and fitted to the page
- [ ] No other page still implies `--json` yields a `text/plain` body —
      grep the docs for `--json`
- [ ] `mdbook build docs` clean

## 7. Gates

- [ ] `cargo test --workspace`, count reported
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo audit`
- [ ] CI green on all 8 jobs before merge

## 8. Report back

Per the handoff's § 7 — in particular the served content-type table
captured from real requests, the D4 sweep results, and whether
consolidating content-type derivation changed any existing response.
