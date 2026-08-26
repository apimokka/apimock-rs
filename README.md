# apimock-rs (API Mock)

[![npm](https://img.shields.io/npm/v/apimock-rs)](https://www.npmjs.com/package/apimock-rs)
[![crates.io](https://img.shields.io/crates/v/apimock?label=rust)](https://crates.io/crates/apimock)
[![License](https://img.shields.io/github/license/apimokka/apimock-rs)](https://github.com/apimokka/apimock-rs/blob/main/LICENSE)

[![Rust Documentation](https://docs.rs/apimock/badge.svg?version=latest)](https://docs.rs/apimock)
[![Dependency Status](https://deps.rs/crate/apimock/latest/status.svg)](https://deps.rs/crate/apimock)
[![Releases Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/release-publish.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/release-publish.yaml)
[![App Docs Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/docs.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/docs.yaml)

![logo](docs/src/assets/logo.png)

Build a working REST API in seconds — without a backend.    
Frontend blocked by an unfinished backend ?
Need stable API responses for UI tests or offline development ?    
Drop JSON files into a folder and your API immediately exists.

## Mock APIs easily 🎈 — just JSON and go

If you’re building or testing APIs, this tool makes mocking painless. It’s super fast, efficient, and flexible when you need it to be.
All you have to do to start up is just use folders and JSON without any config set.

- ❄️ Zero-config start.
- 🌬️ Fast to boot, light on memory.
- 🪄 File-based and rule-based matching. Scripting supported.

### `apimock-rs` handles real project scale

As your project grows, your mock API grows, too. Large mock datasets often cause problems:

- Slow startup
- High memory usage
- Crashes during UI testing
- Unstable CI runs

### When to use ?

- The backend is not ready yet.
- You need stable API responses for UI testing.
- You want offline development.
- CI tests require a predictable API.
- Your mock data is becoming large.

### Performance

apimock-rs does not preload responses. Each response file is read only when a request arrives using non-blocking I/O. This keeps:

- Startup nearly instant
- Memory usage minimal
- Stable behavior under repeated requests

as validated with k6 load testing.
You can run UI development and automated tests continuously without worrying about server instability.

---

## Quick start

Easy to start with [npm package](https://www.npmjs.com/package/apimock-rs).

```sh
# install into your app project
npm install -D apimock-rs
# and go
npx apimock
```

```sh
# just use folders and JSON
mkdir -p api/v1/
echo '{"hello": "world"}' > api/v1/hello.json
npx apimock

# response
curl http://localhost:3001/api/v1/hello
# --> {"hello":"world"}
```

You may also check it out with browser to visit http://localhost:3001/api/v1/hello .

You now have a running REST endpoint.

### `npx apimock` variation

| command | result |
| --- | --- |
| `npx apimock` | Run with all default parameters. |
| `npx apimock -p 4000` | Run with custom port. |
| `npx apimock -d tests/apimock-dyn-route` | Run with custom root dir on server response. |
| `npx apimock -c apimock.toml` | Run with config file giving rich features. Running `npx apimock --init` beforehand is required. |
| `npx apimock --init` | **Interactive setup.** Prompts for port / IP / fallback dir / whether to scaffold a rule-set file, middleware file, and TLS section, then writes `apimock.toml` (and optionally `apimock-rule-set.toml` / `apimock-middleware.rhai`) customised to your answers. |
| `npx apimock --init --yes` | Non-interactive setup: skip every prompt and write the same defaults 4.7.0 wrote (`127.0.0.1:3001`, rule-set file included, TLS commented out). Useful in CI or Docker builds. |
| `npx apimock --init --middleware` | Also scaffold `apimock-middleware.rhai`. Combines with `--yes`. |

When stdin is not a TTY (piped, CI, Docker build), `--init` silently falls back to defaults even without `--yes` — so existing non-interactive usage of 4.7.0 keeps working unchanged.

### Vite project integration

An example of **scripts** section in **package.json** is as below.

**concurrently** is used to run the Vite and API mock servers simultaneously, while **cross-env** enables terminal output coloring. Before starting, ensure you run:

```sh
npm install -D concurrently cross-env
```

Edit package.json:

```json
  "scripts": {
    "apimock": "npx apimock",
    "dev": "cross-env CLICOLOR_FORCE=1 concurrently \"vite\" \"npm run apimock\""
  }
```

Run:

```sh
npm run dev
```

---

### 📖 Documentation - guides and references

For more details, **🧭 check out our [full documentation](https://apimokka.github.io/apimock-rs/)**.

- Configuration Reference: 📋 [View all settings here](https://apimokka.github.io/apimock-rs/user-guide/configuration-reference.html)

---

## Open-source, with care

This project is lovingly built and maintained by volunteers.  
We hope it helps streamline your API development.  
Please understand that the project has its own direction — while we welcome feedback, it might not fit every edge case 🌱

## Acknowledgements

Depends on [tokio](https://github.com/tokio-rs/tokio) / [hyper](https://hyper.rs/) / [toml](https://github.com/toml-rs/toml) / [serde](https://serde.rs/) / [serde_json](https://github.com/serde-rs/json) / [json5](https://github.com/callum-oakley/json5-rs) / [console](https://github.com/console-rs/console) / [rhai](https://github.com/rhaiscript/rhai) / [thiserror](https://crates.io/crates/thiserror) / [anyhow](https://crates.io/crates/anyhow). In addition, [mdbook](https://github.com/rust-lang/mdBook) (as to workflows).
