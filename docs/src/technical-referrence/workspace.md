# Workspace

## Workspace layout (5.0.0+)

Starting with 5.0.0, apimock is organised as a Cargo workspace:

| crate | responsibility |
| --- | --- |
| [`apimock-config`](crates/apimock-config) | TOML config model — load, validate, resolve relative paths. Defines the stage-1 edit / snapshot API types a GUI can depend on. |
| [`apimock-routing`](crates/apimock-routing) | Rule-set definitions, request matching, read-only view types for GUI tooling. |
| [`apimock-server`](crates/apimock-server) | HTTP(S) listener, request dispatch, Rhai middleware compilation, response building. |
| `apimock` (workspace root) | CLI binary + thin façade library. Re-exports the three member crates under `apimock::config`, `apimock::routing`, `apimock::server`. |

End users installing via `cargo install apimock` or `npx apimock` see no difference from 4.8.0. Library consumers migrating from 4.x paths can find the mapping in [CHANGELOG.md](https://github.com/apimokka/apimock-rs/blob/main/CHANGELOG.md).
