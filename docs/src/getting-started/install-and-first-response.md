# Install and first response

## Install

Three ways to get `apimock`:

```sh
# via npm, into an existing project
npm install -D apimock-rs
npx apimock
```

```sh
# via cargo, as a standalone binary
cargo install apimock
apimock
```

```sh
# via a prebuilt binary — no Node or Rust toolchain needed
# download from https://github.com/apimokka/apimock-rs/releases/latest
tar xzf 'apimock@Linux-x64-gnu-<version>.tar.gz'
cd 'apimock@Linux-x64-gnu-<version>'
./apimock
```

Whichever you choose, the command you run afterward is `apimock` (the
package on npm is named `apimock-rs`, but the binary it installs is
`apimock`).

### Which prebuilt archive

**⬇️ [Download from the latest release](https://github.com/apimokka/apimock-rs/releases/latest)**,
then pick the archive for your platform:

| Platform | Archive |
| --- | --- |
| Linux x64 (glibc) | `apimock@Linux-x64-gnu-<version>.tar.gz` |
| Linux x64 (musl) | `apimock@Linux-x64-musl-<version>.tar.gz` |
| Linux aarch64 (musl) | `apimock@Linux-aarch64-musl-<version>.tar.gz` |
| macOS aarch64 | `apimock@macOS-aarch64-<version>.zip` |
| Windows x64 | `apimock@Windows-x64-<version>.zip` |

Each archive unpacks to a directory containing the binary plus an
`apimock.toml`, `apimock-rule-set.toml` and `apimock-middleware.rhai`.
Run `./apimock` from that directory and it loads them automatically —
no `-c` flag and no `apimock --init` step — so a downloaded build is
already serving the example rules:

```sh
curl http://localhost:3001/health   # --> ok
curl http://localhost:3001/greet    # --> Hello, world.
```

That also means a downloaded build is ready for the
[config-file walkthrough](./your-first-config-file.md) as it stands.

## Zero configuration needed

Run `apimock` in an empty directory and it starts immediately — no
config file required:

```sh
apimock
# Listening on http://127.0.0.1:3001 ...
```

At this point, every request 404s, because there's nothing to serve
yet.

## Your first response

Drop a JSON file into the directory `apimock` is running from:

```sh
mkdir -p api/v1/
echo '{"hello": "world"}' > api/v1/hello.json
```

Restart `apimock` (or start it now, if it wasn't already running), then
request the path matching that file — the `.json` extension is
optional:

```sh
curl http://localhost:3001/api/v1/hello
# --> {"hello":"world"}
```

You can check the same thing in a browser at
`http://localhost:3001/api/v1/hello`.

That's file-based routing: the URL path maps directly to a file path.
A directory request (`/`, `/api`, `/api/v1`) looks for an `index.json`
/ `index.json5` / `index.csv` / `index.html` file and 404s if none
exists. `.json5` files are treated the same as `.json`; `.csv` files
are converted to a JSON array of rows.

Next: [Your first config file](./your-first-config-file.md), for
responses that depend on more than just the URL path.
