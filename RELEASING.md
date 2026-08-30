# Releasing

How a release is cut, since RFC 044 made most of it CI's job. This
covers what a release manager still does by hand and what to do when a
step fails. For everyday contribution gates (fmt/clippy/test/etc.), see
[`.github/CONTRIBUTING.md`](.github/CONTRIBUTING.md) — this document
does not repeat that list.

## The flow

```
you:  ./version.sh --update X.Y.Z, update CHANGELOG.md, commit, push main
you:  git tag X.Y.Z && git push origin X.Y.Z        ← the only release trigger
CI:   version-consistency-check, quality-gate
CI:   create a DRAFT Release, notes from CHANGELOG.md
CI:   build 5 targets, attach every asset to the draft
you:  open the draft on GitHub, check it, click "Publish"
CI:   npm publish (3 platform packages, then the core package)
CI:   cargo publish --workspace (4 crates, dependency order)
CI:   verify published artifacts against the Release assets
```

The tag push is the only thing you trigger directly. Everything from
"create a DRAFT Release" onward is `release-executable.yaml`
(`.github/workflows/release-executable.yaml`); everything from
"click Publish" onward is `release-publish.yaml`
(`.github/workflows/release-publish.yaml`). The draft is not visible to
anyone outside the repository until you publish it — nothing is public,
and nothing is published to either registry, until that click.

## Before tagging

1. **Version bump.** `./version.sh --update X.Y.Z`. This updates
   `[workspace.package].version` in `Cargo.toml`, the internal crate
   pins (**set to the exact version** — `"6.0.0"`, not a major-only
   `"6"`; a major-only caret pin never matches a prerelease, which is
   why every RC and alpha used to fail at `cargo build --locked`),
   `Cargo.lock`,
   and all four npm manifests including the `optionalDependencies`
   platform-binary pins — then verifies every surface agrees. If it
   exits non-zero, **stop and fix the script's target, don't hand-edit**
   what it missed; a hand-edited version is exactly the kind of drift
   `version-consistency-check` exists to catch later, at the worst
   possible time.

   **`version.sh` stages what it edits.** To abort a bump,
   `git checkout -- .` **is not enough** — it restores from the index,
   which already holds the new version, so it looks like it worked and
   didn't. Use `git restore --source=HEAD --staged --worktree .`.
   Found 2026-08-27 while reverting a test bump during the 6.0.0
   pre-cut audit.
2. **Remove the `main` development notice from `README.md`** if it is
   present. `README.md` is `readme = "../../README.md"` in the `apimock`
   manifest — it *is* the crates.io landing page — so a notice saying
   "`main` is the 6.0.0 development line" would ship to crates.io as
   part of the release it is warning people away from.

   Put it back after tagging. It exists so someone browsing the
   repository on GitHub does not mistake `main` for a released version.

3. **CHANGELOG.md.** Add a `## [X.Y.Z] - YYYY-MM-DD` section before
   tagging. `create-draft-release` extracts this section verbatim as the
   Release notes and **fails the build phase if it's missing** — there
   is no draft-with-empty-notes fallback, by design (RFC 044 § 4.4).
4. **Push first, and confirm CI is green *on the commit you will tag*.**
   Normally that means `main`. The release gates re-run the same checks
   at tag time (see below), but there's no reason to discover a red gate
   for the first time during a release.

   **Also confirm the `package` job actually ran, rather than
   skipping.** It compares `[workspace.package].version` against
   crates.io and skips `cargo package --workspace` when that version is
   already published — correct between releases, since the check would
   fail spuriously there. But if the crates.io query is *inconclusive*
   (network, rate limit) it **also skips, and the job still reports
   green** — deliberately, to avoid a false red on ordinary pushes. On
   the release path the version has just been bumped and is not yet
   published, so the job must run; a skip there means the packaging
   check silently did not happen, at the one moment it exists for. The
   job logs which branch it took: look for *"is not yet on crates.io;
   running cargo package --workspace"*.

   **Check which commit the green run belongs to**, not just that a
   green run exists. This step used to read "already on `main` with CI
   green", which v5.19.0 showed to be unmeetable for a release cut from
   a branch — and worse, satisfiable by mistake, since `main`'s green run
   at an unrelated commit looks identical in a run listing.

   **Releasing from a branch** (see § below) is now covered by
   `ci.yaml`'s `release/*` trigger. If you ever cut from a branch outside
   that pattern, CI will not run on it at all: use
   `gh workflow run ci.yaml --ref <branch>` and wait for it, rather than
   relying on the tag-time gates alone.
