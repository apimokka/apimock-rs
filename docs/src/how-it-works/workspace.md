# The workspace and its crates

[Architecture](./architecture.md) covers what each crate is responsible
for and how they depend on each other. This page covers the workspace
itself — the mechanics that hold the four crates together as one
project.

## The root `Cargo.toml` is workspace-only

It has no `[package]` section of its own — only `[workspace]`,
`[workspace.package]`, `[workspace.dependencies]`, and the build
profiles. Its own header comment explains why (`Cargo.toml:1-24`): until
`5.1.1`, the `apimock` façade's package metadata lived in this same
file, mixing workspace-wide concerns with one crate's own. `5.1.1`
split the façade out into `crates/apimock/`, leaving this file
responsible only for:

- listing the four workspace members;
- `[workspace.package]` — version, edition, MSRV, licence, and the rest
  of the metadata every crate inherits, so they can't drift apart;
- `[workspace.dependencies]` — every shared external dependency's
  version, pinned once;
- the release/dev build profiles, which only take effect from the
  workspace root regardless of which member is being built.

## Why a façade crate at all

`cargo install apimock` and `npx apimock` both need a crate literally
named `apimock`. Splitting the implementation into responsibility crates
(`apimock-config`, `apimock-routing`, `apimock-server`) but keeping a
thin façade that re-exports them means the install story stays simple —
users keep typing the same name they always have, regardless of how the
implementation is organised underneath (`Cargo.toml:18-24`).

## Version and dependency pinning

All four crates share one version number (`5.15.0` as of this page),
one edition (`2024`), and one MSRV (`1.91.0`) via
`version.workspace = true` / `edition.workspace = true` /
`rust-version.workspace = true` in each crate's own manifest. `./version.sh --update`
is what changes this (see [The quality gates](../contributing/the-quality-gates.md)).

External dependencies are pinned once, in `[workspace.dependencies]`
(`Cargo.toml:49-81`), and each crate's own `[dependencies]` pulls the
version from there rather than stating its own — so two crates can
never end up depending on different versions of the same external
crate. The internal crates (`apimock-config`, `apimock-routing`,
`apimock-server`) are declared the same way, as path dependencies
pinned to the workspace version (`Cargo.toml:52-54`).

## Build profiles

Set once, at the workspace root, because Cargo profile sections only
take effect from there:

```toml
[profile.release]     # shrink executable size
opt-level       = "z"
lto             = true
strip           = true
codegen-units   = 1

[profile.dev]         # to reasonably improve productivity
opt-level       = 1
lto             = false
incremental     = true
```

The release profile trades build time for a smaller binary
(`opt-level = "z"`, full LTO, stripped symbols, single codegen unit) —
sensible for a tool distributed as a downloadable executable. The dev
profile does the opposite: light optimisation and incremental
compilation, for faster local iteration.
