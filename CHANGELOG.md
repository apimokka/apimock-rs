# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

