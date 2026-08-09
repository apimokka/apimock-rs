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
2. **CHANGELOG.md.** Add a `## [X.Y.Z] - YYYY-MM-DD` section before
   tagging. `create-draft-release` extracts this section verbatim as the
   Release notes and **fails the build phase if it's missing** — there
   is no draft-with-empty-notes fallback, by design (RFC 044 § 4.4).
3. **Commit and push to `main` first.** The tag must point at a commit
   already on `main` with CI green — the release gates re-run the same
   checks at tag time (see below), but there's no reason to discover a
   red gate for the first time during a release.
4. **Tag.** `git tag X.Y.Z && git push origin X.Y.Z` — no `v` prefix.
   `version-consistency-check` compares the tag literally against
   `[workspace.package].version`; a mismatched tag fails immediately,
   before anything is built.

**No prerelease version is releasable today.** `X.Y.Z-rc.1` and the like
will fail at `cargo build --locked`, not at the gates: the internal pins
`apimock-{config,routing,server} = { version = "5" }` are caret
requirements, and a caret requirement never matches a prerelease
version. Established empirically during RFC 044's live test with both
`0.0.0-rfc044-test` and `5.16.1-rfc044-test` — the major version is not
what causes it. Cutting an RC would first require changing those pins to
something prerelease-inclusive.

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
state). If it fails partway, re-running the same command is safe:
already-published crates return "already uploaded" rather than erroring
the whole run, since cargo checks the index before each publish.

Authentication is [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
(OIDC, no stored token) — the same class of mechanism npm already uses
above. This requires Trusted Publishing to be enabled **per crate** on
crates.io's own project settings; that's a registry-side configuration
this repository can't express or verify. If it isn't enabled for one of
the four crates, `crates-io-publish` fails with an authentication error
for that crate specifically — check crates.io's settings for it before
re-running.

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
