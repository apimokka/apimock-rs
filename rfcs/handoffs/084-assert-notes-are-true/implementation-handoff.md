# Handoff — RFC 084: assert the release notes are *true*

**Governing RFC.** [084](../../accepted/084-assert-the-notes-are-true.md),
**accepted 2026-10-06** with both Unresolved questions decided on
approval. § 5 was added at approval and is in scope.
**Ships how.** Continuously published — this is a CI gate, **no version
bump, no release**. Merge to `main` and it is live for the next tag.
**Baseline.** **`main`'s head — cut from it.** No hash pinned. At time
of writing `63c67c2`, CI green.
**Branch.** Not needed — `main`. The job runs on `ubuntu-latest` only;
nothing here can behave differently on Windows or macOS (RFC 080 § 3).
**Authorisation.** The owner approved this RFC, which is what permits
touching the release path. **It does not touch
`.github/workflows/release-publish.yaml`** — RFC 066 § 2's named
prohibition — only the build phase. If you find yourself needing to
edit `release-publish.yaml`, **stop and report**.

---

## 0. The one-sentence version

CI already checks the release notes **equal** the CHANGELOG section. Add
a job that checks the section is **true** — for the three classes of
claim that have a machine-readable referent, each of which this project
has got wrong before.

## 1. Where it goes, and what it must be gated on

A new job in `.github/workflows/release-executable.yaml`:

```yaml
  assert-changelog-claims:
    needs: [build]
    runs-on: ubuntu-latest
```

- `needs: [build]`, the same gate `assert-draft-release` uses
  (`release-executable.yaml:272`), so it runs **on the tag, before
  anything is published**, and a failure is fixed by correcting the
  CHANGELOG and re-tagging.
- `env.TAG` is already `${{ github.ref_name }}` at file scope
  (`:30`) — the workflow triggers on `push: tags: ['*.*.*']` (`:19`).
  Use it; do not re-derive the version.
- **Checkout with full history** (`fetch-depth: 0`) **and tags**. § 2
  diffs against the *previous* tag, which a shallow clone cannot see.
  This is the most likely way to get a false green.

## 2. Reuse the extractor — do not write a second one

**`.github/workflows/scripts/extract-changelog-section.sh` already
exists** and is already called twice (`:151`, `:320`). It takes
`<version> [changelog-path]`, prints the section body, and **exits
non-zero with no stdout when the version has no section**.

Use it. A second CHANGELOG parser in this repo would be the
duplicated-logic defect class the audit already charged us with three
times — and it would be a parser that can disagree with the one
generating the notes, which is the specific failure that makes this
whole job pointless.

Put the new logic in
`.github/workflows/scripts/assert-changelog-claims.sh`, taking the
version so it can be **run locally against any tag** — the same reason
`wait-for-npm-propagation.sh` was extracted. A gate that cannot be
exercised outside a live release is an untested gate.

## 3. The three assertions

### (a) An API claim must match the baselines

- If the section contains any of — `byte-identical`, `no public API
  change`, `no API change` (case-insensitive) — then
  `git diff <prev-tag>..<tag> -- 'crates/*/public-api.txt'` **must be
  empty**, and the four baselines must **exist at both tags**. Absence
  is a failure, not a pass; this is RFC 081 Amendment 1's lesson and it
  applies here verbatim.
- Conversely: if the baselines **did** move and the section has no
  `### Added` / `### Changed` / `### Removed` heading **and** no
  migration-guide link, **fail**. An undeclared API change is what RFC
  039's gate exists to surface.
- Previous tag: derive it, do not hardcode. Be explicit about what
  "previous" means for a patch release and **state your choice in the
  package** — 6.2.1's predecessor is 6.2.0, and a naive
  "latest tag before this one" is right here but say so deliberately.

### (b) Every advisory identifier resolves; unaffectedness needs evidence

