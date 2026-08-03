# Implementation Handoff — RFC 032, Release and packaging repair

**Governing RFC.** [RFC 032](../../proposed/032-release-and-packaging-repair.md)
**Milestone.** M1 (Pipeline trust) → v5.15.0
**Status.** Inherited from RFC 032 (Proposed, approved for implementation
2026-08-02)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

**Runs in parallel with RFC 030.** No shared files, no shared code.

---

## 1. Purpose

Make the npm release path work. Today it cannot: the version-sync tool
is a silent no-op, the npm packages disagree with each other and with
the workspace, and a release run would fail at its final job.

## 2. Background

`README.md` advertises npm first — `npm install -D apimock-rs`, then
`npx apimock`. Three compounding defects, verified 2026-08-02:

**(a) `version.sh --update` changes nothing.** Two independent causes,
either sufficient alone:

- It searches for `package.json` under `cargo metadata --no-deps`
  manifest directories — `crates/apimock*/` since the 5.1.1 split. Root
  level `npm/` is not under any of them. Before 5.1.1 the façade crate
  lived at the repo root, so the same `find` did reach `npm/`. The
  workspace split broke it silently.
- Its TOML rewriter matches `/^[[:space:]]*version[[:space:]]*=/`.
  Member manifests contain only `version.workspace = true`, which does
  not match. Verified by running the script's own awk over
  `crates/apimock/Cargo.toml`: output byte-identical to input.

The real version lives in the root virtual manifest's
`[workspace.package]`, which `cargo metadata --no-deps` never lists.

**(b) npm state is internally inconsistent.**

| File | Version |
|---|---|
| `Cargo.toml` `[workspace.package]` | 5.14.0 |
| `npm/package.json` | 5.7.0 |
| `npm/*/package.json` ×3 | 5.7.0 |
| `npm/package.json` → `optionalDependencies` `@apimock-rs/bin-*` | **4.6.9** |

The `optionalDependencies` pin is the serious one and appears in no
prior handoff. `version.sh`'s jq step only sets `.version`, so even a
repaired script would miss it unless explicitly taught.

**(c) `npm publish` would fail.** `npm-core-publish` publishes `npm/` at
an unchanged 5.7.0; the registry rejects an already-published version —
after the Rust assets have already been uploaded to the GitHub release.

## 3. Applicable requirements

RFC 032, plus owner decision **D-03 (2026-08-02): npm resumes at the
next release.** 5.8.0–5.14.0 are **not** backfilled. npm goes 5.7.0 →
5.15.0.

## 4. Change scope

- `version.sh` — rewrite
- `npm/package.json` — `version` and `optionalDependencies` pins
- `npm/darwin-arm64/package.json`, `npm/linux-x64-gnu/package.json`,
  `npm/win32-x64-msvc/package.json` — `version`
- `.github/workflows/release-executable.yaml` — add pre-publish
  version-consistency gate

## 5. Explicit non-change scope

Do **not**:

- Touch any crate source, public API, or TOML config schema.
- Change the build matrix or the set of published targets.
- **Change the release-archive layout.** RFC 032 § 4 dismisses the
  DEC-031 / RISK-003 finding as a misapplication — the flat-extraction
  rule governs project-structure archives delivered to the owner, not
  the binary assets CI builds. Leave the archives exactly as they are.
- Touch registry credentials or workflow `secrets` wiring.
- Upgrade or alter any dependency.
- Modify `index.js` or `postinstall.js` — their logic is correct; only
  the versions around them are wrong.

## 6. Required implementation

### 6.1 Rewrite `version.sh`

Drive from an explicit target list, not a `cargo metadata` walk that
happens to reach things.

Targets:

1. **Root `Cargo.toml`** → `[workspace.package].version`.
   **Section-aware edit, not first-match.** The same file contains
   `version` keys under `[workspace.dependencies]`; a naive first-match
   rewriter will eventually corrupt one.
2. **Member manifests** → leave alone. `version.workspace = true` is
   correct and needs no edit. The old script's attempt to rewrite them
   was always wrong.
3. **`npm/*/package.json`** → `.version`.
4. **`npm/package.json`** → `.version` **and** every
   `.optionalDependencies["@apimock-rs/bin-*"]` entry, set to the same
   version.
5. **Any `package-lock.json`** alongside the above.

