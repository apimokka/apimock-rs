# Acceptance / QA Checklist — RFC 047

**Governing RFC.** [RFC 047](../../done/047-post-publish-artifact-verification.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## The check is the bytes

- [ ] npm platform packages compared **by SHA-256 of the binary**,
      against the corresponding GitHub Release asset's binary
      *(this originally said `sha256sum`; REVIEW-001 § 3 required a
      portable command, since macOS ships no GNU coreutils — shipped as
      `openssl dgst -sha256`)*
- [ ] Version fields are **not** treated as the check
- [ ] Mismatch output names **both hashes and both paths**
- [ ] All four crates asserted present on crates.io at the released
      version

## Evidence — both directions

- [ ] **Positive:** passes against the known-good published v5.16.0
      artifacts, with hashes **recomputed**, not copied from the handoff
- [ ] **Negative:** v5.16.0 npm binary vs v5.15.0 release asset **fails**,
      with both hashes named
- [ ] The negative case is shown, not asserted

## Structure and permissions

- [ ] New job `needs:` the publish jobs — never gates them
- [ ] `contents: read` suffices; **no credential used anywhere**
- [ ] `on:` block **unchanged** — `release: [published]` remains the only
      trigger (RFC 044)
- [ ] Archive handling **reuses** `npm-platforms-publish`'s logic rather
      than duplicating it

## Propagation

- [ ] Retry with backoff before declaring an artifact missing
- [ ] The chosen window, and the reason for it, stated in the review
      request

## Scope held

- [ ] No provenance-signature verification
- [ ] No end-to-end `npm install` resolution test
- [ ] Publish path itself unchanged
