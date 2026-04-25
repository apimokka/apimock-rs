# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [5.1.0] - 2026-04-26

5.1.0 implements the GUI-facing extension layer specified in the
project's "GUI 向け機能拡張開発指示書". The headline addition is
`apimock_config::Workspace` — an editable façade that lets a future
GUI manipulate apimock configuration through structured commands
without touching TOML directly.

This is **not a breaking change** for existing users. The CLI binary,
the on-disk file formats, and every pre-5.0 import path that 5.0.0
already preserved continue to work unchanged.

### Added

- **`apimock_config::Workspace`** — the new top-level handle a GUI
  uses to edit a loaded configuration:
  - `Workspace::load(path)` reads `apimock.toml` and every file it
    references (sharing `Config::new`'s loader so the running server
    sees the same data).
  - `Workspace::snapshot()` produces a fully-owned `WorkspaceSnapshot`
    suitable for serialisation, IPC, or rendering.
  - `Workspace::apply(EditCommand)` mutates the in-memory model.
  - `Workspace::validate()` runs per-node validation.
  - `Workspace::save()` is reserved for 5.2.0; the placeholder
    returns `SaveError::Inconsistent` so callers see the gap clearly.

- **Stable per-node identifiers (`NodeId`).** Every editable node is
  assigned a v4 UUID at load time. IDs survive `apply()` calls within
  one Workspace instance, including operations that shift positions
  (Delete / Move). A GUI selection set anchored on NodeIds therefore
  remains valid across edits.

- **Eight `EditCommand` variants** matching the spec exactly:
  - `AddRuleSet { path }` / `RemoveRuleSet { id }`
  - `AddRule { parent, rule }` / `UpdateRule { id, rule }` /
    `DeleteRule { id }` / `MoveRule { id, new_index }`
  - `UpdateRespond { id, respond }`
  - `UpdateRootSetting { key, value }` with a typed
    `RootSettingKey` enum (`ListenerIpAddress`, `ListenerPort`,
    `ServiceFallbackRespondDir`, `ServiceStrategy`)

- **Per-node validation.** `validate()` produces a `ValidationReport`
  whose `diagnostics` carry `NodeId` + file + severity + message.
  Apply-time validation runs the same pass so `ApplyResult.diagnostics`
  reflects the post-mutation state. A GUI can render a red underline
  on the offending node directly from the diagnostic's `node_id`.

- **`uuid = "1"` (with `v4` and `serde` features) is now a direct
  dependency of `apimock-config`.**

### Changed

- `view::ReloadHint` reshape: was a 3-variant enum
  (`None / Reload / Restart`) in 5.0.0; is now a struct
  `{ requires_reload, requires_restart }` per spec §9. The
  `apimock-server::control::ReloadHint` enum mirror remains and the
  bidirectional `From` impls have been updated so existing server-side
  pattern matching still works.

- `view::WorkspaceSnapshot` reshape: now `{ files, routes, diagnostics }`
  per spec §4.2 (was `{ root, rule_sets, middlewares, diagnostics }`).
  Each `ConfigNodeView` now carries the six spec-mandated fields
  (`id`, `source_file`, `toml_path`, `display_name`, `kind`, `validation`).

- `view::EditCommand` reshape: was 4 placeholder variants in 5.0.0; is
  now the 8 spec-defined variants, all targeted by NodeId rather than
  positional path.

### Status against the spec

- ✅ §12 Step 1 (Workspace + snapshot) — implemented.
- ✅ §12 Step 2 (EditCommand + apply) — implemented; all 8 variants.
- ✅ §12 Step 3 (validation + diagnostics) — implemented; per-node.
- ⏳ §12 Step 4 (save + diff) — placeholder; planned for 5.2.0.
- ⏳ §12 Step 5 (richer routing snapshot) — placeholder; planned for 5.2.0.

This matches the (A)=3 scope agreed before implementation.

### Tests

`cargo test --workspace --lib` reports 44 passing (was 31 at 5.0.0):
- 19 in `apimock-config` (6 path_util + 13 Workspace tests covering
  every `EditCommand` variant, ID stability across shift / move,
  validation diagnostics, and the three `ApplyError` paths)
