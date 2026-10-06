# Migrating to 6.3.0

**Filename and version number are a placeholder** — no release number
has been decided for this cycle yet; RFC 066 § 2 keeps that decision
outside this page's author entirely (versions, tags, and publishing are
never touched without explicit instruction). `6.3.0` is written here
only as "the next minor after 6.2.0", which is already tagged. Rename
this file and its `SUMMARY.md` entry to match whatever the release
process actually settles on.

One RFC lands here: **082**, which adds PATCH to the methods a rule can
match, and explains the refusal for the methods that stay unmatchable.

| RFC | What breaks |
|---|---|
| [082](#http-method-is-now-non-exhaustive-and-patch-is-matchable) | *Library consumers of `apimock_routing::HttpMethod` only:* an exhaustive `match` on `HttpMethod` needs a `_` arm |

## HTTP method is now non-exhaustive, and PATCH is matchable

**RFC 082.** A rule's `when.request.method` accepted only `GET`, `POST`,
`PUT` and `DELETE`, so a `PATCH` rule was refused at load even though
PATCH is in common REST use. It now matches:

```toml
[[rules]]
when.request.url_path = "/users/42"
when.request.method = "PATCH"
respond = { text = "patched" }
```

Two things came with it:

- **`access-control-allow-methods` now includes `PATCH`.** Without that, a
  browser's CORS preflight refuses the PATCH you just configured.
  `response-headers.md` shows the new value.
- **A refused method now says why, when there is a reason.** A method
  that is deliberately not matchable — `OPTIONS`, `HEAD`, `TRACE`,
  `CONNECT` — is refused with the valid set and the reason, instead of the
  set alone:

```
unknown variant `OPTIONS`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`
  — OPTIONS is answered by the built-in CORS preflight handler before rule sets are consulted, so it cannot be matched by a rule
```

A typo still gets the set alone, with the list now including `PATCH`.

**If you match `HttpMethod` exhaustively in Rust**, the enum is now
`#[non_exhaustive]`, so the compiler needs a wildcard arm. Add one
arm — `_ => …` — and the next method added to the enum will not break
your code. This is the same one-time change as `Outcome` in 6.2.0, and
for the same reason. It applies to `apimock_routing::HttpMethod` only;
`Outcome` is unaffected by this release.

**If you rely on the text of the `set --method` refusal**, it now lists
`PATCH` and carries the same reason clause as the config refusal.

## What isn't changing

- **Config spelling stays case-sensitive.** `method = "patch"` in TOML is
  still refused. A request's method on the wire still matches
  case-insensitively, so a client sending `patch` still matches a `PATCH`
  rule.
- **`HEAD` and `OPTIONS` are still not matchable.** Each has a reason in
  the refusal, and the [rule-set schema](../reference/rule-set-schema.md#which-methods-a-rule-can-match-and-why-not-all-of-them)
  explains why.
- **The `set` CLI still accepts any casing of `--method`.** It uppercases
  the value before reading it, as it always has.
