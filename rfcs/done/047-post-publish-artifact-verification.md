# RFC 047 — Verify what was actually published

**Status.** Implemented (v5.17.0). Approved by the project owner
2026-08-12; reviewed in `REVIEW-001` / `REVIEW-002`. The jobs themselves
first execute during v5.17.0's own release — knowingly, as RFC 044's
publish phase did before them.
**Tracks.** Pipeline trust. A green publish job proves an upload
succeeded. It does not prove the right bytes arrived, and this project
has already shipped fifteen months of npm packages containing the wrong
binary without noticing.
**Touches.** `.github/workflows/release-publish.yaml` (one new job),
`RELEASING.md`. **No crate source, no public API.**

## Summary

Add a final job to the publish phase that pulls each published artifact
back down from the registry and asserts it matches what the GitHub
Release advertises — for npm, that the binary inside the published
tarball is byte-identical to the binary inside the release asset.

## Motivation

### The failure this catches has already happened here

npm published `@apimock-rs/*` packages whose version fields read 5.9.0
through 5.10.0 while the binaries inside were **4.6.9**. That went
undetected across multiple releases. Every CI job involved was green,
because every job that ran did succeed — none of them looked at what
landed.

### It happened again in the sense that only a manual check found the truth

Verifying v5.16.0 on 2026-08-12 meant doing this by hand:

```
npm tarball binary : cfbb5389db8c7f8f9d3cadcd3361af7efc5e0c0e5ba7f57cdc2cf0b9ac9f847e
release asset binary: cfbb5389db8c7f8f9d3cadcd3361af7efc5e0c0e5ba7f57cdc2cf0b9ac9f847e
```

That is the check that matters, and it is currently a thing a person
remembers to do. The version field agreeing is *not* the check — it was
agreeing throughout the 4.6.9 period.

### It is cheap

The comparison is a download and a `sha256sum` per platform. The inputs
already exist: the release assets are attached to the Release, and the
published packages are public. Nothing new has to be built or stored.

## Goals

1. After publishing, assert that each published npm platform package
   contains a binary byte-identical to the corresponding GitHub Release
   asset's binary.
2. Assert that each of the four crates resolves on crates.io at the
   released version.
3. Fail loudly, naming the artifact and both hashes, when they differ.
4. Add nothing to the critical path of publishing itself — this runs
   after, and its failure means "investigate", not "the release is
   broken" (by then it is published either way).

## Non-goals

- Blocking or gating the publish. By the time an artifact is on a
  registry it cannot be withdrawn; this job exists to tell a human
  immediately rather than fifteen months later.
- Verifying provenance signatures. Worth considering later; it is a
  different mechanism and would widen this to a security review.
- Verifying the npm **core** package's behaviour (that it installs the
  right platform package). That is an install-time integration test and
  a larger piece of work — noted under Unresolved questions.

## Proposed design

A `verify-published` job in `release-publish.yaml`, `needs:` the publish
jobs, matrixed over the same three npm platform targets.

For each platform:

1. `npm pack @apimock-rs/bin-<target>@<version>` — public, unauthenticated.
2. `gh release download <tag> --pattern '<matching asset>'` — read-only.
3. Extract both; `sha256sum` the binary from each; compare.
4. On mismatch, fail with both hashes and both paths in the message.

Then, once:

5. For each of `apimock`, `apimock-config`, `apimock-routing`,
   `apimock-server`, assert crates.io serves the released version — the
   registry API is public, so this needs no credential.

**Note the propagation caveat:** registries are not instantly consistent.
The job should retry with a short backoff before declaring a package
missing, and the review request should state what window was chosen and
why. A false failure here trains people to ignore the job, which would
defeat its purpose entirely.

## Testing and verification

- Run the comparison logic locally against the **already-published
  v5.16.0** artifacts, which are known-good — it must pass.
- Construct a negative case: compare a v5.16.0 npm binary against a
  v5.15.0 release asset and show the job fails with both hashes named.
  A verification job that has never been seen to fail is not evidence.
- Confirm every command used is read-only and unauthenticated, so the
  job needs no additional permission beyond `contents: read`.

## Risks

| Risk | Mitigation |
|---|---|
| Registry propagation delay causes false failures | Retry with backoff; state the chosen window in the review request |
| The job becomes noise and gets ignored | It must be silent when correct and specific when not — no warnings, no partial passes |
| Scope drift into full provenance verification | Explicitly a non-goal here |

## Unresolved questions

1. Should this also verify that `npm install apimock-rs` on each platform
   resolves and runs the right binary end to end? That is the check
   closest to what a user experiences, and the most work. Deliberately
   left out of this RFC; raise separately if wanted.
2. Should a mismatch open an issue automatically, or only fail the job?
   Failing is enough while releases are infrequent and watched.
