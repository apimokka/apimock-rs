# RFC 063 — Confine the serve path: a remotely reachable traversal, and a config capability

**Status.** Proposed — awaiting owner approval.
**Tracks.** Security. **Blocking for 6.0.0**, and it raises a question
about **released versions** — see § Disclosure.
**Touches.** `crates/apimock-server/src/dyn_route.rs`,
`crates/apimock-server/src/response/file_response.rs`, the three
`FileResponse` construction sites, `docs/src/reference/threat-model.md`.
**Depends on.** [RFC 062](../accepted/062-v6-threat-model.md), which
confined the **write** path and prompted this by documenting what it did
not cover.

## Summary

`GET /../outside.txt` returns **HTTP 200** with the contents of a file
outside the configured respond directory. Nothing in the serve path
confines a resolved file to the directory it is supposed to be served
from. Fix that, and confine `respond.file_path` with it.

## Motivation

### Finding 1 — remotely reachable path traversal (the serious one)

Reproduced against the built binary. A server with
`fallback_respond_dir = "serve"`, and `outside.txt` one level above it:

```
$ curl --path-as-is 'http://127.0.0.1:3199/../outside.txt'
SECRET-OUTSIDE-CONTENT      [HTTP 200]
```

**The file is outside the served directory and it is returned to an
unauthenticated HTTP client.**

Boundaries, measured rather than assumed:

| Attempt | Result |
|---|---|
| `/../outside.txt`, raw `..` | **200 — content leaked** |
| `/../../outside.txt` | 404 |
| `/%2e%2e/outside.txt` (encoded) | 404 — the encoded form is not decoded into `..` |
| normal `/hello` | 200, correct |

So the reachable form is a **raw `..` segment**, which requires a client
that does not normalise it (`curl --path-as-is` does; many clients and
proxies do not).

**Cause.** `dyn_route` builds the path from the request URL and joins it
onto the respond directory (`dyn_route.rs:126`, `dir.join(…)`), then
checks only `.exists()`. `normalize_url_path`
(`apimock-routing/src/util/http.rs:20`) trims leading and trailing
slashes and applies a prefix — **it does not remove `..` segments**, and
nothing downstream does either. A grep for `canonicalize`, `starts_with`
or any traversal guard across `apimock-server` finds none in the serve
path.

**This is in released code.** `git show 5.19.0:…/dyn_route.rs` has the
same `dir.join` with no guard.

### Finding 2 — `respond.file_path` is unconfined (the one that started this)

Found by the dev team while writing RFC 062's threat-model page, and
reproduced:

```toml
[prefix]
respond_dir = "responses"
[[rules]]
[rules.respond]
file_path = "../../outside.txt"     # served as authored
```

Not remotely reachable — `file_path` values are static,
operator-authored configuration, never built from request input. What it
is: **a config file can serve any file the process can read.**

That matters more in v6 than it did in v5, because **v6 is designed for
machine-generated configs**. "Do not run untrusted configs" is
reasonable guidance for a file a person wrote by hand and much weaker
for a tool whose headline feature is an agent writing them — and
`set --file <path>` is the supported way such a value gets in.

### Both are the same root cause

Neither the request-derived path nor the config-derived path is ever
checked against the directory it is meant to stay inside. RFC 062
confined the **write** path; the **read** path was left open, and that
asymmetry was never a decision — it is just where we happened to look.

### What limits the damage today, honestly

- The default bind is `127.0.0.1`
  (`LISTENER_DEFAULT_IP_ADDRESS`), so the traversal is not reachable off
  the machine unless the operator changes it — which is a supported
  configuration.
- Encoded traversal does not work; only a raw `..`.
- The threat-model page written yesterday already says apimock **should
  not be exposed to an untrusted network**.

**None of that is a fix.** Documentation that says "do not expose this"
does not make an arbitrary-file-read acceptable in a tool people run on
laptops holding credentials, and "bound to localhost by default" is
exactly the reasoning by which these ship.

## Goals

