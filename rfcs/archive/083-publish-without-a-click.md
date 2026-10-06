# RFC 083 — Publish without a click: finish the act the tag already started

**Status.** **Archived 2026-10-07 — superseded by
[RFC 086](../done/086-the-cut-authorises-the-publish.md).** Not
implemented. The owner's goal — releases independent of their manual
operation — is met for minor releases by RFC 086, which needs no
workflow change, no long-lived credential and no `workflow_call`
restructuring: the architect publishes with user-authenticated `gh`.
**Kept for its finding:** a release published by CI with Actions'
default `GITHUB_TOKEN` does not fire `release-publish.yaml`. Anyone who
later revisits publishing *from inside CI* should start from § 3.
**Tracks.** Release process. Closes issue
[#82](https://github.com/apimokka/apimock-rs/issues/82) — *"ci: cargo
publish when building executables is successful"*.
**Supersedes in part.** RFC 081 § 1 and § 4; RFC 081's Non-goal
*"Continuous deployment. A release stays a deliberate act."* is
addressed head-on in § 1 rather than quietly dropped.
**Touches.** `.github/workflows/release-executable.yaml`,
`.github/workflows/release-publish.yaml`, `RELEASING.md`, and RFC 066
§ 2.
**Origin.** Issue #82 (owner's, 2026). Reopened 2026-10-06: the
architect proposed closing it as *decided against*, and stopped on
noticing it matches what the owner had said in the same session —
*"I want it independent of my manual operation as possible."*

## Summary

For a **Tier A** release, let the publish happen automatically once the
build phase goes green, instead of waiting for someone to click
Publish. Tier B is unchanged — the owner publishes.

**This is not continuous deployment.** The owner still pushes the tag,
and the tag is still the only thing that starts a release. What this
removes is a second confirmation of a decision already taken.

**There is a blocker, and it is the reason this needs an RFC rather than
a patch:** a release published by CI using the default `GITHUB_TOKEN`
**will not fire `release-publish.yaml` at all**. Done naively, #82's
fix produces a published GitHub Release with nothing on crates.io or
npm — a worse partial release than any this project has had. § 3 is
about that.

## Motivation

### What the Tier A click actually protects — almost nothing

RFC 081 § Motivation named three things CI cannot see. Two of them are
already settled before the click:

1. **Whether the release should happen now.** Answered by the owner at
   the **tag**. `RELEASING.md`: *"The tag push is the only thing
   triggered directly, and it stays the owner's … a release cannot
   begin without the owner regardless of tier."* The calendar judgement
   happens there, not at the draft.
2. **It cannot undo.** True and permanent — but this is an argument
   about publishing something *wrong*, not about publishing at the
   wrong *moment*. It is answered by the build-phase assertions, which
   § 2 makes a precondition of publishing rather than a parallel
   activity.
3. **Whether the release notes are true.** The one that survives — and
   for Tier A it is **already** self-review. RFC 081 says so in its own
   words: *"I write the release notes. Under a blanket 'architect may
   publish' I would also approve them."* It then granted Tier A anyway,
   reasoning that Tier A excludes security claims and API changes, *"the
   two places notes have actually been wrong here."*

So for Tier A the click is the architect re-reading its own prose. That
is the weakest review in the system, and removing it costs close to
nothing.

### It also fixes a hazard RFC 081 admitted it could not

RFC 081 Amendment 1 (B) recorded that `create-draft-release` runs
**before** `build`, so the draft is publishable from the moment it
exists — long before `assert-draft-release` has said anything — and
that the assert job *"cannot prevent an early publish"*. The current
mitigation is a sentence in `RELEASING.md` telling a human to confirm
the build phase green by run id.

If publishing is **triggered by** the assertions passing, that hazard
stops being a documented caution and becomes structurally impossible.
A human can publish early; a `needs:` edge cannot.

## Design

### 1. The deliberate act moves nowhere — it was always the tag

RFC 081's Non-goal says *"a release stays a deliberate act."* It still
is. The owner decides the version, authorises the cut, and pushes a
signed tag. Nothing here lets a merge to `main` reach a registry, and
nothing publishes without a tag the owner caused.

What changes is that the owner's decision is no longer re-confirmed by a
second manual step whose only remaining content is *"did the architect's
own notes look right to the architect?"*

### 2. Tier A publishes itself, from inside the build phase

A new job in `release-executable.yaml`:

```yaml
  auto-publish:
    needs: [assert-draft-release]
    if: <classified Tier A, per § 4>
```

Because it is `needs:`-gated on `assert-draft-release`, the five assets
and the notes/CHANGELOG match are **proven before** anything is
published. Tier B skips this job and the draft waits for the owner,
exactly as today.

### 3. The blocker: `GITHUB_TOKEN` cannot fire the publish phase

GitHub does not start workflow runs from events created with the default
`GITHUB_TOKEN` — a deliberate anti-recursion rule. `release-publish.yaml`
fires **only** on `release: types: [published]`, which its own header
calls *"a structural fact, not a convention."*

So `gh release edit "$TAG" --draft=false` with `secrets.GITHUB_TOKEN`
would mark the Release published and **publish nothing**. The failure
is silent and public: GitHub shows a real release, the registries have
nothing, and the workflow that would have fixed it never ran.

Two ways out.

**(a) A PAT or GitHub App token with release-write scope.** Smallest
diff, and **recommended against.** It introduces a long-lived
credential that can publish releases, to replace a click whose value we
just argued is near zero. That trades a structural security property
for convenience, which is the wrong direction for this project.

**(b) Call the publish phase directly — recommended.** Convert
`release-publish.yaml`'s jobs to `workflow_call`, and have the build
phase invoke them after `assert-draft-release`. The `release:
published` trigger is **kept** so a Tier B release published by the
owner still works exactly as it does now. The draft→published flip
happens with `GITHUB_TOKEN` and no longer needs to *trigger* anything,
because the publishing is already underway.

(b) needs no new credential, keeps both entry points, and makes the
assertion a true precondition. It does restructure the publish path, so
it needs the owner's approval under RFC 066 § 2 — which is what this
RFC is asking for.

**Order matters, and must be decided:** flip the draft *before* the
registry publishes (public release, registries filling) or *after* (the
registries lead, the Release goes public last). **Recommend after**, so
a registry failure leaves the draft unpublished and the release
recoverable by re-running, which is how both prior npm failures were
repaired.

### 4. Tier A must be machine-classified, and still declared

Auto-publishing on a tier means the tier can no longer be a judgement
recorded in a release record. RFC 081 § 2's four conditions split:

| condition | machine-readable? |
|---|---|
| No `### Security` section in the CHANGELOG entry | **yes** |
| No `crates/*/public-api.txt` change since the previous tag, baselines present at both | **yes** |
| Major component unchanged | **yes** |
| No new or lowered default that can refuse a previously-accepted request | **no — judgement** |

So: the tier is **declared in-repo, in the tagged commit**, and CI
**verifies the declaration against the three mechanical conditions**.

- Declaration absent or unreadable → **Tier B.** The draft waits.
- Declares A, and all three mechanical conditions agree → auto-publish.
- Declares A, but a mechanical condition says B → **fail the build
  phase loudly and publish nothing.** The declaration was wrong, which
  is a process failure to report under RFC 081 § 5, not a thing to
  work around.
- Declares B → the draft waits.

This keeps RFC 081 § 5's rule — *classification is declared, never
inferred silently* — and makes "fail towards Tier B" structural rather
than a disposition. The judgement condition stays human, but it is now
written down, in git, before the tag, instead of asserted in a message
after the fact.

### 5. A precondition this RFC should not ship without

Auto-publish removes the human who has twice noticed a partial publish
in progress. `verify-published` checks the three platform npm packages
and **never the core `apimock-rs` package** — the one users install and
the one that failed at 6.0.0 and 6.2.0 (ROADMAP carries this).

**This RFC should not ship before that gap is closed.** Removing the
watcher while the highest-risk artifact is unverified is the wrong
order, even though the adaptive wait (2026-10-06) made the race much
less likely.

## Non-goals

- **Touching Tier B.** A security fix, an API change, a major, or a
  lowered refusing default still means the owner publishes.
- **Letting a merge to `main` reach a registry.** Only a tag starts a
  release; only the owner tags.
- **Changing what publishes, in what order, or with what credentials**
  beyond § 3's restructure, which exists specifically to *avoid* a new
  credential.
- **Removing the `release: published` trigger.** It stays, for Tier B.

## Testing and verification

- **Prove the `GITHUB_TOKEN` constraint on a throwaway tag before
  building anything on top of it.** Publish a draft from inside Actions
  with the default token and confirm whether `release-publish.yaml`
  fires. § 3 rests on documented GitHub behaviour, not on an
  observation in this repository — if it turns out to fire, option (a)
  stops being necessary and this RFC gets simpler.
- A declared-A release whose mechanical conditions say B **fails the
  build phase** — prove it by crafting that mismatch on a throwaway
  tag, not by observing a green run.
- A release with no declaration leaves the draft unpublished.
- Dry-run the classifier against 6.0.0, 6.1.0, 6.2.0 and 6.2.1 and
  report the tier each would have got. 6.2.1 must read Tier B (it has a
  `### Security` section). If any reads Tier A that a person would call
  Tier B, § 4 is wrong and this RFC needs changing before adoption.
- A registry failure with § 3(b)'s recommended ordering leaves the
  Release **unpublished** and re-runnable.

## Risks

| Risk | Mitigation |
|---|---|
| CI publishes a release whose notes are wrong | Tier A excludes security content and API changes by condition — the two places notes have actually been wrong here. Unchanged from RFC 081's accepted reasoning; what is removed is the architect reviewing its own prose. |
| The draft is marked published but registries get nothing | The whole of § 3. This is the failure mode a naive fix produces, and it is why the RFC exists. |
| A mis-declared tier auto-publishes something that needed the owner | § 4 cross-checks the declaration against three mechanical conditions and fails towards Tier B; a declared A that contradicts them stops the build phase. |
| Auto-publish removes the human who caught two partial publishes | § 5 makes core-npm verification a precondition. |
| The restructure in § 3(b) breaks the owner's Tier B path | The `release: published` trigger is kept and must be exercised on a throwaway tag before this lands. |

## Unresolved questions

1. **Where does the tier declaration live?** A dedicated file in the
   tagged commit (`.github/release-tier`, holding version and tier) is
   the simplest auditable option. It must **not** go in the CHANGELOG
   entry, because `assert-draft-release` requires the release notes to
   equal that section verbatim — a marker there would ship to crates.io
   as part of the notes.
2. ~~**Does the owner want Tier A to stay as narrow as it is?**~~
   **Answered 2026-10-06: it stays exactly as narrow.** The architect
   proposed *widening* it — dropping RFC 081 § 2's no-API-change
   condition — and withdrew that on review. See § Amendment note.
3. **Should `apimock`'s own crates.io publish order change?** No reason
   found; raised only because § 3(b) is the first thing to restructure
   that path.


---

## Amendment note — 2026-10-06: the premise is false, and must not be manufactured

**This RFC is on hold.** Not withdrawn — its reasoning about the Tier A
click stands — but it **automates a branch that has never executed.**

### Measured, after drafting

Every recent release is Tier B, most for two independent reasons:

| release | `crates/*/public-api.txt` changed | `### Security` | major | tier |
|---|---|---|---|---|
| 5.19.1 | 0 | yes | same | **B** |
| 6.0.0 | 0 | yes | bumped | **B** |
| 6.1.0 | 4 | yes | same | **B** |
| 6.2.0 | 2 | yes | same | **B** |
| 6.2.1 | 0 | yes | same | **B** |

**Tier A has never happened.** So this RFC would restructure the
project's most dangerous code path — three production failures behind it
— to automate a case that does not arise.

### The withdrawn proposal, recorded because it was wrong

On noticing the above, the architect recommended **dropping RFC 081
§ 2's no-API-change condition**, so Tier A would become reachable and
6.3.0 would be the first Tier A release. **That recommendation is
withdrawn.**

It is loosening a release gate *because the automation it was designed
for is otherwise unusable* — a structural property traded for
convenience, which inverts the owner's standing position that the less
secure option is the one to avoid. RFC 081's own wording covers the
spirit of it: *"Nothing about a release should ever be changed in order
to reach Tier A; the tier is read off the release, never engineered into
it."* Editing the conditions is not editing a release, so not the
letter — but the motivation was to manufacture the premise, which is the
same error wearing a different hat.

The condition is also load-bearing, not ceremony: an API break reaching
users with a wrong or missing migration note is the confusion the
owner's philosophy ranks highest, crates.io has no unpublish, and that
click is the last human read of a migration entry before it is
permanent.

### What replaces it

[RFC 084](../done/084-assert-the-notes-are-true.md): make CI assert the
notes' machine-checkable claims, so the click's remaining content stops
resting on the architect reviewing the architect's prose. Then the gate
can shrink **on evidence**.

And for the owner's actual request — fewer manual steps — the honest
target is the tier they actually click: make the **Tier B**
confirmation cheap (CI posts the classification and every assertion
result, so they confirm a summary rather than conduct an
investigation), not make Tier A reachable.

**Revisit this RFC only if a Tier A release occurs on its own.**
