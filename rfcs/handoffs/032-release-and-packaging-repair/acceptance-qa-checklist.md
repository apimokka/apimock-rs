# Acceptance / QA Checklist — RFC 032

**Governing RFC.** [RFC 032](../../done/032-release-and-packaging-repair.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Paste actual command output into the review-request package, not a
summary of it.

---

## `version.sh` behaviour

Run all destructive checks on a scratch copy of the tree.

- [ ] `./version.sh --list` still works and reports all four crates
- [ ] `./version.sh --help` still works
- [ ] `--update 9.9.9` sets **root** `Cargo.toml`
      `[workspace.package].version` to `9.9.9`
- [ ] `--update 9.9.9` leaves member manifests untouched
      (`version.workspace = true` preserved verbatim)
- [ ] `--update 9.9.9` sets all three `npm/*/package.json` versions
- [ ] `--update 9.9.9` sets `npm/package.json` `.version`
- [ ] `--update 9.9.9` sets **all three**
      `optionalDependencies["@apimock-rs/bin-*"]` pins
- [ ] `[workspace.dependencies]` version pins in root `Cargo.toml` are
      **unchanged** — confirm `rustls`, `tokio`, `hyper` etc. still hold
      their own versions, not `9.9.9`
- [ ] Round-trip: `--update` back to the original version leaves the
      tree byte-identical (`git diff` empty)

## Self-verification

- [ ] With one target made unwritable or malformed, the script exits
      **non-zero**
- [ ] The failure message **names the file** that could not be updated
- [ ] A successful run exits 0 only after re-reading and confirming
      every target

## Dry-run fidelity

- [ ] `--dry-run --update <ver>` lists exactly the files `--update`
      modifies — no extras, no omissions
- [ ] `--dry-run` modifies nothing (`git status` clean afterwards)

## Repository version state

- [ ] `grep -rn '"version"' npm/` agrees with
      `[workspace.package].version`
- [ ] No `optionalDependencies` pin still reads `4.6.9`
- [ ] No `package.json` still reads `5.7.0`

## Release workflow

- [ ] Pre-publish gate exists and runs **before** any publish job
- [ ] A run with a deliberately mismatched npm version **fails at the
      gate**, not at publish
- [ ] The gate compares all three: git tag, workspace manifest, npm files
- [ ] `permissions` and `secrets` blocks are unchanged
- [ ] Build matrix and published-target set are unchanged
- [ ] **Archive layout is unchanged** — the `tar czf` / `7z a` steps and
      `RELEASE_SRC_DIR` nesting are exactly as before

## Non-change scope

- [ ] No crate source touched
- [ ] No public API touched
- [ ] `npm/index.js` and `npm/postinstall.js` unmodified
- [ ] No dependency added, removed, or upgraded
- [ ] No credential read, printed, echoed, or logged
- [ ] `version.sh` is still POSIX `/bin/sh`

## Documentation

- [ ] `.github/CONTRIBUTING.md` documents the version-bump command
- [ ] No `docs/`, `README.md`, or `CHANGELOG.md` change

## Escalations to report

- [ ] npm registry credential validity — **not verifiable from the
      repo**; flag as an open item for the owner before the first real
      release, so a publish failure is not misdiagnosed as this RFC
- [ ] Anything that would require touching crate source

## Review-request package

- [ ] Created at
      `.git-exclude/review-request/032-release-and-packaging-repair/`
- [ ] Entry-point document orients a reviewer with no prior context
- [ ] Contains all 10 items from § 9.2 of the workflow document
- [ ] Calls out the **self-verification step** and the **section-aware
      root-manifest edit** for focused review
