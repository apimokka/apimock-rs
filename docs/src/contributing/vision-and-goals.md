# Vision and goals

## Vision

A developer-friendly, sleek, functional HTTP(S) mock server that
doesn't require complicated configuration, but accepts rich
customisation around routing when you need it.

Designed around:

- **Easy setup.** A single small executable; config-less mode works
  out of the box.
- **Performance.** Fast to start, light on memory — see
  [Design notes](../how-it-works/design-notes.md).
- **Cross-platform support.**

## Goals

### 1. Basic

- File-based routing needs no configuration at all.
- `.json`, `.json5`, and `.csv` files are all served as JSON.

### 2. Customisation

- Rule-based routing for conditional responses.
- Per-response or per-rule-set delay
  (`respond.delay_response_milliseconds`) to simulate a slow backend.
- Custom HTTP status codes via `respond.status`.

### 3. Dynamic processing

- Multiple responses for the same URL path, chosen by header, body
  content, or [response strategy](../guides/vary-the-response-for-one-path.md).
- Middleware as Rhai scripts, for cases rule-based matching can't
  express.

### 4. Safe and observable usage

- Config validation — missing files, unreachable rules — via
  `apimock validate`.
- Startup logs print every loaded rule set and rule.
- Request headers and body are logged when
  [`log.verbose`](../reference/apimock-toml-root-settings.md) is
  enabled.
- An integration test suite backs the server's behaviour — see
  [Build and test locally](./build-and-test-locally.md).

### 5. Embedding

- The `spawn` Cargo feature offers an alternate entry point for running
  `apimock` as a subprocess, forwarding its log output to the parent
  process over a `tokio::sync::mpsc` channel — see
  [Architecture](../how-it-works/architecture.md).