- 15 in `apimock-routing` (unchanged from 5.0.0)
- 10 in `apimock` façade (unchanged from 5.0.0)
- 0 in `apimock-server` (none ported when the crate was carved out)

## [5.0.0] - 2026-04-23

5.0.0 splits apimock into a Cargo workspace with three responsibility-
focused library crates behind a thin façade, in preparation for a GUI
front-end that can depend on a stable config + routing API without
reaching into the HTTP runtime.

This is a **breaking change** for library consumers. CLI behaviour,
config file formats, and on-disk layouts are unchanged — `cargo install
apimock` / `npx apimock` work exactly as before.

### Workspace layout

| crate | responsibility |
| --- | --- |
| `apimock-config` | TOML config model — loading, validation, relative-path resolution, and the stage-1 GUI-facing edit / snapshot API shape. |
| `apimock-routing` | Rule-set definitions, request matching, and the stage-1 read-only view types a future GUI binds against. |
| `apimock-server` | HTTP(S) listener, per-request dispatch, Rhai middleware compilation, response building. |
| `apimock` (façade) | CLI entry point, logger installation, `App` composition. Re-exports the three member crates for backwards-compatible `use apimock::routing::...` paths. |

### Added

- **GUI-facing stage-1 view API.** Every type a future GUI will depend
  on is defined with its field shape + rustdoc — ready to be populated
  in stage 2 without further signature churn. Highlights:
  - Routing: `RouteCatalogSnapshot`, `RuleSetView`, `RuleView`,
    `RespondView`, `RouteMatchView`, `RouteValidation`.
  - Config: `WorkspaceSnapshot`, `ConfigFileView`, `ConfigNodeView`,
    `EditCommand`, `EditTarget`, `EditValue`, `ApplyResult`, `SaveResult`,
    `ReloadHint`, `Diagnostic`.
  - Server: `ServerHandle`, `ServerControl`, `ServerState`, `ReloadHint`.
  - All are `#[non_exhaustive]` so stage-2 fills in fields additively.

- **Per-crate error types.** `apimock_routing::RoutingError`,
  `apimock_config::ConfigError`, `apimock_server::ServerError`. Each
  `#[from]`-wraps the layer below, so `?` propagation works unchanged
  in practice; pattern-matching callers can now see which layer the
  failure originated in.

### Changed (breaking)

- Module paths changed for all library consumers. Map:

  | 4.8.0 path | 5.0.0 path |
  | --- | --- |
  | `apimock::core::config::Config` | `apimock::config::Config` or `apimock_config::Config` |
  | `apimock::core::server::routing::rule_set::RuleSet` | `apimock::routing::RuleSet` or `apimock_routing::RuleSet` |
  | `apimock::core::server::Server` | `apimock::server::Server` or `apimock_server::Server` |
  | `apimock::core::error::AppError` | split into the three per-crate errors above |
  | `apimock::core::app::App` | `apimock::App` |
  | `apimock::core::args::EnvArgs` | `apimock::EnvArgs` |

- `ServiceConfig` no longer carries `Vec<MiddlewareHandler>`.
  Compiled Rhai middlewares are now a separate
  `apimock_server::LoadedMiddlewares` value, built from
  `ServiceConfig::middlewares_file_paths` at server startup. This
  removes a cross-layer leak (a config struct must not hold hyper-
  producing runtime objects).
- `Respond::response(...)` method removed. The equivalent free
  function `apimock_server::respond_response::respond_response(...)`
  replaces it. Rationale: the routing crate must stay free of hyper
  body-construction so a GUI can depend on it cheaply.
- `Server::new(AppState)` → `Server::new(Config)`. `AppState` is now
  internal to `apimock-server` and combines `Config` + `LoadedMiddlewares`.

### Not changed