**Self-verification is the point of this task.** After writing, re-read
every target and assert the new version is present; exit non-zero with
a diff if any target still shows the old value. The original defect was
not that the script had a bug — it was that the script reported success
it never checked. A rewrite without this step has not fixed the real
problem.

`--dry-run` must report exactly the set `--update` would change.

Keep the existing CLI surface (`--list`, `--update`, `--dry-run`,
`--help`). The script is `/bin/sh`; keep it POSIX.

### 6.2 Correct npm version state

Run the repaired script to bring every npm file to the current
workspace version. Per D-03, no backfill of 5.8.0–5.14.0.

### 6.3 Pre-publish gate

In `release-executable.yaml`, before any publish job: fail the release
if the git tag, `[workspace.package].version`, and every npm `version`
and `optionalDependencies` pin do not all agree. This is the backstop
that makes recurrence loud.

## 7. Required tests

Shell tooling — evidence is executed runs.

1. **Round-trip.** On a scratch copy: `--update 9.9.9`, assert by grep
   that all target categories show `9.9.9`; then `--update 5.14.0` and
   assert the tree is byte-identical to the original.
2. **Self-verification fires.** Make one target unwritable or malformed;
   confirm non-zero exit naming the file.
3. **`--dry-run` fidelity.** Dry-run file list exactly equals what
   `--update` modifies.
4. **Pre-publish gate fires.** A run with a deliberately mismatched npm
   version fails *before* any publish step.

Do all destructive testing on a copy under a scratch directory, never
the working tree.

## 8. Required documentation updates

- `.github/CONTRIBUTING.md`: one line on how a version bump is
  performed (`./version.sh --update <ver>`), so the next person does not
  hand-edit.
- No `docs/` or `README.md` change — nothing user-visible changes.
- No CHANGELOG entry; that is written at M1 release preparation.

## 9. Acceptance criteria

1. `./version.sh --update <ver>` updates the workspace manifest, all npm
   versions, and all `optionalDependencies` pins in one run.
2. The script self-verifies and exits non-zero on any target it failed
   to update.
3. `--dry-run` reports exactly what `--update` changes.
4. No `package.json` disagrees with `[workspace.package].version`.
5. The release workflow fails before publishing on any version mismatch.
6. The release-archive layout is unchanged.

## 10. Prohibited shortcuts

- Hand-editing the npm versions and calling it done. Hand-editing is why
  the `optionalDependencies` pin sat four minor versions stale. The
  tooling must do it.
- Omitting self-verification because the script "obviously works" — that
  is precisely the assumption that failed.
- Rewriting `version.sh` in a language other than POSIX `sh` without
  raising it first; that changes the project's tooling dependencies.
- Testing `--update` against the real working tree.

## 11. Compatibility constraints

`npx apimock` and `cargo install apimock` must keep working unchanged
for end users. The crate name, binary name, npm package names, and
platform-package names are all fixed.

## 12. Security constraints

Do not read, print, echo, or log registry credentials. The release
workflow's `permissions` block and `secrets` references are out of
scope — leave them exactly as they are.

## 13. Known risks

| Risk | Mitigation |
|---|---|
| A section-unaware rewrite corrupts `[workspace.dependencies]` version pins | Section-aware edit required; round-trip test asserts byte-identical restore |
| Rewritten script has a new, different bug | Self-verification makes failure loud; round-trip test is the harness |
| npm credentials turn out to be invalid | Out of scope, but flag it — confirm before the first real release, or a publish failure will be misdiagnosed as this RFC's fault |

## 14. Required evidence

- Full output of the round-trip test, both directions
- Output of the self-verification failure case
- `--dry-run` output alongside the `--update` file list
- The workflow run showing the pre-publish gate failing on a
  deliberately mismatched version
- `grep -rn '"version"' npm/` and the root manifest's version, side by
  side, showing agreement

## 15. Required review-request format

Package under
`.git-exclude/review-request/032-release-and-packaging-repair/` with an
entry point usable without prior context. Per § 9.2 of the workflow
document: implementation summary, addressed requirements, changed files,
implementation decisions, deviations from the approved design, executed
tests and results, build/static-analysis results, unresolved issues,
known limitations, requested review focus.

Reviewer's focus will be **the self-verification step and the
section-aware root-manifest edit** — the two places where a plausible
implementation can still be wrong in the same way the original was.