1. A resolved file is served **only** if it stays within the directory
   it was resolved against — for every one of the three `FileResponse`
   construction sites.
2. A traversal attempt is a **404**, not an error that reveals whether
   the target exists.
3. `respond.file_path` is confined the same way, with an opt-out
   consistent with RFC 062's `--allow-outside` if one is wanted at all.
4. Regression tests that fail without the fix.

## Non-goals

- Reworking `normalize_url_path`'s prefix semantics. It can strip `..`
  as part of this, but its existing behaviour stays.
- Authentication, authorisation, or rate limiting. apimock is a
  development tool and this RFC does not change that.
- Confining Rhai's own filesystem access. That is T2 territory and a
  separate question.

## Design

**Canonicalise and compare.** For each of the three sites, resolve the
candidate path, canonicalise it and the base directory, and serve only
if the canonical candidate is inside the canonical base. Otherwise 404.

The three sites (all constructing `FileResponse`):

| Site | Base directory |
|---|---|
| `dyn_route.rs:139` | the fallback respond dir |
| `respond_response.rs:77` | the rule set's `respond_dir` |
| `middleware/middleware_response.rs:63` | the same, for a Rhai-returned path |

**404, not 403.** A distinct error tells a prober that the file exists
outside the root. 404 tells them nothing.

**Symlinks.** Canonicalisation resolves them, so a symlink pointing
outside the base is refused. That is the right default and it may break
someone's deliberate layout — hence § Unresolved 2.

## Testing and verification

- **The exact repro above returns 404**, and a test asserts it —
  `--path-as-is`, raw `..`, against a real running server.
- The encoded form, and `..` in the middle of a path, are covered too.
- Normal file serving is unaffected: the W7 script and every existing
  `dyn_route` test still pass.
- `respond.file_path` pointing outside `respond_dir` is refused.
- A Rhai middleware returning an outside path is refused.
- **Each test fails without the fix.** Apply the tests first, watch them
  fail, then fix — and report both halves, the ritual that has caught a
  vacuous test three times this cycle.

## Risks

| Risk | Mitigation |
|---|---|
| Canonicalisation breaks a legitimate symlinked layout | § Unresolved 2; opt-out if needed |
| `canonicalize` on every request costs latency | Measure it. The base can be canonicalised once at load; only the candidate is per-request |
| Fixing 6.0.0 leaves released versions exposed | § Disclosure — that is the owner's call, not mine to assume |
| A 404-for-everything approach hides real misconfiguration | Log the refusal at debug level with the reason; the response stays a bare 404 |

## Disclosure — the question I cannot answer alone

**This is present in 5.19.0 and, by the look of the code, in every v5
release.** That makes it a vulnerability in shipped software, not only a
6.0.0 defect.

The owner needs to decide:

1. **Does v5 get a patch release** (5.19.1) with the fix backported, or
   does the fix ship only in 6.0.0?
2. **Is a security advisory published** — GitHub Security Advisory,
   RUSTSEC — or is it handled as an ordinary bug-fix release?
3. **Does 6.0.0 wait** for that process, or ship on its own schedule?

My recommendation: **backport to a 5.19.1 and publish a GitHub advisory.**
The bar for "we told people" should not be a changelog line when the
issue is unauthenticated file read. But the severity is genuinely
bounded by the localhost default, and the owner may reasonably weigh
that differently.

## Unresolved questions

1. **Does `respond.file_path` get an opt-out, or is confinement
   absolute?** RFC 062 gave the write path `--allow-outside`. Symmetry
   argues for one; the fact that this is the *serve* path, reachable by
   anything that can send a request, argues against.
2. **Symlinks.** Canonicalisation refuses a symlink escaping the base.
   Is that right, or should a symlink inside the base be followed
   wherever it points? Recommend refusing, and revisiting on evidence.
3. **Is `..` stripped in `normalize_url_path` as well?** Defence in
   depth argues yes. It would also change matching for rules whose
   `url_path` contains `..`, which nothing sensible does.
