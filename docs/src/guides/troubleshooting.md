# Troubleshooting

Organised by symptom — what you actually see, not what causes it —
since that's what you have when something isn't working. Every check
below was reproduced against a running server before being written
down; each names a command you can run yourself, so a stale entry
fails visibly rather than misleading quietly.

## My file 404s

A request served by `fallback_respond_dir` (the zero-config,
"drop a JSON file in a folder" mode) can 404 for more than one reason.
Check these, in order:

**1. Is it actually inside `fallback_respond_dir`?** A file that exists
elsewhere on disk but outside the configured directory always 404s —
this is deliberate confinement (RFC 063), not a bug, and there's no
opt-out. If you need it served, point `fallback_respond_dir` at where
it actually lives, or move it.

```
$ curl -i --path-as-is http://localhost:3001/../outside.json
HTTP/1.1 404 Not Found
```

(`--path-as-is` matters for this specific check: `curl` normally
resolves `..` out of a URL client-side before sending it, same as a
browser would, so a plain `curl` here would send `/outside.json` —
still a 404 if that file doesn't exist inside `fallback_respond_dir`
either, but not actually testing confinement. `--path-as-is` sends the
raw, unresolved path.)

**2. Is the extension one apimock infers?** A request with no
extension (`/users`) tries `.json`, `.json5`, then `.csv`, in that
order, then `directory/index.*`. A `.txt` or other extension isn't in
that list — request it with the extension, or rename the file.

```
$ curl -o /dev/null -s -w '%{http_code}\n' http://localhost:3001/hello
200   # resolves hello.json
```

**3. Is case actually the problem, or did you rule it out too early?**
Every path segment is folded case-insensitively — `/API/Users.json`,
`/api/users.json`, and `/Api/USERS.JSON` all resolve the same file,
whatever case it's saved as on disk (RFC 075 F-05). If a differently
cased request still 404s, case isn't the cause; look at the other
items on this list instead of re-checking case.

**4. Is the URL percent-encoded the way you think it is?** `%20`
decodes to a space, and a non-ASCII filename is reachable either by its
literal UTF-8 bytes in the URL or the percent-encoded form (RFC 075
F-03) — both resolve the same file:

```
$ curl -o /dev/null -s -w '%{http_code}\n' 'http://localhost:3001/my%20file.json'
200   # resolves "my file.json"
```

A `+` in a path is **not** decoded to a space — that's
`application/x-www-form-urlencoded` behaviour for query strings and
form bodies, not paths (RFC 3986). If your filename has a literal `+`,
request it unencoded.

**5. Did a rule set claim the prefix and then not match?** A request
under a rule set's `[prefix].url_path` is scoped to that rule set —
`/api` matches `/api` and `/api/x`, never a sibling like `/apixyz`
(RFC 075 F-02). But once a request falls under a prefix, it's checked
against *that rule set's own rules*, not the fallback directory — a
miss there is a rule-matching problem (see the next section), not a
file-404 one, even though both currently answer 404.

## My rule matches everything (or nothing)

**Check `apimock validate` first — a rule with an unrecognised
condition key now fails to load, with a specific error, rather than
silently matching more broadly than intended (RFC 069).** Before
6.1.0, a misspelled condition key was silently ignored — the condition
you thought you wrote never existed, so the rule matched on whatever
conditions *were* spelled correctly (or matched everything, if none
were). That failure mode is gone: today, the config doesn't load at
all, and the error names the exact key and file:

```
$ apimock validate --config apimock.toml
apimock validate: failed to load config: invalid rule set TOML in `./rules.toml`
(/path/to/rules.toml): TOML parse error at line 2, column 14
  |
2 | when.request.uri_path = "/only-here"
  |              ^^^^^^^^
unknown field `uri_path`, expected one of `url_path`, `method`, `headers`, `body`
(did you mean `url_path`?)
```

