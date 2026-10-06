# RFC 084 — Assert the release notes are *true*, not merely present

**Status.** Proposed — awaiting owner approval.
**Tracks.** Release process. Strengthens [RFC 081](../done/081-tiered-release-confirmation.md)
§ 3 rather than widening § 2.
**Touches.** `.github/workflows/release-executable.yaml`, a new
`.github/workflows/scripts/assert-changelog-claims.sh`, `RELEASING.md`.
**Origin.** 2026-10-06. The owner asked for releases *"independent of my
manual operation as possible"*. The architect first proposed reaching
that by **dropping** RFC 081's no-API-change condition so Tier A became
reachable, then **withdrew it** on review: that is loosening a safety
gate so a desired automation becomes usable, which inverts the owner's
standing position that the less secure option is the one to avoid. This
RFC is the replacement — make the machine verify more of what a human
currently trusts, so the gate can shrink on evidence instead of by
decree.

## Summary

CI already asserts that a release's notes **equal** its CHANGELOG
section (RFC 081 § 3). Nothing asserts the section is **accurate**.

That gap is the entire remaining content of the publish click. RFC 081
said so in its own words: *"CI asserts the CHANGELOG section exists;
nothing asserts it is accurate. Release notes go to crates.io as a
permanent public record."*

So: assert the claims that **can** be checked, in the build phase,
before anything is published. Three classes, and **each one traces to a
real incident in this project**.

## Motivation — every assertion below exists because we got it wrong

**1. API-change claims.** 6.2.1's notes say *"the API baselines are
byte-identical to 6.2.0's."* True — I verified it by hand. But
`docs/src/library/api-stability.md` carried an **additive-only 6.x
promise that was never approved and contradicted the RFC it cited**, and
it stood published for weeks. A prose claim about API compatibility is
exactly the kind this project has already got wrong in public.

**2. Advisory claims.** While handling GHSA-8v8q-w7pc-3cg3 I drafted a
statement declaring apimock 5.0.0–5.19.0 **unaffected** by an advisory
they were vulnerable to. It was caught before it shipped — by attention,
which is the thing RFC 081 exists to stop relying on.

**3. Version-number claims.** 6.2.1's notes say *"Upgraded `rustls`
0.23.43 → 0.23.45"*. `version.sh` keeps **manifests** consistent and
verifies seven surfaces; it does not read prose. A notes entry naming a
dependency version that the lockfile contradicts would ship unnoticed.

None of these are hypothetical, and none are caught by anything today.

## Design

A new job, `assert-changelog-claims`, in the **build phase**
(`release-executable.yaml`), `needs:`-gated the same way
`assert-draft-release` is — so it runs on the tag, before publishing,
and a failure is fixable by re-running the build phase rather than by
yanking.

It reads the tagged commit's CHANGELOG section for the tag's version and
asserts:

### 1. An API claim matches the baselines

If the section asserts no API change — matching a small set of phrases
(`byte-identical`, `no public API change`, `no API change`) — then
`git diff <prev-tag>..<tag> -- 'crates/*/public-api.txt'` **must be
empty**, and the baselines must exist at both tags.

Conversely, if the baselines **did** move and the section contains no
`### Added`/`### Changed`/`### Removed` heading and no migration-guide
link, **fail**: an undeclared API change is the exact failure RFC 039's
gate exists to surface, and this catches it in prose as well as in the
baseline.

### 2. Every advisory identifier resolves, and we do not assert
### un-affectedness without evidence

- Extract `RUSTSEC-\d{4}-\d{4}`, `GHSA-[a-z0-9-]+`, `CVE-\d{4}-\d+`.
- Each must **resolve** — `rustsec.org/advisories/<id>` for RUSTSEC, the
  GitHub advisory API for GHSA/CVE. A typo'd advisory ID in permanent
  public notes is both wrong and unfixable after publish.
- **A sentence claiming versions are *unaffected* fails the job unless
  the same section cites the evidence.** This one is deliberately blunt:
  it is the claim I nearly shipped wrongly, and a machine cannot judge
  the analysis. Requiring the claim to carry its reasoning is what the
  job can enforce. If that proves too blunt in practice, narrowing it is
  a cheap amendment; starting permissive is not, because the failure is
  permanent.

### 3. Dependency version claims match the lockfile

For each `name X → Y` / `name X -> Y` pattern naming a crate in
`Cargo.lock`, the **target** `Y` must equal the version the lockfile
actually resolves. 6.2.1's *"rustls 0.23.43 → 0.23.45"* would be checked
against `rustls 0.23.45`, and a stale figure would fail.

### 4. What it deliberately does **not** do

- **It does not judge prose.** Whether a feature description is a fair
  summary stays human. This job checks claims with a machine-readable
  referent, nothing else.
- **It does not gate on style**, length, or wording.
- **It does not replace the tier system.** RFC 081 § 2's conditions are
  untouched, and **the no-API-change condition stays** — see § Origin.

## How this reaches the owner's actual request

Two steps, in order, neither of which loosens a gate:

1. **This RFC.** The notes' checkable claims stop depending on the
   architect reviewing the architect's prose.
2. **Then, and only then, the click is worth revisiting** — on the
   evidence that what it protects is now enforced. If Tier A still never
   occurs (it never has: 5.19.1, 6.0.0, 6.1.0, 6.2.0 and 6.2.1 are all
   Tier B, most for two independent reasons), the honest next move is to
   make the **Tier B** confirmation cheap rather than to make Tier A
   reachable: have CI post the classification and every assertion result
   so the owner confirms a summary instead of conducting an
   investigation.

[RFC 083](./083-publish-without-a-click.md) stays proposed and should
not be implemented on a manufactured premise.

## Testing and verification

- **Prove each assertion fires**, on a throwaway tag, by crafting the
  failure — a notes entry claiming `byte-identical` while a baseline
  moved; a typo'd `RUSTSEC-9999-9999`; a `rustls 0.23.43 → 0.23.44`
  figure the lockfile contradicts. **Not by observing green.**
- **Replay against real history.** Run it over 6.0.0, 6.1.0, 6.2.0 and
  6.2.1 as tagged. **All four must pass**; any false positive means the
  phrase matching in § 1 is too eager, and that must be fixed before
  adoption rather than discovered on a live release.
- Confirm a network failure reaching an advisory database fails
  **closed** with a clear message, and never silently skips — the
  `package` job's inconclusive-query skip is the precedent for how this
  goes wrong.

## Risks

| Risk | Mitigation |
|---|---|
| The job blocks a legitimate release late | It is in the build phase, before publishing; a failure is fixed by correcting the CHANGELOG and re-tagging, and nothing has reached a registry. |
| Phrase matching in § 1 produces false positives | The replay over four real releases must pass before adoption; the phrase set is small and explicit, not a general parser. |
| § 2's unaffectedness rule is too blunt | Stated as deliberate, with narrowing offered as a cheap amendment. Erring towards a failed build beats erring towards a permanent public claim. |
| An advisory database is unreachable in CI | Fails closed, loudly. An unverifiable advisory ID in permanent notes is not an acceptable default. |
| It creates the impression the notes are *verified* | § 4 is explicit that prose is not judged. `RELEASING.md` must say which claims are enforced and which are still trusted. |

## Unresolved questions

1. **Should § 2's unaffectedness rule ship on, or off, first?** Starting
   on is my recommendation and the safer default; the owner may prefer a
   release of observation first.
2. **Does the Tier B summary comment belong here or in RFC 083?** It is
   the part that most directly answers the owner's request and it
   loosens nothing, so it could ship independently of either RFC.
