# RFC 032 — Release and packaging repair

**Status.** Implemented (v5.15.0)
**Tracks.** M1 (Pipeline trust). The npm distribution path — the one
the README advertises first — cannot work as currently configured, and
the tool meant to prevent that (`version.sh`) has been a silent no-op
since the 5.1.1 workspace split.
**Touches.** `version.sh`, `npm/package.json`, `npm/*/package.json`,
`.github/workflows/release-executable.yaml`, release-deliverable
documentation. No crate source, no public API, no runtime behaviour.

## Summary

Repair the cross-ecosystem release path: make `version.sh --update`
actually update every version it claims to, including the workspace
manifest and the npm packages' interdependency pins, and correct the
npm version state.

This RFC is independent of RFCs 030/031 and can be implemented in
parallel with them.

## Motivation

`README.md` presents npm as the primary install path — `npm install -D
apimock-rs`, then `npx apimock`. That path is broken in three
compounding ways, and none of them are visible without running the
tooling.

### 1. `version.sh --update` is a silent no-op

Verified on 2026-08-02 against the v5.14.0 tree. Two independent
causes, either of which alone would be sufficient:

**Cause A — the npm directory is unreachable.** The script iterates
`cargo metadata --no-deps` manifest paths, then searches for
`package.json` with:

```sh
find "$crate_dir" -mindepth 1 -maxdepth 2 -type d
```

Since 5.1.1 those `crate_dir` values are `crates/apimock`,
`crates/apimock-config`, `crates/apimock-routing`, and
`crates/apimock-server`. Root-level `npm/` is not under any of them.
Before 5.1.1 the façade crate lived at the repository root, so the same
`find` did cover `npm/` and `npm/*/`. **The workspace split silently
broke the script, and nothing reported it.**

**Cause B — the TOML rewriter matches nothing.** The rewriter targets:

```awk
!found && /^[[:space:]]*version[[:space:]]*=/ { ... }
```

Every member manifest contains exactly one version line:
`version.workspace = true`. That does not match — after `version` comes
`.`, not whitespace or `=`. Confirmed by running the script's own awk
over `crates/apimock/Cargo.toml`: the output is byte-identical to the
input.

Meanwhile the *real* version lives in the root virtual manifest's
`[workspace.package]`, which `cargo metadata --no-deps` never lists at
all. So the script cannot reach it either.

Net effect: `./version.sh --update 5.14.0` prints four reassuring
"would update" / "updated" lines and changes nothing.

### 2. The npm packages are internally inconsistent

| File | Version |
|---|---|
| `Cargo.toml` `[workspace.package]` | 5.14.0 |
| `npm/package.json` | 5.7.0 |
| `npm/*/package.json` (3 platform packages) | 5.7.0 |
| `npm/package.json` → `optionalDependencies` `@apimock-rs/bin-*` | **4.6.9** |

The `optionalDependencies` pin is the serious one, and it is not
mentioned anywhere in the v5.14.0 handoff bundle. The core package
resolves its platform binaries at `4.6.9` while the platform packages
publish at `5.7.0`. A user installing the core package does not get the
binaries the repository builds.

Note also that `version.sh`'s jq step only ever sets `.version` — so
even a fully repaired script would not fix this pin.

### 3. `npm publish` would fail anyway

`npm-core-publish` runs `npm publish` in `npm/` with the version
unchanged at `5.7.0`. If 5.7.0 was ever published, the registry rejects
it as already-published; the release workflow fails at its last job,
after the Rust assets have already been uploaded to the GitHub release.

### 4. Not a problem: the release-asset archive layout

DEC-031 / RISK-003 in the v5.14.0 handoff bundle flag the release
archives for nesting a top-level directory, against a project rule
requiring files at the archive root.

**On review, that finding does not apply and is dismissed here.** The
rule governs a *project structure archive* — a source-tree tarball
delivered to the project owner, illustrated in the rule text as
`/file1`, `/src/`. The archives produced by `release-executable.yaml`
are a different artifact entirely: a compiled binary plus three example
config files, downloaded by end users from a GitHub release. The rule
was never about them.

The bundle conflated the two, and the nesting it observed came from the
*development-session handoff tarballs* (`tar --transform
's|^work|apimock-rs-X.Y.Z|'`), which are indeed produced flat-rule
non-compliant. That is a process note for whoever prepares a handoff
archive, not a defect in this repository, and it needs no code change.

No archive-layout change is in this RFC's scope.

## Guide-level explanation

After this RFC:

```sh
./version.sh --update 5.15.0
```

updates, in one operation: the workspace manifest's
`[workspace.package].version`, all `npm/*/package.json` versions, the
core `npm/package.json` version, **and** the `optionalDependencies`
pins that reference the platform packages — then verifies its own work
and exits non-zero if any target file still shows the old version.

A release cuts cleanly through to npm without manual intervention.

## Reference-level explanation

### `version.sh`

Rewritten around an explicit list of targets rather than a `cargo
metadata` walk that happens to reach them:

1. **Root manifest.** `[workspace.package].version` — the source of
   truth. Must be edited section-aware, not by first-match: the file
   also contains dependency `version` keys under
   `[workspace.dependencies]`, and a naive first-match rewriter would
   eventually corrupt one.
2. **Member manifests.** Left alone. They inherit via
   `version.workspace = true`, which is correct and needs no edit. The
   old script's attempt to rewrite them was always wrong.