5. **Tag.** `git tag X.Y.Z && git push origin X.Y.Z` — no `v` prefix.
   `version-consistency-check` compares the tag literally against
   `[workspace.package].version`; a mismatched tag fails immediately,
   before anything is built.

**Prereleases are releasable — fixed 2026-08-17.** `X.Y.Z-alpha.N`,
`-beta.N` and `-rc.N` all work: `./version.sh --update` writes the
**exact** workspace version into the internal pins, and a caret
requirement containing a prerelease opts into prerelease matching.

Until then it did not work at all. The pins were major-only (`"5"`, i.e.
`^5`), and a caret requirement never matches a prerelease — so every
RC, alpha and beta failed at `cargo build --locked` rather than at the
gates. Established empirically during RFC 044 with both
`0.0.0-rfc044-test` and `5.16.1-rfc044-test`; the major component was
never what caused it. Verified fixed by bumping to `6.0.0-alpha.1` and
building `--locked` before reverting.

## What each release gate checks

Both re-run at tag-push time, independent of whatever `ci.yaml` already
did on `main` — a release is triggered by a tag, which pull-request
branch protection never covered, so nothing upstream is trusted as a
substitute.

- **`version-consistency-check`** — the tag, `[workspace.package].version`,
  every npm manifest's `.version`, and every `optionalDependencies` pin
  all have to read the same string. Most likely failure: forgot to run
  `version.sh`, or hand-edited something it would have caught.
- **`quality-gate`** — `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test --workspace`. Same three checks `ci.yaml` runs on every
  push; see `.github/CONTRIBUTING.md` for what each one means. Failing
  here means the tagged commit wasn't actually green — re-tag after
  fixing, following the recovery steps below.

If either fails, no draft Release is created and nothing downstream
runs.

## The draft — what to check before clicking Publish

- **Release notes** are the CHANGELOG section verbatim. If they read
  wrong, the CHANGELOG entry was wrong when tagged — the fix is a new
  release, not editing the draft's notes (the notes are meant to match
  what's actually in `CHANGELOG.md` on the tagged commit).
- **All five assets present** — `apimock@Linux-aarch64-musl-X.Y.Z.tar.gz`,
  `Linux-x64-gnu`, `Linux-x64-musl` (`.tar.gz`), `macOS-aarch64`,
  `Windows-x64` (`.zip`). If the draft is visible but short an asset,
  the `build` matrix has a failed leg — check its run before publishing;
  publishing triggers npm/crates.io regardless of whether every target
  succeeded.

Publishing is the only human confirmation point in the whole flow.
Nothing before it is public; nothing after it is reversible.

## npm publish order

`npm-platforms-publish` (`release-publish.yaml`) publishes the three
platform packages (`@apimock-rs/bin-linux-x64-gnu`,
`@apimock-rs/bin-darwin-arm64`, `@apimock-rs/bin-win32-x64-msvc`) as a
matrix, then `npm-core-publish` publishes the core `apimock` package,
which declares all three as `optionalDependencies`.

**`npm-core-publish` has failed once, on 6.0.0** — a partial publish:
the three platform packages were live, the core package was not, and
`crates-io-publish` never ran (it is downstream of `npm-core-publish` —
see below). Root cause: npm silently omits an `optionalDependencies`
entry it cannot yet resolve, and `npm-core-publish` starts generating
its lockfile immediately after `npm-platforms-publish` reports success,
with no wait for npm's own propagation delay. The gap between the last
platform publish completing and the core job starting has been as low
as 2 seconds on a real release; a rerun 113 seconds later succeeded. As
of this writing that gap is still unguarded — see the open item this
section exists to work around until it is closed.

### How to tell what actually published

**Check the registry, not the job log** — the log tells you what was
*attempted*, not what a client sees. For all four npm packages:

