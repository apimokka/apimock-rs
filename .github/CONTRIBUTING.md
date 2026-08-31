## ✨ Contributing

We’re happy to receive feedback, bug reports, and questions via GitHub Issues.  
Pull requests are also welcome — though please note that we may not always be able to accept them.

This project is maintained as a labor of love. We welcome community participation, but:

- Issues that are respectful and constructive are appreciated.
- Pull requests are reviewed, but acceptance is not guaranteed.
- We do not engage in long debates or vision disagreements.
- If you have a different direction in mind, please fork freely, provided proper licensing is respected.

Thanks for understanding the scope and spirit of the project.

## Before you open a PR

CI (`.github/workflows/ci.yaml`) runs several checks on every push to
`main` and every pull request, and all of them are required to merge.
Reproduce the core ones locally before pushing:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo check --workspace   # on the toolchain pinned as rust-version in Cargo.toml
cargo audit               # requires: cargo install cargo-audit --locked
cargo update --workspace --locked
cargo package --workspace # skipped in CI when the current version is already on crates.io
mdbook build docs         # requires: mdbook, mdbook-mermaid
```

`cargo test --workspace --lib` is a fine fast-feedback command while
you're iterating, but it is **not** the gate — it runs a subset of the
suite. `cargo test --workspace` (no `--lib`) is what CI actually checks.

`cargo audit` also runs on a weekly schedule, independent of any push or
pull request. **A scheduled run turning red with no new commit is
correct behaviour, not a broken gate** — it means a vulnerability
advisory was published against a dependency this project already uses,
which is new information about existing code, not about your change.

## If you change a crate's public API

RFC 039's `public-api` job fails when a crate's public surface changes
without its checked-in baseline (`crates/<name>/public-api.txt`)
changing in the same commit — additive changes included, not just
removals. Update the baseline in the commit that changes the API; don't
auto-generate it reflexively without reading the diff, since the diff
*is* the review artefact.

```sh
rustup install nightly-2026-08-29   # or whatever ci.yaml currently pins
cargo +nightly-2026-08-29 install cargo-public-api --locked
cargo +nightly-2026-08-29 public-api -p <crate> -s > crates/<crate>/public-api.txt
```

Requires a nightly toolchain (`cargo-public-api` builds rustdoc JSON) —
unrelated to `rust-version`/the `msrv` job above, which stays the
authority on what the crates support to build.

## Version bumps

Use `./version.sh --update <version>` to bump the version — it updates
the workspace manifest and every npm package (including the
`optionalDependencies` platform-binary pins) together and verifies the
result. Do not hand-edit version fields individually.

## Cutting a release

See [`RELEASING.md`](../RELEASING.md) at the repository root — version
bump, tagging, the draft-Release review point, and registry publishing
are covered there, not here.
