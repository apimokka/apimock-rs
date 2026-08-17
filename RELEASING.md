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
   pins (kept at `"5"`, not re-pinned to the exact version), `Cargo.lock`,
   and all four npm manifests including the `optionalDependencies`
   platform-binary pins — then verifies every surface agrees. If it
   exits non-zero, **stop and fix the script's target, don't hand-edit**
   what it missed; a hand-edited version is exactly the kind of drift
   `version-consistency-check` exists to catch later, at the worst
   possible time.
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
