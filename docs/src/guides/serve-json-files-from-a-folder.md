# Serve JSON files from a folder

The headline feature: drop JSON files into a folder and they're
immediately reachable as an API — no rules, no rule set, nothing to
author.

```sh
mkdir -p api/v1/
echo '{"hello": "world"}' > api/v1/hello.json
apimock

curl http://localhost:3001/api/v1/hello
# --> {"hello": "world"}
```

The URL path maps directly to a file path under `service.fallback_respond_dir`
(`.` by default — the current directory). A `.json` file is served
**exactly as written** — same key order, same whitespace, same
formatting. It is not parsed and re-serialised, so what you put on disk
is what a client gets back, byte for byte. (`.json5` and `.csv` are
different: both are conversions by design — `.json5` because JSON5
syntax isn't valid JSON and has to become some, `.csv` because a table
becomes a JSON array of objects, one per row, keyed by column header —
and both are still recognised alongside `.json` when the extension is
optional, below.)

## URL-to-file resolution

- **The extension is optional** in the request — `/hello` and
  `/hello.json` both resolve `hello.json`, trying `.json`, `.json5`,
  then `.csv` in that order for the extension-less form.
- **Percent-encoding is decoded** — `/my%20file.json` resolves
  `my file.json`, and a non-ASCII filename is reachable using either
  its literal UTF-8 bytes in the URL or the percent-encoded form.
- **Case is folded at every segment**, not only the filename —
  `/API/Users.json`, `/api/users.json` and `/Api/USERS.JSON` all
  resolve the same file, whatever case it's actually saved as on disk.
  apimock does this folding itself rather than relying on the
  filesystem: Linux is case-sensitive, Windows and macOS (APFS by
  default) are not, so a config depending on the filesystem's own
  behaviour would work when written and 404 (or resolve something
  unexpected) the moment it ran somewhere else — the failure mode is
  specifically "works on the author's machine, breaks in CI or for a
  teammate on a different OS". Uniform, apimock-enforced case folding
  is what makes a committed rule set behave identically everywhere.
  **Unicode *normalisation* is a separate question and out of scope**:
  a filename that's the same characters encoded differently (NFC vs
  NFD — how an accented letter is represented, not how its case is
  folded) is filesystem-dependent and not resolved by apimock; that's
  a known limitation, not a bug to report.

A full worked example — collection and member endpoints, plus a CSV
file — is [`crates/apimock/examples/serve-json-resources/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/serve-json-resources),
runnable and automatically verified; its
[README](https://github.com/apimokka/apimock-rs/blob/main/crates/apimock/examples/serve-json-resources/README.md)
walks through it with real `curl` output.

This is the last stage in the request pipeline — see
[Matching order and precedence](../how-it-works/matching-order-and-precedence.md).
Once you need conditional responses (different output depending on
headers, method, or body content), move on to
[Match on URL path and method](./match-on-url-path-and-method.md).
