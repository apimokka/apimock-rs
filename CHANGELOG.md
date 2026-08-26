# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.8.2] - 2026-08-26

**Release-process fixes. No functional change** — the binary and library
are byte-for-byte what 4.8.1 shipped.

### Fixed

- **The 4.x line can publish independently of 5.x/6.x again.** Trusted
  publishing records on npm and crates.io bind to a repository *and a
  workflow filename*. This line published from `release-executable.yaml`
  while the records name `release-publish.yaml`, so only one line could
  publish at a time and the records had to be repointed by hand for each
  4.x release — which is exactly what blocked 4.8.1, on both registries.
  The workflow is renamed to `release-publish.yaml`, so one record per
  package now covers both lines.

- **`version.sh` no longer corrupts `Cargo.lock`.** It ran `cargo fetch`
  to refresh the lockfile. Any cargo command that rewrites this lockfile
  re-resolves it, which on this line consolidates two `rand` entries into
  one and removes 43 lines — after which the test suite no longer
  compiles. The script now edits this package's own version line
  directly and verifies that nothing else changed. This bit the 4.8.1
  release and was corrected by hand at the time.

## [4.8.1] - 2026-08-20

**Security release.** Fixes a path-traversal issue in the file-serving
path. See the security advisory linked from the release notes.

### Security

- **Resolved file paths are now confined to the directory they were
  resolved against.** The file-serving fallback built a path from the
  request URL and joined it onto the configured response directory,
  checking only that the result existed — never that it stayed inside
  that directory. A request carrying an un-normalised `..` segment could
  therefore read files outside it.

  The same missing check applied to a rule's `respond.file_path` and to
  a path returned by a Rhai middleware script. Both are
  operator-authored rather than request-derived, so neither was
  reachable by a client, but both could reference locations outside the
  response directory.

  Every resolved path is now canonicalised and checked against the
  canonicalised base directory for its site, enforced at a single point
  so every caller inherits it. Anything outside is refused with a bare
  `404`, indistinguishable from an ordinary not-found. As a second,
  independent control, `..` segments are stripped from the request path
  before file resolution.

  Affected: 4.0.0 through 5.19.0. Also fixed in 5.19.1. **v4 remains
  supported — there is no requirement to change major version.**

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