If validation passes and a rule still matches more (or less) than
expected, it's a genuine matching question, not a config-loading one —
see [Dry-run a rule](./dry-run-a-rule.md) (`apimock match-test`), which
shows every condition checked and why each one did or didn't match,
and [Matching order and precedence](../how-it-works/matching-order-and-precedence.md)
for how rule sets and rules are tried in order.

## My snapshot test broke after upgrading

**A `.json` `file_path` response is now served exactly as written —
byte for byte (RFC 076).** It used to be parsed and re-serialised on
every request: minified, with object keys sorted alphabetically,
regardless of how the file was actually formatted on disk. A snapshot
or golden-file test built against that old, minified/alphabetised
output now sees the file's real bytes instead:

```
$ printf '{\n  "zebra": 1,\n  "apple": 2\n}\n' > data.json
$ curl http://localhost:3001/data
{
  "zebra": 1,
  "apple": 2
}
```

If your test expected `{"apple":2,"zebra":1}`, that expectation was
pinning the old defect, not the intended behaviour — update it to the
fixture file's own bytes. See
[Serve JSON files from a folder](./serve-json-files-from-a-folder.md)
for the full explanation, and
[Migrating to 6.2.0](./migrating-to-6-2.md) if you're upgrading across
this change specifically. `.json5` and `.csv` are unaffected —
converting them is the point, not a defect.

## My CORS request fails (credentials not reflected)

**A credentialed cross-origin request (carrying `Cookie` or
`Authorization`) only gets its `Origin` reflected back if that origin
is allowed (RFC 067).** `http://localhost:*` and `http://127.0.0.1:*`
are always allowed; anything else needs to be listed in
`[service].cors_allow_credentials_origins`. An unlisted origin still
gets a response — just the same non-credentialed
`access-control-allow-origin: *` a request with no credentials gets,
which a browser then refuses to expose to credentialed cross-origin
JavaScript:

```
$ curl -i -H 'Origin: https://my-app.example.com' -H 'Cookie: session=abc' http://localhost:3001/data
access-control-allow-origin: *
vary: *
# no access-control-allow-credentials — browser blocks the credentialed read
```

Add the origin to the config and it reflects correctly:

```toml
[service]
cors_allow_credentials_origins = ["https://my-app.example.com"]
```

```
access-control-allow-origin: https://my-app.example.com
access-control-allow-credentials: true
vary: Origin
```

See [Response headers](../reference/response-headers.md#cors--origin-and-credentials)
for the full table, and
[the threat model](../reference/threat-model.md#deliberate-allowances-with-reasons)
for why an unlisted origin isn't refused outright.

## My request is refused with 413

**A request body over `[service].max_request_body_bytes` (default 32
MiB) is refused before it's buffered, not after.**

```
$ head -c 40000000 /dev/zero | curl -s -i -X POST --data-binary @- http://localhost:3001/data
HTTP/1.1 413 Payload Too Large

request body exceeds the configured limit (33554432 bytes)
```

If you legitimately need larger request bodies, raise the limit:

```toml
[service]
max_request_body_bytes = 67108864  # 64 MiB
```

There's no way to disable the cap entirely — a bound, even a generous
one, is the point. Before it existed, a request body of any size was
collected whole; the external audit measured one 256 MiB request
taking the process from 9 MiB RSS to 462 MiB, reachable by a single
unauthenticated request (see [the threat model](../reference/threat-model.md)).

## Still stuck?

- **`apimock get`** answers "what would the server return for this
  request?" without starting a server — see
  [Check what a request returns](./check-what-a-request-returns.md).
- **`apimock match-test`** shows every condition on a rule and why it
  did or didn't match — see [Dry-run a rule](./dry-run-a-rule.md).
- **`--format json`** on any subcommand gives a structured
  `{"schema", "apimock", "result"}`/`{"schema", "apimock", "error"}`
  envelope; a failure's `error.kind` is one of a closed, documented set
  (`usage`, `config_invalid`, `config_unreadable`, `io`, `conflict`,
  `internal`) — see
  [the CLI reference](../reference/cli-reference.md#the-response-envelope---format-json)
  for what each means and which exit code it maps to.
