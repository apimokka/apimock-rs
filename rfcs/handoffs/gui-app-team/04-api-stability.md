# 4. API stability — what we promise, and what enforces it

## The promise

**Within 6.x, the public API is additive-only.** A signature that
compiles against 6.0.0 will compile against every later 6.x.

That is not a good intention. As of 6.0.0 it is **mechanically
enforced**.

## The gate (RFC 039)

Each crate has a checked-in baseline of its complete public API:

```
crates/apimock/public-api.txt            166 lines
crates/apimock-config/public-api.txt    1868 lines
crates/apimock-routing/public-api.txt   1444 lines
crates/apimock-server/public-api.txt     803 lines
```

CI regenerates these with `cargo-public-api` and **fails if they differ
from the checked-in file.** Nothing auto-updates: a commit that changes
the API must contain the baseline change, which puts the API diff in
front of a reviewer in the pull request.

**Additive changes fail too**, until the baseline is declared. That is
deliberate — additive-only means *declared*, not *unchecked*.

**Two consequences for you, both good:**

1. **`git log crates/<name>/public-api.txt` is the API's changelog.**
   Not a hand-maintained list that drifts — the actual surface, with
   the commit that changed it and its message.
2. **These files are the answer to "what can I call?"** They are
   generated from the crates and gated, so they cannot be stale. This
   handoff quotes them; if the two disagree, believe the baseline.

The baseline was generated at the 6.0.0 tag with zero source changes
since it, so it *provably* is the released surface.

## `#[non_exhaustive]` — what you cannot do (RFC 052)

Several public types are `#[non_exhaustive]`. From outside the defining
crate this means:

- **No struct-literal construction** — including `..Default::default()`.
- **No exhaustive matching** — your `match` needs a `_` arm.

Field access by name still works everywhere. Only literal construction
and exhaustive matching are affected.

Affected types include `TraceConfig`, `RequestSummary`, `ParsedRequest`,
`LogConfig`, `VerboseConfig`, `Prefix`, and most of
`apimock_server::control` and the error enums. The complete, current
list is in the baselines — grep for `#[non_exhaustive]`.

**Where a constructor was needed, one exists:**

- `ParsedRequest::new(url_path, component_parts)` — then
  `.with_body(body_json, body_len)` to attach one.
- `VerboseConfig::new(header, body)` — a `const fn`, so it works in a
  `const` initializer.
- `TraceConfig::default()`, `ServerControl::new()`, and so on.

**If you need to construct a `#[non_exhaustive]` type and there is no
constructor, that is a gap — tell us.** RFC 052 added constructors
where a cross-crate caller actually existed; you are a cross-crate
caller who did not exist yet. `docs/src/guides/migrating-to-6-0.md`
has the full reasoning.

## Errors (RFC 041)

Error variants are **boxed**, and the enums are `#[non_exhaustive]`.
You will be matching with a `_` arm; that is intended, and it is what
lets us add a variant without breaking you.

The error types you will meet: `WorkspaceError`, `ApplyError`,
`SaveError`, `ConfigError` (from `apimock-config`); `ServerError` /
`ServerErrorKind` / `TlsKind` (from `apimock-server`).

`SaveError::Conflict` is the one with real semantics for a GUI — see
[`02-editing-configuration.md`](./02-editing-configuration.md).

## Semver, concretely

- **6.x** — additive only, gate-enforced. Upgrade freely.
- **7.0** — may break. There is already one known candidate; see
  [`06-known-gaps.md`](./06-known-gaps.md) § 1.
- The four crates are **always published together at the same
  version.** `version.sh` bumps every manifest in lockstep and CI
  asserts it. Do not mix 6.0.0 of one with 6.1.0 of another; the
  internal pins are exact and will not let you anyway.

## MSRV

`rust-version = "1.91.0"`, asserted by a CI job. Note the API baseline
job runs on a **pinned nightly** — that is an inspection tool only and
says nothing about what you need to compile. **1.91.0 is the number
that binds you.**
