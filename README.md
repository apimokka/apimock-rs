# apimock-rs (API Mock)

[![npm](https://img.shields.io/npm/v/apimock-rs)](https://www.npmjs.com/package/apimock-rs)
[![crates.io](https://img.shields.io/crates/v/apimock?label=rust)](https://crates.io/crates/apimock)
[![License](https://img.shields.io/github/license/apimokka/apimock-rs)](https://github.com/apimokka/apimock-rs/blob/main/LICENSE)    
[![Rust Documentation](https://docs.rs/apimock/badge.svg?version=latest)](https://docs.rs/apimock)
[![Dependency Status](https://deps.rs/crate/apimock/latest/status.svg)](https://deps.rs/crate/apimock)
[![Releases Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/release-executable.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/release-executable.yaml)
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

### Setup with `npx apimock --init`

| command | result |
| --- | --- |
| `npx apimock --init` | Interactive setup. Prompts for port / IP / fallback dir / whether to scaffold a rule-set file, middleware file, and TLS section, then writes `apimock.toml` (and optionally `apimock-rule-set.toml` / `apimock-middleware.rhai`) customised to your answers. |
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

## Workspace layout (5.0.0+)

Starting with 5.0.0, apimock is organised as a Cargo workspace:

| crate | responsibility |
| --- | --- |
| [`apimock-config`](crates/apimock-config) | TOML config model — load, validate, resolve relative paths. Defines the stage-1 edit / snapshot API types a GUI can depend on. |
| [`apimock-routing`](crates/apimock-routing) | Rule-set definitions, request matching, read-only view types for GUI tooling. |
| [`apimock-server`](crates/apimock-server) | HTTP(S) listener, request dispatch, Rhai middleware compilation, response building. |
| `apimock` (workspace root) | CLI binary + thin façade library. Re-exports the three member crates under `apimock::config`, `apimock::routing`, `apimock::server`. |

End users installing via `cargo install apimock` or `npx apimock` see no difference from 4.8.0. Library consumers migrating from 4.x paths can find the mapping in [CHANGELOG.md](CHANGELOG.md).

### GUI-facing Workspace API (5.1.0+)

5.1.0 adds `apimock_config::Workspace` for tooling that wants to edit a configuration through structured commands instead of TOML text. A GUI typically holds a single `Workspace` value for one editing session, calls `snapshot()` to produce a render-ready view, and `apply(EditCommand)` to mutate. Every editable node carries a stable `NodeId` (a v4 UUID) that survives reorderings, so a selection set anchored on NodeIds remains valid across edits.

```rust
use apimock_config::{
    Workspace, EditCommand, RespondPayload, RulePayload,
    NodeId, ConfigFileKind, NodeKind,
};

let mut ws = Workspace::load("apimock.toml".into())?;

// Find the rule-set node from the snapshot.
let snap = ws.snapshot();
let rule_set_id: NodeId = snap.files.iter()
    .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
    .and_then(|f| f.nodes.iter().find(|n| matches!(n.kind, NodeKind::RuleSet)))
    .map(|n| n.id)
    .expect("rule set");

// Add a new rule.
let result = ws.apply(EditCommand::AddRule {
    parent: rule_set_id,
    rule: RulePayload {
        url_path: Some("/api/new".into()),
        method: Some("GET".into()),
        respond: RespondPayload {
            text: Some("hello".into()),
            ..Default::default()
        },
    },
})?;

if !result.diagnostics.is_empty() {
    // surface each per-node diagnostic in the GUI
}
```

Saving back to disk (`Workspace::save`) is reserved for 5.2.0; 5.1.0 covers Steps 1–3 of the GUI extension plan (snapshot + apply + validate).

---

## Open-source, with care

This project is lovingly built and maintained by volunteers.  
We hope it helps streamline your API development.  
Please understand that the project has its own direction — while we welcome feedback, it might not fit every edge case 🌱

## Acknowledgements

Depends on [tokio](https://github.com/tokio-rs/tokio) / [hyper](https://hyper.rs/) / [toml](https://github.com/toml-rs/toml) / [serde](https://serde.rs/) / [serde_json](https://github.com/serde-rs/json) / [json5](https://github.com/callum-oakley/json5-rs) / [console](https://github.com/console-rs/console) / [rhai](https://github.com/rhaiscript/rhai) / [thiserror](https://crates.io/crates/thiserror) / [anyhow](https://crates.io/crates/anyhow). In addition, [mdbook](https://github.com/rust-lang/mdBook) (as to workflows).
