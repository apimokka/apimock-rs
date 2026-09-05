# Handoff — RFC 081 § 3: assert the draft before it can be published

**Governing RFC.** [081](../../accepted/081-tiered-release-confirmation.md),
accepted 2026-09-06.
**Milestone.** Next release. Not urgent — but the tier rule is already
in force, so until this lands the asset/notes check is back to being
someone's attention, which is what the RFC exists to fix.
**Baseline.** `main`'s head — cut from it. Everything after RFC 081's
own commit is prose.
**Branch.** **Take one.** RFC 080 § 3's carve-out does not obviously
apply (no filesystem or path work), but this touches a **release
workflow**, and a mistake here is only discovered during a live
release. Branch, let CI run, merge green.

> ## 🔒 What you must not touch
>
> RFC 066 § 2 forbids editing **`release-publish.yaml`** — publisher
> records bind to the **filename**, so a rename silently breaks
> publishing. This work is entirely in
> **`release-executable.yaml`**, which has no such binding. Do not
> rename either file, and do not alter what publishes, in what order,
> or with what credentials.

## 1. The one job to add

`release-executable.yaml`'s jobs today:

```
version-consistency-check → quality-gate → create-draft-release → build
```

Add a final job — `needs: [build]`, so it runs after every asset is
attached and before any draft is publishable.

It asserts two things, and fails the build phase if either is wrong:

1. **Exactly the five expected assets are present on the draft**, named
   for the tag:
   `apimock@Linux-aarch64-musl-X.Y.Z.tar.gz`,
   `apimock@Linux-x64-gnu-X.Y.Z.tar.gz`,
   `apimock@Linux-x64-musl-X.Y.Z.tar.gz`,
   `apimock@macOS-aarch64-X.Y.Z.zip`,
   `apimock@Windows-x64-X.Y.Z.zip`.
2. **The release notes are non-empty and byte-identical** to the
   `CHANGELOG.md` section for this tag.

For (1), the names above are the ones 6.1.0's draft actually carries —
verified against it, not transcribed from the docs. Derive them from
the tag rather than hard-coding a version.

For (2), `create-draft-release` already extracts that section to build
the notes; the assertion should compare against the same extraction
rather than reimplementing it. **If you find yourself writing a second
CHANGELOG parser, stop** — two parsers that can disagree is the defect,
not the fix. Factor the existing one out if that is what it takes, and
say so.

## 2. Prove it fires

**A green run proves nothing here.** This job's entire value is failing
when something is wrong, and it will be green on every correct release
forever, which is exactly how a broken assertion hides.

- [ ] **Delete an asset from a draft on a throwaway tag** and show the
      job fails, with output naming the missing asset.
- [ ] **Perturb the notes** (or the CHANGELOG section) and show it
      fails, with output showing the difference.
- [ ] Both on a throwaway tag — `0.0.0-rfc081-test` or similar, deleted
      afterwards. Never on a real release tag.

Quote both failures in the package. RFC 081 § Testing asks for this
specifically.

## 3. The dry-run classification (RFC 081 § Testing)

Classify the last three real releases — **6.1.0, 6.0.0, 5.19.1** — against
§ 2's four tests, and report the results as a table.

Expected: **all three are Tier B.** 6.1.0 fails three of four (it has a
`### Security` section, `apimock-server/public-api.txt` changed, and it
adds limits that refuse previously-accepted requests). If any of the
three classifies as **Tier A**, the tests are wrong and **say so
instead of adopting them** — RFC 081 says its own tests need changing
in that case, and that finding is more valuable than the job.

This is a paper exercise against existing artifacts; no tag or release
is touched.

## 4. Not in scope

- **Automating the classification.** § 2's third test is a judgement.
  A script that guesses it would be worse than a human reading four
  lines. The tier is declared in the release record, by a person.
- **Asserting the tag is signed.** RFC 081's unresolved question 2 —
  offered, not taken. Out of scope; the finding behind it (123 of 169
  tags signed, all 46 unsigned ones between `0.9.0` and `2.9.4`) is
  recorded there if it comes back.
- **Anything in `release-publish.yaml`.**

## 5. Acceptance

- [ ] The job exists, `needs: [build]`, in `release-executable.yaml`
- [ ] Both failure modes demonstrated on a throwaway tag, output quoted
- [ ] No second CHANGELOG parser introduced
- [ ] The three-release dry-run classification reported
- [ ] `release-publish.yaml` untouched; neither workflow renamed
- [ ] Gates green; CI green on all 12 jobs before merge, and `main`'s
      own run re-verified after (RFC 066 Amendment 3)

## 6. Report back

`.git-exclude/review-request/081-tiered-release-confirmation/`, with the
two quoted failures and the classification table.