- Binary behaviour (`apimock`, `npx apimock`, `apimock --init`, `apimock -p`, `-d`, `-c`) is byte-identical to 4.8.0.
- Config file formats (`apimock.toml`, rule-set TOML, middleware `.rhai`) are unchanged.
- Performance: both bench suites produce the same numbers as on 4.8.0 (expected — the same code runs, just organised differently).
- Interactive `--init` flow from 4.8.0: unchanged.
- Existing CHANGELOG entries below.

### Migration note

If you were importing from `apimock::core::...` directly, the simplest
migration is to replace the `core::` prefix with the member crate name
(`config::`, `routing::`, or `server::`). The façade re-exports all
three under those names. If you had a `use apimock::core::error::AppError;`,
pick whichever of `ConfigError` / `RoutingError` / `ServerError`
matches the failure you were catching — or use `anyhow::Error` at your
process boundary (that's what the binary does now).

## [4.8.0] - 2026-04-23

4.8.0 focuses on making the project approachable on first contact and on
giving contributors an honest way to track performance. No runtime
dependency additions, no breaking changes to config files or public
library API.

### Added

- **Interactive `apimock --init`.** Prompts for listener IP, port,
  fallback response directory, whether to include a rule-set file, a
  middleware file, and a TLS section, then writes `apimock.toml`
  (templated from your answers) plus the optional companion files.
  - `--yes` / `-y` skips every prompt and uses the 4.7.0 defaults.
  - When stdin is not a TTY (CI, pipes, Docker build), prompts are
    skipped automatically and defaults are used — 4.7.0 non-interactive
    behaviour is preserved exactly.
  - Idempotent: an existing `apimock.toml` still triggers an early
    warn+exit, so repeatable `apimock --init` scripts keep working.
  - Implemented in ~350 lines of plain `std::io` (no `dialoguer` /
    `inquire` / `requestty`) to respect the project's small-footprint
    promise.
- **Benchmarks via criterion 0.8.**
  - `benches/routing.rs` — pure-CPU `RuleSet::find_matched` microbenches,
    parametrised over rule-set sizes 1 / 10 / 100.
  - `benches/response_latency.rs` — end-to-end HTTP latency across five
    response kinds (text, status-only, file-warm, dyn-route fallback,
    404) using criterion's `async_tokio` support.
  - `examples/bench_load.rs` — standalone load sampler that drives the
    server at a target RPS and emits CSV samples of RSS, CPU ticks,
    in-flight requests, and latency every 100 ms. Covers the
    memory / CPU axes that criterion doesn't measure. Reads
    `/proc/self/{status,stat}` directly (no new crates); Linux-only for
    those two columns, latency/throughput columns work everywhere.

### Changed

- `EnvArgs::default()` now reads `--yes` / `-y` in addition to the
  existing flags. No source-compatible breakage — the flag is purely
  additive.
- `Cargo.toml` dev-deps gain `criterion`, `tempfile`, and `log` with the
  `std` feature (needed by benches / example only; release builds
  unaffected).

## [4.7.0] - 2026-04-23

4.7.0 is an internal-cleanup release. Public CLI behaviour and config file
formats are unchanged — but a few library-level signatures changed for
contributors:

- **Typed errors.** Startup-time failures (missing config, unreadable TLS,
  malformed rule set, Rhai compile error, …) now flow through a single
  `apimock::core::error::AppError` enum (powered by `thiserror`) instead of
  `String` / `panic!`. `App::new`, `apimock::new`, `Config::new`,
  `RuleSet::new`, `Server::new`, `MiddlewareHandler::new`,
  `load_certs` / `load_private_key` and `EnvArgs::default` now return
  `AppResult<T>`. The binary entry point uses `anyhow::Result` at the
  process boundary so typos in user config produce a single-line error
  instead of a backtrace.
- **No more `unwrap` / `expect` on hot paths.** The HTTP `accept` loops,
  TLS handshakes, `ParsedRequest` construction, and per-request logging
  all degrade gracefully instead of panicking.
- **Lower nesting.** `Respond::response`, `ParsedRequest::from` and
  `dyn_route_content` are shallower now, with early returns and
  `let … else` in place of long `if let … else` chains.
- **Richer rustdoc.** Key modules and functions now document *why* they
  exist and *why* they're shaped the way they are, not just what they do.

