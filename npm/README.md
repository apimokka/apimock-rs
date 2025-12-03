# apimock-rs (API Mock)

## Mock APIs easily 🎈 — just JSON and go

If you’re building or testing APIs, this tool makes mocking painless. It’s super fast, efficient, and flexible when you need it to be.
All you have to do to start up is just use folders and JSON without any config set.

- ❄️ Zero-config start.
- 🌬️ Fast to boot, light on memory.
- 🪄 File-based and rule-based matching. Scripting supported.

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

For more details, **🧭 check out [the docs](https://apimokka.github.io/apimock-rs/)**.

---

## Open-source, with care

[This project](https://github.com/apimokka/apimock-rs) is lovingly built and maintained by volunteers.  
We hope it helps streamline your API development.  
Please understand that the project has its own direction — while we welcome feedback, it might not fit every edge case 🌱

## Acknowledgements

Depends on [tokio](https://github.com/tokio-rs/tokio) / [hyper](https://hyper.rs/) / [toml](https://github.com/toml-rs/toml) / [serde](https://serde.rs/) / [serde_json](https://github.com/serde-rs/json) / [json5](https://github.com/callum-oakley/json5-rs) / [console](https://github.com/console-rs/console) / [rhai](https://github.com/rhaiscript/rhai). In addition, [mdbook](https://github.com/rust-lang/mdBook) (as to workflows).