- Extract `RUSTSEC-\d{4}-\d{4}`, `GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}`,
  `CVE-\d{4}-\d+`. 6.2.1's entry cites `RUSTSEC-2026-0285` — use it as
  your known-good fixture.
- Each must resolve: `https://rustsec.org/advisories/<id>` for RUSTSEC;
  the GitHub advisory API for GHSA/CVE.
- **A sentence claiming versions are *unaffected* fails unless the same
  section cites evidence.** The owner approved this **on**, deliberately
  blunt: a build failure is fixed by editing the CHANGELOG, while a
  wrong unaffectedness claim on crates.io cannot be withdrawn. Define
  "cites evidence" concretely in the script and **say what you chose** —
  this is the judgement call in the task.
- **Fail closed on a network error.** Not "skip, and go green". The
  `package` job's inconclusive-query skip is in this repo precisely as
  the example of how this goes wrong, and it is called out in
  `RELEASING.md`.

### (c) Dependency version claims match the lockfile

- For each `<name> X → Y` / `<name> X -> Y` where `<name>` is a crate in
  `Cargo.lock`, the **target** `Y` must equal the version the lockfile
  resolves.
- 6.2.1's *"Upgraded `rustls` 0.23.43 → 0.23.45"* is your fixture; it
  must pass, and `0.23.44` must fail.
- Note both arrow forms, and that the notes use a Unicode `→`.

### (d) Explicitly out of scope

**Do not judge prose.** No style, length or wording checks, and no
attempt to decide whether a feature description is fair. Only claims
with a machine-readable referent. If you find yourself writing a
heuristic for "is this sentence accurate", stop — that is the part that
stays human, and § 4 of the RFC says so.

## 4. § 5 — the Tier B summary block

Added at approval. **It loosens nothing**: the owner still publishes
every Tier B release.

Post a short block **on the draft Release** stating:

- the **tier**, with each of RFC 081 § 2's four conditions and its result
- `assert-draft-release`'s outcome (five assets; notes == CHANGELOG)
- **this job's assertions, itemised** — the API claim, each advisory ID
  resolved, each dependency version checked
- the build-phase **run id, as a link**

- [ ] **It must state which claims were enforced and which remain
      trusted prose.** A block that reads as "verified" when prose was
      never judged is this RFC's own defect class aimed at the owner.
      Word it so that cannot be misread.
- [ ] It must not fail the build if posting fails — report, don't block.
      The assertions are the gate; the summary is a convenience.

## 5. Acceptance

**Prove each assertion fires. A green run proves nothing here.**

- [ ] (a) fires: a section claiming `byte-identical` while a baseline
      moved — crafted on a throwaway tag, output quoted
- [ ] (a) fires: baselines moved, no `### Changed`, no migration link
- [ ] (b) fires: a typo'd `RUSTSEC-9999-9999`
- [ ] (b) fires: an unaffectedness sentence with no evidence
- [ ] (b) fails **closed** when the advisory host is unreachable —
      simulate it
- [ ] (c) fires: `rustls 0.23.43 → 0.23.44` against a lockfile at
      `0.23.45`
- [ ] **Replay against real history: 6.0.0, 6.1.0, 6.2.0 and 6.2.1 as
      tagged must all PASS.** Any false positive means (a)'s phrase set
      is too eager — fix it before this lands, not on a live release.
      **Quote all four results.**
- [ ] `fetch-depth: 0` present, and a test that the job fails rather
      than silently passing when the previous tag is unreachable
- [ ] The script runs standalone: `bash …/assert-changelog-claims.sh 6.2.1`
- [ ] `release-publish.yaml` **untouched** — confirm with `git diff --name-only`
- [ ] `RELEASING.md` says which claims are enforced and which are still
      trusted prose
- [ ] CI green on `main` (RFC 066 Amendment 3)

## 6. Report back

`.git-exclude/review-request/084-assert-notes-are-true/`, including:

- the four-release replay results, quoted
- your definition of "cites evidence" for § 3(b), with reasoning
- your choice of "previous tag" and why
- each crafted failure's output
