# RFC 074 — TLS: bound the handshake, and fail loudly

**Status.** Accepted — owner approved 2026-09-01.
**Tracks.** Availability. External audit 2026-09-01, S-07, S-08.
**Touches.** `crates/apimock-server/src/server.rs`,
`crates/apimock-server/src/tls.rs`, `docs/src/guides/serve-over-https.md`.

## Summary

Two independent TLS problems:

1. **No handshake timeout and no connection cap.** A client that opens a
   connection and never completes the TLS handshake holds resources
   indefinitely; nothing bounds how many may do so.
2. **A TLS setup failure silently degrades the server to HTTP-only.**
   The operator asked for HTTPS, did not get it, and is not told.

## Motivation

**S-08 is the one to fix first**, and it is not really an availability
bug — it is a *trust* bug. An operator configures TLS, the certificate
path is wrong or the PEM will not parse, and the server comes up serving
plain HTTP. Every subsequent request they believe is encrypted is not.

For a development tool that is less catastrophic than in production —
but the whole reason someone enables TLS locally is to reproduce a
TLS-dependent behaviour, and silently not doing it makes the
reproduction meaningless while looking like success.

**S-07** is the ordinary slowloris shape. The audit notes there is also
no connection cap to bound concurrency, so the two compound.

Scope check: `threat-model.md` says apimock is not hardened for hostile
input and should not face an untrusted network, and this RFC does not
pretend otherwise. But a handshake timeout is not hardening — it is
hygiene, and its absence also bites the honest case where a client dies
mid-handshake.

## Goals

1. A TLS setup failure **prevents startup** with a clear error, rather
   than degrading to HTTP.
2. An incomplete handshake is dropped after a bounded time.
3. Concurrent connections are bounded by something.

## Non-goals

- Hardening against a determined attacker. Out of stated scope.
- Changing certificate loading, reload, or the resolver
  (`ReloadableCertResolver` is sound and well-tested).

## Design

**S-08** — treat a TLS configuration error as fatal at startup. If
`[listener.tls]` is present and cannot be honoured, exit with a
`config_invalid`-shaped error naming the file and the parse failure.

> **The silent-degradation path must not survive behind a flag.** If an
> operator genuinely wants "HTTPS if possible, HTTP otherwise", that is
> a distinct, explicit setting — not a failure mode. Recommend not
> adding it until someone asks.

**S-07** — a handshake timeout (a few seconds is generous locally) and a
maximum concurrent-connection count, both configurable, both defaulting
to values no ordinary development use will reach.

## Testing and verification

- A malformed PEM: server **exits**, message names the file. Assert it
  did not bind an HTTP listener.
- A missing certificate file: same.
- A valid TLS config: unchanged, and the existing TLS tests pass.
- A connection that opens and sends nothing is dropped after the
  timeout, **and the server serves other requests throughout** — the
  second half is the finding.
- The connection cap is reached and the server recovers once connections
  close.

## Risks

| Risk | Mitigation |
|---|---|
| Failing startup breaks someone relying on the fallback | They are relying on being silently unencrypted. Named in the release note as a fix, not a regression |
| A timeout too short breaks a slow client | Configurable, and defaulted generously; local handshakes complete in milliseconds |
| A connection cap surprises a load test | Configurable; and a mock server that silently accepts unbounded connections is not a better load-test target |

## Unresolved questions

1. **What is the right default connection cap?** Too low breaks
   legitimate parallel test suites; too high is no bound at all.
   Establish from a realistic parallel-test workload rather than
   picking a round number.
