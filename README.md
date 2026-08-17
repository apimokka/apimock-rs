# apimock-rs (API Mock)

[![npm](https://img.shields.io/npm/v/apimock-rs)](https://www.npmjs.com/package/apimock-rs)
[![crates.io](https://img.shields.io/crates/v/apimock?label=rust)](https://crates.io/crates/apimock)
[![License](https://img.shields.io/github/license/apimokka/apimock-rs)](https://github.com/apimokka/apimock-rs/blob/main/LICENSE)    
[![Rust Documentation](https://docs.rs/apimock/badge.svg?version=latest)](https://docs.rs/apimock)
[![Dependency Status](https://deps.rs/crate/apimock/latest/status.svg)](https://deps.rs/crate/apimock)
[![Releases Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/release-executable.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/release-executable.yaml)
[![App Docs Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/docs.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/docs.yaml)

![logo](https://raw.githubusercontent.com/apimokka/apimock-rs/main/docs/src/assets/logo.png)

> ### ⚠️ `main` is the 6.0.0 development line
>
> **The latest release is [5.19.0](https://github.com/apimokka/apimock-rs/releases/latest)** — install from
> [crates.io](https://crates.io/crates/apimock) or [npm](https://www.npmjs.com/package/apimock-rs) as below and you get that.
>
> `main` carries **breaking changes** headed for 6.0.0 and is not a
> supported version. If you are migrating, see the
> [migration guide](https://apimokka.github.io/apimock-rs/guides/migrating-to-6-0.html).
> If you are reading this on crates.io or npm, you are looking at a
> released version and this notice does not apply to it.

Drop JSON files into a folder and your API immediately exists.

## Overview

apimock-rs is an HTTP(S) mock server built in Rust: point it at a
folder of JSON files and it serves them as a REST API, zero
configuration required. An optional TOML rule set adds conditional
matching, Rhai scripting, and response strategies when you need more.

- ❄️ Zero-config start.
- 🌬️ Fast to boot, light on memory.
- 🪄 File-based and rule-based matching. Scripting supported.

## Why / When

- The backend is not ready yet.
- You need stable API responses for UI testing.
- You want offline development.
- CI tests require a predictable API.
- Your mock data is becoming large.

---

## Quick start

```sh
# via npm, into your app project
npm install -D apimock-rs && npx apimock
```

```sh
# or via cargo, as a standalone binary
cargo install apimock && apimock
```

```sh
# or download a prebuilt binary — no Node or Rust toolchain needed
# https://github.com/apimokka/apimock-rs/releases/latest
tar xzf 'apimock@Linux-x64-gnu-<version>.tar.gz'
cd 'apimock@Linux-x64-gnu-<version>' && ./apimock
```

Prebuilt binaries are published for Linux (x64 gnu, x64 musl, aarch64
musl), macOS (aarch64) and Windows (x64). Linux archives are `.tar.gz`,
macOS and Windows are `.zip`. Each one also ships an `apimock.toml`,
`apimock-rule-set.toml` and `apimock-middleware.rhai`, picked up
automatically when you run from that directory — so a downloaded build
answers `curl http://localhost:3001/health` straight away, with no setup
step at all.

```sh
# just use folders and JSON
mkdir -p api/v1/
echo '{"hello": "world"}' > api/v1/hello.json
npx apimock   # (or `apimock` / `./apimock`, depending on how you installed it)

# response
curl http://localhost:3001/api/v1/hello
# --> {"hello":"world"}
```

You now have a running REST endpoint (the commands below assume `npx`;
drop it for `cargo install`, or use `./apimock` for a downloaded
binary).

### `npx apimock` variation

| command | result |
| --- | --- |
| `npx apimock` | Run with all default parameters. |
| `npx apimock -p 4000` | Run with custom port. |
| `npx apimock -d tests/apimock-dyn-route` | Run with custom root dir on server response. |
| `npx apimock -c apimock.toml` | Run with config file giving rich features. Running `npx apimock --init` beforehand is required. |

### Setup with `npx apimock --init`

| command | result |
| --- | --- |
| `npx apimock --init` | Interactive setup. Prompts for port / IP / fallback dir / whether to scaffold a rule-set file, middleware file, and TLS section, then writes `apimock.toml` (and optionally `apimock-rule-set.toml` / `apimock-middleware.rhai`) customised to your answers. |
| `npx apimock --init --yes` | Non-interactive setup: skip every prompt and write the default config (`127.0.0.1:3001`, rule-set file included, TLS commented out). Useful in CI or Docker builds. |
| `npx apimock --init --middleware` | Also scaffold `apimock-middleware.rhai`. Combines with `--yes`. |

When stdin is not a TTY (piped, CI, Docker build), `--init` silently
falls back to the same defaults even without `--yes` — so
non-interactive usage in scripts and CI keeps working unchanged.

### Vite project integration

Run Vite and apimock-rs together with **concurrently** (parallel
processes) and **cross-env** (colored output across platforms):

```sh
npm install -D concurrently cross-env
```

```json
  "scripts": {
    "apimock": "npx apimock",
    "dev": "cross-env CLICOLOR_FORCE=1 concurrently \"vite\" \"npm run apimock\""
  }
```

```sh
npm run dev
```

---

## Features / Design Notes

**Read-on-demand, not preloaded.** No response is read at startup —
each is read from disk only when a matching request arrives, off the
async runtime's request-handling threads via a dedicated blocking-I/O
thread pool. Startup time and memory use stay flat regardless of
dataset size, and behaviour stays stable under repeated requests.

**Middleware, then rules, then the file tree.** Every configured
middleware script gets first refusal; unhandled requests then go
through the rule sets in order; anything still unmatched falls back to
serving a file directly by URL path. Zero-config mode is just that
fallback path with nothing else configured.

**Body matching uses a dotted-path mini-syntax, not JSONPath.**
`"customer.tier"` or `"items.0.sku"` — keys joined by `.`, numeric
segments index arrays. It resembles JSONPath but isn't one; a
`"$.foo.bar"`-style path will not match anything.

**`apimock validate` and `apimock match-test`** check a config, or
dry-run a rule match, without starting a server — useful in CI. See
the [docs](https://apimokka.github.io/apimock-rs/).

---

### 📖 Documentation - guides and references

For more details — including the complete configuration reference —
**🧭 check out our [full documentation](https://apimokka.github.io/apimock-rs/)**.

---

## Open-source, with care

This project is lovingly built and maintained by volunteers.  
We hope it helps streamline your API development.  
Please understand that the project has its own direction — while we welcome feedback, it might not fit every edge case 🌱

## Acknowledgements

Depends on [tokio](https://github.com/tokio-rs/tokio) / [hyper](https://hyper.rs/) / [hyper-util](https://crates.io/crates/hyper-util) / [http-body-util](https://crates.io/crates/http-body-util) / [rustls](https://github.com/rustls/rustls) / [tokio-rustls](https://github.com/rustls/tokio-rustls) / [rhai](https://github.com/rhaiscript/rhai) / [toml](https://github.com/toml-rs/toml) / [json5](https://github.com/callum-oakley/json5-rs) / [csv](https://github.com/BurntSushi/rust-csv) / [regex](https://github.com/rust-lang/regex) / [globset](https://crates.io/crates/globset) / [ignore](https://crates.io/crates/ignore) / [uuid](https://github.com/uuid-rs/uuid) / [console](https://github.com/console-rs/console) / [indexmap](https://github.com/indexmap-rs/indexmap) / [log](https://github.com/rust-lang/log) / [serde](https://serde.rs/) / [serde_json](https://github.com/serde-rs/json) / [thiserror](https://crates.io/crates/thiserror) / [anyhow](https://crates.io/crates/anyhow) / [tempfile](https://crates.io/crates/tempfile). In addition, [mdbook](https://github.com/rust-lang/mdBook) (as to workflows).
