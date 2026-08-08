# Design notes

Why apimock-rs behaves the way it does, for the two decisions readers
run into most often.

## Why read-on-demand, not preloaded

No response file is read at startup. Each one is read from disk only
when a matching request arrives — `task::spawn_blocking` moves the
actual `fs::read`/`fs::read_to_string` call onto a dedicated
blocking-I/O thread pool, off the async runtime's request-handling
threads (`crates/apimock-server/src/response/file_response.rs:82,93`).

This is what keeps startup time and memory use flat regardless of how
large a mock dataset grows: nothing scans or loads the whole file tree
up front. The fallback file-tree serving path
(`crates/apimock-server/src/dyn_route.rs`) does its own directory read
per request, not once at startup. The only mechanism in the codebase
that scans a directory tree ahead of time is the `Workspace` snapshot
API used by config-editing tooling — a GUI-facing feature, unrelated to
serving requests. See
[Filter the served file tree](../guides/filter-the-served-file-tree.md)
for that distinction in more detail.

## Why dotted paths, not JSONPath

`when.request.body.json` conditions and `respond.csv_records_key` both
use apimock's own dotted-path mini-syntax — `"customer.tier"`,
`"items.0.sku"` — object keys joined by `.`, a numeric segment indexing
into an array. This is deliberately **not** canonical JSONPath (RFC
9535): a `"$.foo.bar"`-style path is not special-cased, and `[0]`
bracket-array syntax isn't recognised. See
[Body path syntax](../reference/body-path-syntax.md) for the exact
resolution rules.

This distinction isn't cosmetic. A rule-set fixture in this project's
own history was written using `$.`-prefixed pseudo-JSONPath paths,
which silently never matched anything (because a leading `$` is just a
literal object key to this resolver, one that essentially never
exists) — the fixture shipped broken for three releases before anyone
noticed, precisely because a broken *condition* fails silently by never
matching, rather than by erroring. Every place in this documentation
that shows a body-path condition says so explicitly, for that reason.
