# API mock (apimock-rs)

[![crates.io](https://img.shields.io/crates/v/apimock?label=latest)](https://crates.io/crates/apimock)
[![Rust Documentation](https://docs.rs/apimock/badge.svg?version=latest)](https://docs.rs/apimock)
[![Dependency Status](https://deps.rs/crate/apimock/latest/status.svg)](https://deps.rs/crate/apimock)
[![Releases Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/release-executable.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/release-executable.yaml)
[![App Docs Workflow](https://github.com/apimokka/apimock-rs/actions/workflows/docs.yaml/badge.svg)](https://github.com/apimokka/apimock-rs/actions/workflows/docs.yaml)
[![License](https://img.shields.io/github/license/apimokka/apimock-rs)](https://github.com/apimokka/apimock-rs/blob/main/LICENSE)

![logo](docs/.assets/logo.png)

## 🪄 Mock APIs easily — no setup stress, just JSON and go

If you’re building or testing APIs, this tool makes mocking painless. You don’t need to write any config files — just use folders and JSON. It’s super fast, efficient, and flexible when you need it to be.

- 🎈 No config needed to get started
- 🥷 Fast to launch, light on memory, out of your way
- 🧩 Moreover, advanced matching and custom scripting supported

It’s rebuilt from the ground up in version 4. Designed to help developers of all levels.

### Getting started

```sh
# install
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

```sh
# also, there's room to tweak things later
npx apimock --init
```

### 📖 Documentation

For more details, **🧭 check out [the docs](https://apimokka.github.io/apimock-rs/)**.

### 💻️ GUI wrapper (development in progress)

[apimokka](https://github.com/apimokka/apimokka), an optimized GUI bundle, will be also available.

---

## 🛠️ App overhaul announcement

### ⚠️ v4, our new major version, was released. Compatibility Note

v4 is a complete rewrite, breaking compatibility with v3. A direct migration isn't supported; please opt to continue with v3 or start fresh with v4. V4's streamlined file-based and rule-based architecture aims to make new response configurations less burdensome.

---

## Open-source, with care

This project is lovingly built and maintained by volunteers.  
We hope it helps streamline your API development.  
Please understand that the project has its own direction — while we welcome feedback, it might not fit every edge case 🌱

## Acknowledgements

Depends on [tokio](https://github.com/tokio-rs/tokio) / [hyper](https://hyper.rs/) / [toml](https://github.com/toml-rs/toml) / [serde](https://serde.rs/) / [serde_json](https://github.com/serde-rs/json) / [json5](https://github.com/callum-oakley/json5-rs) / [console](https://github.com/console-rs/console) / [rhai](https://github.com/rhaiscript/rhai). In addition, [mdbook](https://github.com/rust-lang/mdBook) (as to workflows).