3. **`npm/*/package.json`.** `.version`.
4. **`npm/package.json`.** `.version` **and** every
   `.optionalDependencies["@apimock-rs/bin-*"]` entry, set to the same
   version.
5. **Any `package-lock.json`** present alongside the above.

**Self-verification is mandatory.** After writing, the script re-reads
every target and asserts the new version is present, exiting non-zero
with a diff if not. This is the property whose absence caused the
original defect: the old script's output claimed success it never
checked. A repair that does not add this check has not fixed the real
problem.

`--dry-run` must report exactly the set of files `--update` would
change — no more, no less.

### npm version state

Set `npm/package.json`, `npm/*/package.json`, and the
`optionalDependencies` pins to the version being released, via the
repaired script. Decision **D-03** in `ROADMAP.md` — whether
5.8.0–5.14.0 are backfilled on npm or the channel simply resumes at the
next release — belongs to the project owner and must be recorded in
this RFC before it moves to `done/`.

### Release workflow

Add a version-consistency check as a **pre-publish gate**: fail the
release if the git tag, `[workspace.package].version`, and every npm
`version` / `optionalDependencies` pin do not all agree. This is the
backstop that makes a future recurrence loud rather than silent.

The archive layout, the build matrix, and the set of published targets
are unchanged — see Motivation § 4.

### Explicit non-change scope

- No crate source, no public API, no TOML config schema.
- No change to the build matrix, the set of published targets, or the
  release-archive layout.
- No credential, secret, or registry-authentication change. Where npm
  publish credentials live was not verified and is out of scope.

## Required tests

Shell tooling, so the evidence is executed runs rather than unit tests:

1. **`version.sh` round-trip.** On a scratch copy: run
   `--update 9.9.9`, then assert by `grep` that all five target
   categories show `9.9.9`. Then run `--update 5.14.0` and assert the
   tree is byte-identical to the original.
2. **Self-verification fires.** Deliberately make one target
   unwritable or malformed; confirm the script exits non-zero and names
   the file.
3. **`--dry-run` fidelity.** Confirm the dry-run file list exactly
   equals the set `--update` modifies.
4. **Pre-publish gate fires.** A workflow run with a deliberately
   mismatched npm version must fail *before* any publish step.

## Acceptance criteria

1. `./version.sh --update <ver>` updates the workspace manifest, all
   npm versions, and all `optionalDependencies` pins in one run.
2. The script self-verifies and exits non-zero on any target it failed
   to update.
3. `--dry-run` reports exactly the files `--update` would change.
4. No `package.json` in the tree disagrees with
   `[workspace.package].version`.
5. The release workflow fails before publishing on any version
   mismatch between tag, workspace manifest, and npm files.
6. The release-archive layout is unchanged.

## Drawbacks

Rewriting `version.sh` replaces a script whose behaviour is at least
familiar with one that is new. Mitigated by the round-trip test and by
the self-verification step — the new script's failure mode is a loud
non-zero exit, where the old one's was silent success.

The pre-publish consistency gate can block a release at the last
moment. That is preferable to the current behaviour, which is to
publish an inconsistent package or fail after the GitHub release
already exists.

## Rationale and alternatives

**Alternative A (chosen): repair the tooling.** The owner confirmed on
2026-08-02 that npm remains a supported channel. Given that, the
tooling has to work.

**Alternative B: drop npm, document `cargo install` only.** Coherent,
and it would delete this entire class of problem — but it contradicts
the README's own framing and the owner's decision. Rejected.

**Alternative C: fix the versions by hand now, repair tooling later.**
Rejected: hand-fixing is what has been happening, and it is why the
`optionalDependencies` pin sat four minor versions out of date without
anyone noticing.

**Alternative D: replace `version.sh` with `cargo-release` or a
similar published tool.** Genuinely attractive — a maintained tool
would not have this defect class. Not chosen now because it changes the
release process wholesale during a milestone whose purpose is to make
the *existing* process trustworthy. Worth revisiting once M1 is done.

## Unresolved questions

1. ~~**D-03: backfill npm or resume at the next release?**~~
   ✅ **Resolved 2026-08-02 by the project owner: resume.** Not
   backfilled. The 5.15.0 release notes should say so plainly, since
   npm users will see a version gap.

   **Factual correction, 2026-08-03.** This RFC states the gap as
   "5.7.0 → next release, 5.8.0–5.14.0 unpublished". That figure was
   inferred from `npm/package.json`'s local content and is wrong. The
   live registry shows the last version actually published to npm is
   **5.10.0** (2026-05-16); 5.8.0 was never published, but 5.9.0 and
   5.10.0 were. **The real gap is 5.10.1–5.14.0.** The published 5.10.0
   carries the `4.6.9` `optionalDependencies` pin this RFC fixes — so
   the defect was live for npm users, not merely latent in the repo.

   crates.io, by contrast, is current: all four crates published
   through 5.14.0 as of 2026-08-01.

   The *decision* is unaffected — resume rather than backfill — only
   the range it describes. Recorded as an addendum rather than by
   editing the text above, per this project's precedent for correcting
   a `done/` RFC (see the v5.11 addendum in
   [RFC 007](./007-rule-evaluation-strategy-variants.md)).
2. **Are npm registry credentials still valid?** Not verifiable from
   the repository. Must be confirmed before the first release that
   exercises the repaired path, or the release will fail at publish
   for an unrelated reason and confuse the diagnosis.
