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

CI (`.github/workflows/ci.yaml`) runs six checks on every push to `main`
and every pull request, and all six are required to merge. Reproduce
them locally before pushing:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo check --workspace   # on the toolchain pinned as rust-version in Cargo.toml
cargo audit               # requires: cargo install cargo-audit --locked
cargo update --workspace --locked
```

`cargo test --workspace --lib` is a fine fast-feedback command while
you're iterating, but it is **not** the gate — it runs a subset of the
suite. `cargo test --workspace` (no `--lib`) is what CI actually checks.

`cargo audit` also runs on a weekly schedule, independent of any push or
pull request. **A scheduled run turning red with no new commit is
correct behaviour, not a broken gate** — it means a vulnerability
advisory was published against a dependency this project already uses,
which is new information about existing code, not about your change.

## Version bumps

Use `./version.sh --update <version>` to bump the version — it updates
the workspace manifest and every npm package (including the
`optionalDependencies` platform-binary pins) together and verifies the
result. Do not hand-edit version fields individually.
