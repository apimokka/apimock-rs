# Implementation Handoff — published documentation with links that do not resolve

**Source.** Pre-6.0.0 audit residual (`mdbook-linkcheck`); ROADMAP
findings 2026-08-30 (broken intra-doc links; the `docs` job's
unauthenticated API call).
**Milestone.** 6.x. **Not blocking.**
**Baseline.** `main` @ `eb79803` (plus whatever has merged since —
check).

**Self-contained.** Three items on the same two jobs. Two share a
theme: **published documentation contains links that do not resolve,
and nothing checks.** The third is why one of those jobs is unreliable.

---

## 1. mdBook builds do not check content integrity

The `docs` job in `ci.yaml` was added before 6.0.0 to catch a broken
docs build on a pull request. It does that. **It does not catch a
broken document**, and the dev team who added it established exactly
that by trying to break it three ways — two of which mdBook did not
consider errors at all:

| Break | mdBook |
|---|---|
| Unclosed code fence at EOF | **No error** — renders as one code block |
| `SUMMARY.md` entry for a page that does not exist | **No error** — silently creates a stub and exits 0 |
| Invalid `book.toml` | Fails, exit 101 |

I reproduced the second one myself: appending a `SUMMARY.md` line for a
missing page exits 0 and writes a stub into `src/`.

**Also unchecked: internal anchors.** A `#heading-anchor` link that
resolves to nothing is invisible to `mdbook build`. One survived
undetected in `return-errors-and-status-codes.md` until a human read
the paragraph during RFC 065.

**Add `mdbook-linkcheck`** to the `docs` job. It covers both cases —
missing pages and unresolvable anchors.

> **Expect it to find existing breakage.** That is the point, but it
> means the job may be red on its first run. **Fix what it finds if the
> fixes are unambiguous** (a typo'd anchor, a moved page). **If it
> reports something where the right fix is a judgement call — a link to
> a page that should exist but does not — report it rather than
> inventing content.**
>
> If the total is large enough that fixing it is its own task, say so
> and stop; a half-fixed set plus a red gate is worse than a clear
> report.

## 2. Nothing runs `cargo doc`, and there are 16 broken intra-doc links

Generating RFC 039's public-API baseline surfaced **16
`rustdoc::broken_intra_doc_links` warnings in `apimock-routing`** —
links like `` [`Equal`] `` in `body_operator.rs` / `strategy.rs` doc
comments resolving to no item in scope.

They are harmless to the build, correctly outside RFC 039's gate (that
job discards stderr, rightly — they are not public API), and they went
unseen **because no job in this project runs `cargo doc`.**

These are broken links in documentation published to docs.rs for four
crates.

**Add a `cargo doc --no-deps` check.** Whether warnings become errors
(`RUSTDOCFLAGS="-D warnings"`) is your call — but if you do not fail on
them, say how the job is expected to surface them, because a warning
nobody reads is what produced 16 of these.

Fix the 16 while you are there if they are unambiguous; report any that
are not.

## 3. A required job fails for reasons unrelated to the commit

Both `ci.yaml`'s `docs` job and `docs.yaml` install mdBook by
`curl`-ing `https://api.github.com/.../releases/latest`
**unauthenticated**, then `jq`-ing the asset URL.

It rate-limited during the post-6.0.0 work: `jq: error … Cannot iterate
over null` — nothing to do with the commit. The dev team diagnosed it
correctly and reran, which is the problem: **a required job that fails
for unrelated reasons trains people to rerun rather than read.**

Options, in the order I would consider them:

- **Pin the mdBook and mdbook-mermaid versions** instead of resolving
  "latest" at run time. Removes the API call entirely, and makes the
  docs build **reproducible** — which resolving "latest" on every run
  currently prevents. A docs build that silently changes tool version
  between runs is its own small hazard.
- Authenticate the call with the workflow's own `GITHUB_TOKEN`.
- Cache the binaries.

**Pinning is my recommendation**, for the reproducibility argument as
much as the rate limit. **Both files need it** — `ci.yaml` and
`docs.yaml` have the same call, and fixing one leaves the other.

## 4. Not in scope

- Restructuring the documentation, or writing new pages.
- `release-publish.yaml` or anything on the publish path.
- Changing what any page says, beyond repairing links.

## 5. Acceptance

- [ ] `mdbook-linkcheck` runs in the `docs` job and **fails on a
      deliberately broken link** — prove it fires, do not just show
      green
- [ ] A `SUMMARY.md` entry for a missing page is now caught (this is
      the case that silently stubs today)
- [ ] `cargo doc` check in place; the 16 `apimock-routing` warnings are
      fixed or reported
- [ ] Both `ci.yaml` and `docs.yaml` no longer depend on an
      unauthenticated `api.github.com` call at build time
- [ ] `mdbook build docs` still clean; `cargo test --workspace`, `fmt`,
      `clippy -D warnings`
- [ ] CI green before merge, including any job you add or change

## 6. Report back

`.git-exclude/review-request/post-6-0-0-docs-integrity/`, including
what the link checks found — **especially anything you decided not to
fix, and why.**
