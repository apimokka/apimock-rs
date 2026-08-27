# apimock-rs (API Mock) Documentation

<div class="logo">
  <style>
    .logo {
        width: 100%;
        display: flex;
        justify-content: center;
    }
    .logo img {
        height: 4.4em;
    }
  </style>
  <img src="assets/logo.png">
</div>

A developer-friendly, featherlight and functional HTTP(S) mock server built in Rust.

### Who is this for?

- Developers who want to quickly mock APIs without heavy setup.
- Beginners who benefit from minimal configuration.
- Advanced users needing logic-based response behavior.
- Agents (and scripts) driving apimock non-interactively: checking what
  a request would return with [`get`](guides/check-what-a-request-returns.md),
  writing a rule with [`set`](guides/add-or-change-a-rule.md), and
  reading `--format json` output instead of parsing human text.

### Quick Start

Easy to start with [npm package](https://www.npmjs.com/package/apimock-rs).

```sh
npm install -D apimock-rs

npx apimock
# alternatively, starts with spefic root directory:
# npx apimock -d tests
```

![demo](https://github.com/apimokka/apimock-rs/blob/main/docs/src/assets/demo.gif?raw=true)

## For Users

- [**Getting started**](getting-started/) — install and your first mock API, in order
- [Guides](guides/) — task-indexed how-tos
- [Reference](reference/) — exhaustive lookup
- [How it works](how-it-works/) — what apimock does and why

---

## Contributing

- [Contributing](contributing/) — build, test, and the RFC process
- [Source code (GitHub)](https://github.com/apimokka/apimock-rs)
