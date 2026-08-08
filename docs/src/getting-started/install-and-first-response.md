# Install and first response

## Install

Two ways to get `apimock`:

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

Either way, the command you run afterward is `apimock` (the package on
npm is named `apimock-rs`, but the binary it installs is `apimock`).

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
