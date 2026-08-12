# Implementation Handoff — RFC 047, Verify what was actually published

**Governing RFC.** [RFC 047](../../proposed/047-post-publish-artifact-verification.md)
**Milestone.** M3 — P1. Independent of RFC 046; the two can run in parallel.
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

Add a `verify-published` job to
`.github/workflows/release-publish.yaml` that downloads what was just
published and checks it is what the GitHub Release advertises.

The version field agreeing is **not** the check. npm published packages
labelled 5.9.0–5.10.0 containing **4.6.9** binaries, undetected across
several releases, with every CI job green throughout. The check is the
bytes.

## 2. The comparison

For each of the three npm platform packages:

1. `npm pack @apimock-rs/bin-<target>@<version>` — public, no auth.
2. `gh release download <tag> --pattern '<matching asset>'` — read-only.
3. Extract both; `sha256sum` the binary from each; compare.
4. On mismatch: fail, printing **both hashes and both paths**.

Then once, for `apimock`, `apimock-config`, `apimock-routing`,
`apimock-server`: assert crates.io serves the released version. The
registry API is public — no credential.

The extraction logic already exists in this same workflow's
`npm-platforms-publish` job. Reuse its archive handling rather than
writing a second, divergent copy.

## 3. Constraints

- **`needs:` the publish jobs.** This runs after, never gates.
- **`contents: read` only.** Every command must be read-only and
  unauthenticated. If you find yourself needing a credential, stop —
  something is wrong with the approach.
- **Do not touch the `on:` block.** `release: [published]` is the sole
  trigger and that is a structural guarantee, not a default. See RFC 044.
- **Handle propagation delay.** Registries are not instantly consistent.
  Retry with a short backoff before declaring anything missing, and state
  in the review request what window you chose and why. A job that cries
  wolf gets ignored, which defeats the entire purpose.

## 4. Evidence required

- **Positive:** run the comparison locally against the already-published
  **v5.16.0** artifacts, which are known good. It must pass. For
  reference, the Linux x64 gnu binary hashes to
  `cfbb5389db8c7f8f9d3cadcd3361af7efc5e0c0e5ba7f57cdc2cf0b9ac9f847e` on
  both sides — but recompute it, don't take it from here.
- **Negative:** compare a v5.16.0 npm binary against a **v5.15.0**
  release asset and show the job fails with both hashes named. *A
  verification job that has never been observed to fail is not evidence
  that it works.*
- Confirm no step needs more than `contents: read`.

## 5. Out of scope

- Gating or blocking the publish.
- Provenance signature verification — different mechanism, would pull in
  a security review.
- End-to-end `npm install apimock-rs` platform resolution. It is the
  check closest to a user's experience and it is deliberately deferred;
  see RFC 047 Unresolved question 1.

## 6. Escalation

Blocking issues or design questions go in a
`.git-exclude/review-request/` package, per project convention.