```sh
npm view @apimock-rs/bin-linux-x64-gnu version
npm view @apimock-rs/bin-darwin-arm64 version
npm view @apimock-rs/bin-win32-x64-msvc version
npm view apimock version
```

Compare each against the tag. A package missing the tagged version
never published; one showing it did, regardless of what the job's exit
code said.

### Recovery

**Used successfully on 6.0.0:** `gh run rerun <run-id> --failed`. This
re-runs only the failed job and its downstream dependents — the three
platform packages that already published are **not** re-attempted,
which matters, because re-publishing an already-published npm version
fails outright rather than being treated as a no-op (the exact error
text has not been captured here — not re-derived by deliberately
re-publishing a live package just to quote it). Find the run id with
`gh run list --workflow=release-publish.yaml`.

Do **not** attempt a local `npm publish` for the core package as a
substitute — like crates.io (see below), this repository authenticates
to npm via a trusted-publisher record (OIDC via GitHub Actions, no
stored token), the same class of mechanism described in § "Publisher
records bind to a workflow *filename*" below. A local `npm publish`
cannot present that identity.

### A partial npm publish is not self-healing

`crates-io-publish` `needs: npm-core-publish` — so an npm-core-publish
failure means **crates.io never runs at all**, silently. A release
"finishing" with only three of four npm packages and zero crates
published looks, from the Actions run list, like a single red job deep
in the pipeline — check what actually reached each registry (this
section, and § "If crates.io publishing fails partway" below) rather
than trusting the shape of the failure to be obvious from the log.

## crates.io publish order

`cargo publish --workspace` (a single command, `crates-io-publish` job
in `release-publish.yaml`) resolves the four crates' dependency order
itself and waits for each to land on the index before publishing the
next — `apimock-routing` → `apimock-config` → `apimock-server` →
`apimock`. Verified in the real v5.15.0 publish (exit 0, no partial
state).

**If it fails partway, re-running is not automatic.** An earlier version
of this document claimed already-published crates are skipped with an
"already uploaded" notice. They are not — v5.16.0 disproved it on the
first real run:

```
error: crate apimock@5.16.0 already exists on crates.io index
```

cargo exits 101 and the job fails. So a partial publish has to be
finished by hand — but **since 2026-08-17 that is no longer a single
command.**

### If crates.io publishing fails partway

With *"Require trusted publishing for all new versions"* on, a local
`cargo publish` authenticates with an API token and is therefore
**rejected**. The obvious recovery — running `cargo publish -p <crate>`
from a maintainer's machine — cannot work, and re-running the whole
workflow only fails again on the first crate that already exists.

The procedure is:

1. Work out which crates actually landed. **Check crates.io, not the job
   log** — the log tells you what the job attempted.
2. On crates.io, temporarily switch *"Require trusted publishing"*
   **off** for the crates still to publish.
3. Publish the remainder individually, in dependency order:
   `apimock-routing`, `apimock-config`, `apimock-server`, `apimock`.
4. **Switch it back on.** This is the step that gets forgotten, and
   forgetting it silently returns the project to the state the switch
   exists to prevent.

Deliberately more steps than before. The alternative was leaving a
token-shaped hole open permanently against a failure that has not yet
happened once.

