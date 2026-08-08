# Build and test locally

```sh
git clone https://github.com/apimokka/apimock-rs.git
cd apimock-rs
cargo build --workspace
cargo test --workspace
```

No `rust-toolchain.toml` — any stable toolchain at or above the pinned
MSRV works day to day. The MSRV itself, `1.91.0`
(`Cargo.toml` `[workspace.package]`), is what CI actually checks; see
[The quality gates](./the-quality-gates.md).

## Running the tests

**`cargo test --workspace` is the gate — not `--lib`.** The difference
is large: `--lib` runs 212 tests (the four crates' unit tests only);
the full `--workspace` command runs all 409, adding the integration
suites under `crates/apimock/tests/` — 140 tests in `tests/server.rs`
alone. `--lib` is a fine fast-feedback loop while iterating, but it
silently skips just under half the suite, so it is never the check
that decides whether something passes.

No network access is needed. TLS-related tests generate their own
throwaway self-signed certificate at test time (`rcgen`, via
`crates/apimock/tests/util/tls.rs`) rather than fetching one.

## Building the docs site

Not required for working on the Rust workspace — only if you're editing
`docs/src/`. Needs `mdbook` and the `mdbook-mermaid` preprocessor
(`docs/book.toml`):

```sh
cargo install mdbook mdbook-mermaid
cd docs && mdbook build
```

`.github/workflows/docs.yaml` deploys `docs/book/`'s output on every
push to `main` — there is no staging step, so a change that leaves the
site incoherent (a broken link, a `SUMMARY.md` entry with no page) goes
live as soon as it merges. Build locally before pushing a docs change.
