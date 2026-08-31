# RFC 067 — CORS: stop reflecting any origin with credentials

**Status.** Proposed — awaiting owner approval.
**Tracks.** Security. **Highest-ranked finding of the 2026-09-01
external audit** (S-01, D-04).
**Touches.** `crates/apimock-server/src/response_handler.rs`,
`crates/apimock-config` (a new setting),
`docs/src/reference/threat-model.md`,
`docs/src/reference/response-headers.md`.

## Summary

When a request carries `Cookie` or `Authorization`, apimock reflects its
`Origin` header verbatim into `Access-Control-Allow-Origin` **and** sets
`Access-Control-Allow-Credentials: true`. Any origin, no allowlist, not
configurable.

That is the textbook CORS misconfiguration, and it is the one allowance
`threat-model.md` never analyses.

## Motivation

**Verified 2026-09-01** against a running server:

```
$ curl -H 'Origin: https://evil.example' -H 'Cookie: session=abc' …
access-control-allow-credentials: true
access-control-allow-origin: https://evil.example
```

Identical with `Authorization:`. Without either header the response is
the safe `Access-Control-Allow-Origin: *` with no credentials — the gate
is `is_likely_authenticated_request` (`response_handler.rs:274`), which
checks exactly those two headers.

**Binding to `127.0.0.1` is not a mitigation.** The dangerous request
originates from the developer's own browser, on a page they merely
visited, and targets their own loopback listener. Our default bind
protects against a remote attacker reaching the port; it does nothing
here. A developer with apimock running who opens any web page has every
mock response readable cross-origin, with whatever cookies their browser
holds for `127.0.0.1`.

**Why this matters more than its severity suggests.** The behaviour
appears to be a deliberate convenience — reflecting the origin is what
makes credentialed `fetch()` from a front-end under development work at
all. It is not a bug in the sense of an oversight. But it is undocumented
as a trade-off, and `threat-model.md`'s stated purpose is to enumerate
"what apimock allows on purpose and why". This is allowed on purpose and
the why was never written down.

## Goals

1. Credentialed reflection happens only for origins the operator named.
2. The default is safe without configuration.
3. The common development case — a front-end on `localhost:5173` calling
   a mock on `localhost:3001` — still works without ceremony.
4. `threat-model.md` gains the subsection it is missing.

## Non-goals

- A full CORS policy engine. Preflight handling, `Allow-Methods` and
  `Allow-Headers` stay as they are.
- Changing the non-credentialed path. `*` without credentials is
  correct and stays.

## Design

**A new setting**, `[service].cors_allow_credentials_origins`, a list of
exact origin strings. Empty by default.

| Request | Today | After |
|---|---|---|
| No `Cookie`/`Authorization` | `ACAO: *`, no credentials | unchanged |
| Credentialed, origin **in** the list | reflect + credentials | reflect + credentials |
| Credentialed, origin **not** in the list | reflect + credentials | **`ACAO: *`, no credentials** |

The unlisted credentialed case degrades to the non-credentialed
behaviour rather than erroring: the response is still served, the
browser simply refuses to expose it to a credentialed cross-origin read.
That is the correct failure — it is the browser's decision to enforce,
and erroring would break the many requests that send a `Cookie` header
incidentally and do not need CORS at all.

**`Vary: Origin` must still be sent** whenever the response depends on
the request's `Origin`, or a shared cache will serve one origin's
response to another.

### The convenience question, answered explicitly

Requiring configuration for the common case would be a regression in the
zero-config experience this project leads with. **Recommend: treat
loopback origins as implicitly allowed** — `http://localhost:*` and
`http://127.0.0.1:*` — since a page served from the developer's own
machine is already inside the trust boundary the loopback bind assumes.
Everything else must be named.

This keeps "front-end on :5173, mock on :3001" working untouched, and
closes the case that actually matters: a *remote* page reading loopback
responses with credentials.

**Owner decision:** if implicit loopback is unwanted, the alternative is
an empty default plus a documented one-line config. Safer, less
convenient, and it will surprise people mid-project.

## Testing and verification

- The four rows of the table above, asserted against a live server.
- `Vary: Origin` present whenever the origin is reflected.
- A loopback origin works with credentials and **no** configuration.
- A non-loopback origin is refused credentials until listed, then works.
- Existing CORS tests still pass; `response-headers.md`'s "always
  present" table stays accurate.

## Risks

| Risk | Mitigation |
|---|---|
| Breaks a working credentialed setup from a non-loopback origin | It does, deliberately — and the fix is one config line, named in the release note. This is the break the RFC exists to make |
| Implicit loopback is itself a hole | Only for an attacker who can already serve from the developer's own machine, which the loopback bind already assumes cannot happen |
| Setting name churn | Name it once, here, and document it in the same change |

## Unresolved questions

1. **Should the setting accept patterns** (`https://*.example.com`) or
   exact origins only? Exact is safer and simpler; patterns are what
   people will ask for. **Recommend exact-only now**, since widening
   later is additive and narrowing is not.