Authentication is [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
(OIDC, no stored token) — the same class of mechanism npm already uses
above. This requires Trusted Publishing to be enabled **per crate** on
crates.io's own project settings; that's a registry-side configuration
this repository can't express or verify. If it isn't enabled for one of
the four crates, `crates-io-publish` fails with an authentication error
for that crate specifically — check crates.io's settings for it before
re-running.

**Status as of v5.18.0: the automated path is proven, and trusted
publishing is now *required*.** `crates-io-publish` published for the
first time in v5.17.0 and again in v5.18.0. RFC 047's `verify-published`
jobs failed on their first outing (a missing `mkdir`) and ran clean in
v5.18.0, which was the first release where every job in this workflow
went green end to end.

Accordingly, crates.io's *"Require trusted publishing for all new
versions"* was switched **on** for all four crates on 2026-08-17. A
leaked or stale API token can no longer publish a version nobody
authorised.

**Read § "If crates.io publishing fails partway" before relying on any
manual `cargo publish`** — that switch is what makes the old advice
wrong.

## Releasing an older line: three "latest" pointers to override

When a newer line already exists, **every mechanism that tracks "the
newest thing" will try to point at the release you are cutting**, and
each has to be overridden separately. Cutting 4.8.1 and 4.8.2 after
5.19.1 hit all three.

| Pointer | Default | Override |
|---|---|---|
| npm `latest` dist-tag | moves to the published version | `npm publish --tag v4x` |
| GitHub Release "Latest" badge | `make_latest=true` | `gh release create … --latest=false` |
| crates.io | none — resolves by semver requirement | nothing to do |

**npm refuses outright**, which is the friendly case: publishing 4.x
without `--tag` fails with *"Cannot implicitly apply the `latest` tag
because previously published version 5.19.1 is higher"*. You cannot get
it wrong silently.

**GitHub does it silently**, which is the one that bites. `gh release
create` marks the new release "Latest" with no warning, so the
repository's front page and the *"Download the latest release"* links in
`README.md` and the docs start pointing at the older line. It was missed
on both 4.8.1 and 4.8.2 and corrected by hand afterwards.

The dist-tag must not be a valid semver range: `v4` is rejected
(*"Tag name must not be a valid SemVer range"*) because it parses as
`>=4.0.0 <5.0.0-0`. `v4x` is what the 4.x line uses.

### Checklist when cutting any 4.x release

1. `gh release create <tag> --latest=false …`
2. Confirm afterwards: `gh api /repos/apimokka/apimock-rs/releases/latest --jq .tag_name`
   must still report the newest 5.x/6.x tag.
3. Confirm npm: `npm view apimock-rs dist-tags` — `latest` on the newest
   line, `v4x` on 4.x.

## Publisher records bind to a workflow *filename* — check this first

**This has blocked three releases.** v5.16.0's npm E404, and both
registries again during the 4.8.1 security release. It presents as a
permissions error and reads like a credentials problem, which is why it
costs an hour every time.

A trusted publisher record on npm or crates.io binds **repository +
workflow filename**, recorded **per package/crate**. A workflow with a
different filename is a different publisher, however similar its
contents.

| Line | Publish workflow | Covered by the records? |
|---|---|---|
| 5.x / 6.x | `release-publish.yaml` | Yes — this is what the records name |
| 4.x | `release-executable.yaml` | **No** — v4 publishes from its own build workflow |

The 4.8.1 release needed the records temporarily repointed at
`release-executable.yaml`, then reverted. **While repointed, the 5.x/6.x
line cannot publish**, and vice versa. Only one line can publish at a
time under this arrangement.

### How to recognise it

- **npm:** `404 The requested resource '<pkg>@<version>' could not be
  found or you do not have permission to access it` — on a package that
  plainly exists. A 404 on publish means permission, not absence.
- **crates.io:** `403 Forbidden: New versions of this crate can only be
  published using Trusted Publishing`.

Neither message names the workflow, which is the whole difficulty.

**A misleading near-miss:** npm's *"Cannot implicitly apply the `latest`
tag because previously published version X is higher"* fires **before**
authentication — it only reads public metadata. Getting that error does
**not** prove the publisher record is correct; fix the tag and the real
permissions error appears next.

### The permanent fix, not yet done

Give the 4.x line a publish workflow named `release-publish.yaml` too,
so one record per package covers both lines and nothing needs
repointing. Whether that works depends on whether the records pin a
**ref** as well as a filename — if they do, a branch-scoped record is
needed instead. Check the record's fields before doing the rename.

## Trusted publishing binds to the workflow *filename*

Both registries identify the publisher as a **repository plus a workflow
filename**, recorded **per package**, on the registry's own settings —
not in this repository. Renaming a workflow file, or moving a publish job
into a different file, silently invalidates every one of those records.

This cost v5.16.0 four failed publish attempts, so it is worth stating
exactly how it presents. npm answers an unidentified publish with:

```
npm error code E404
npm error 404 Not Found - PUT https://registry.npmjs.org/@apimock-rs%2fbin-linux-x64-gnu
npm error 404  The requested resource '...' could not be found or you do
               not have permission to access it.
```

That reads like a missing or misnamed package. It is not — it is an
authentication failure. **The reliable signal is the provenance line.** A
successful trusted publish prints, immediately before `+ package@version`:

```
npm notice publish Signed provenance statement with source and build information from GitHub Actions
```

If that line is absent, npm never obtained an identity and published
anonymously, whatever the error text says.

The records are **per package**, which is the part most easily missed:
this project has four npm packages and four crates, so **eight separate
records**, each needing the same edit. Fixing the one package named in
the error is not enough — the three platform packages publish first, and
the run dies before ever reaching the core package.

So: if a publish job is ever renamed or moved, update all eight records
before the next release, and treat the first release afterwards as
unproven until it goes green.

## Releasing from a branch

Used for the first time by **v5.19.0** (RFC 054), because `main` carried
breaking work that the release had to exclude.

```sh
git checkout -b release/X.Y <base-tag>
```

Everything else is unchanged — the tag-push trigger does not care which
branch a tag lives on, `version-consistency-check` reads the *tag's*
tree, and `create-draft-release` extracts notes from the CHANGELOG at
the tag. `main` need not contain the release's CHANGELOG section at all.

Two things that are easy to get wrong:

- **The branch's baseline is its base tag's, not `main`'s.** v5.19.0's
  was 425 tests, while `main` was at 437. A test count compared against
  the wrong baseline looks like a regression or a windfall, and is
  neither.
- **Merge the branch back to `main` afterwards**, or the release's work
  exists only on a tag. The version bump comes with it, which is correct:
  `main` then reads the released version until the next bump.

## Post-publish verification

After both registries are published to, `verify-published` and
`verify-crates-io` (`release-publish.yaml`, RFC 047) pull each artifact
back down — the npm platform packages via `npm pack`, the Release asset
via `gh release download`, both hashed and compared; each crate checked
against crates.io's public API — and fail loudly on any mismatch or
missing version. This exists because it has actually happened here:
npm shipped packages labelled 5.9.0–5.10.0 containing a 4.6.9 binary for
months, undetected, because nothing checked what actually landed.

**A `verify-*` failure means the artifact is already public and needs
investigating — not re-running.** By the time these jobs run, publishing
already happened; re-running them changes nothing about what's on the
registry. Treat a failure the same as discovering the problem by hand:
check what's actually live (`npm pack`, crates.io's site) against what
the Release asset says, then decide whether a yank/deprecate is
warranted (see below).

## Recovery paths, and their limits

Both registries are effectively **append-only**. Neither of the
following removes a bad release; both only stop it from spreading
further.

- **`cargo yank --version X.Y.Z <crate>`** — marks the version so it
  can't become a *new* dependency (`cargo add` and fresh `Cargo.lock`
  resolutions skip it), but projects that already pinned it in
  `Cargo.lock` keep building against it, and the crate's source stays
  downloadable forever. Yank each of the four crates individually if
  needed; yanking `apimock` doesn't yank the libraries under it.
- **`npm deprecate @apimock-rs/... "message" `** — attaches a warning
  shown on install, nothing more. It does not stop installs.
  `npm unpublish` exists but is time-limited (72 hours) and blocked once
  anything depends on the version, so it is not a real option for this
  project once a release is more than a few days old.
- **The GitHub Release itself** can be deleted or edited freely — that
  part is not append-only, but doing so doesn't touch either registry.

There is no rollback that un-publishes a package version. The realistic
recovery for a bad release is a new, corrected version, published the
normal way, with the bad one yanked/deprecated so it stops being
resolved by default.

## If the draft flow itself misbehaves

If `create-draft-release` or `build` fails after the tag is already
pushed, deleting the tag and Release and re-pushing the same tag
re-triggers the whole build phase cleanly — nothing about the flow is
order-sensitive to a retry, since the draft doesn't exist until this
phase creates it. If a **publish** job fails after you've clicked
Publish, treat it as a partial release: check which of npm/crates.io
actually succeeded (see the run logs) before deciding whether a yank/
deprecate is warranted for what did land.
