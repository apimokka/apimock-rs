# Migrating to 6.0.0

This is a preview, written from 5.19.0, of what 6.0.0 is expected to
break — so that meeting a deprecation warning in this release gives you
the whole picture at once rather than one line at a time. It covers two
different kinds of break:

- **CLI changes**, which 5.19.0 already warns about at the point you hit
  them (see below).
- **Library changes**, which cannot be warned about at all — there's no
  mechanism for a Rust compiler warning to say "this will be a breaking
  change in a future major version" the way a CLI can print to stderr.
  If you depend on `apimock-server`, `apimock-config`, or
  `apimock-routing` directly (rather than only running the `apimock`
  binary), this section is the only warning you get before 6.0.0.

None of this has shipped yet. 6.0.0's timing is the project owner's
decision alone (see the [roadmap](https://github.com/apimokka/apimock-rs/blob/main/ROADMAP.md)),
and the RFCs this page describes may still change before then. This page
will be revised alongside them; treat it as the best information
available today, not a frozen spec.

## CLI: `apimock validate --json` is removed

Covered in depth in the [CLI reference](../reference/cli-reference.md#apimock-validate)
and the [validate-in-CI guide](./validate-config-in-ci.md); summarised
here because it's the one break you can act on immediately.

`--json` (a bare diagnostics array) is deprecated as of 5.19.0 and
removed in 6.0.0. **`--format json` is available now**, in 5.19.0,
carrying the response shape 6.0.0 keeps — so you can switch today and
verify against a real binary before the old flag is gone. 6.0.0's
removal of `--json` is expected to fail loudly with a machine-readable
error naming `--format json`, rather than silently changing what a
script parses — a general policy for breaking CLI invocations at a
major version, not specific to this one flag.

No other CLI invocation that works today is expected to change.
`match-test`'s text output is untouched — 6.0.0 *adds* `--format json`
to it rather than reshaping what it prints. Bare `apimock` keeps working
as an alias for `apimock serve`.

## Library: five public structs are now `#[non_exhaustive]`

**Done, on `main`, ahead of 6.0.0's eventual release** (RFC 052) — this
is the one item on this page that has already shipped rather than being
a preview, because `main` is the 6.0.0 line and the break is real from
this point on for anyone building against it.

`TraceConfig`, `RequestSummary` (`apimock-server::trace`),
`ParsedRequest` (`apimock-routing`), `LogConfig`, and `VerboseConfig`
(`apimock-config`) are all `pub` structs with public fields. Before this
change, constructing one with a struct literal, or exhaustively
destructuring one (`let Foo { a, b, c } = value;` naming every field),
both compiled from any crate. Now both stop compiling from outside the
type's defining crate — fields stay publicly readable by name
(`value.body_json` still works everywhere), only literal construction
and exhaustive destructuring are affected.

**What replaces a struct literal, concretely — the two types with a
real cross-crate constructor:**

- **`ParsedRequest::new(url_path: String, component_parts: hyper::http::request::Parts) -> Self`**
  builds one with no body (`body_json`/`body_len` both `None`) — the
  shape every existing caller outside `apimock-routing` actually wanted.
  Chain **`.with_body(body_json: Option<Value>, body_len: Option<usize>)`**
  to attach one, replacing whatever was there before (it doesn't merge
  with a prior call).
- **`VerboseConfig::new(header: bool, body: bool) -> Self`** — a `const
  fn`, so it works in a `const` initializer, which a runtime-only
  builder would not. `LogConfig` didn't need one: nothing outside
  `apimock-config` ever constructed it with a literal — every existing
  use goes through `Default` or `Deserialize`, both untouched by
  `#[non_exhaustive]`.

**`TraceConfig` and `RequestSummary` got the attribute but no new
constructor** — every construction site for both, checked across the
whole workspace, was already inside `apimock-server`, the crate that
defines them, so nothing outside that crate was ever affected.
`TraceConfig::default()` (already existed) remains how to build one from
elsewhere if you need to; a real cross-crate literal site would need
its own constructor the same way `ParsedRequest`'s did, and none exists
today.

**What replaces exhaustive destructuring:** match or destructure with
`..` to ignore fields you don't use (`let Foo { a, .. } = value;`), which
already compiled before this change and keeps compiling after it — the
mechanical fix, if you hit this, is adding `..`.

**Why now, in one release, rather than piecemeal:** three RFCs landing
on `main` this month (040, 050, and the shape of 051's own configuration
surface) each added fields to one or more of these types, and every one
of those additions was, strictly, a breaking API change that went
unnoticed until asked about directly. RFC 052 takes that break once,
deliberately, instead of repeating it by accident — see RFC 052 itself
for the full reasoning.

**Whether the GUI constructs any of these five is still an open
question** (RFC 052's Unresolved 1) — the constructors above were built
for what this workspace's own code needs, established from source
rather than guessed at. If the GUI turns out to construct one of the
three that didn't get a constructor, that's an additive addition on top
of this shape, not a redesign.

## Library: `TraceConfig`, `ParsedRequest`, and `RequestSummary` already had new fields, before `#[non_exhaustive]`

For the historical record — the reason RFC 052 exists at all. RFC 040
and RFC 050 each added fields to these types before `#[non_exhaustive]`
existed to absorb that:

- `TraceConfig` gained `header_redaction`, `header_denylist`,
  `header_allowlist` (RFC 040 — request-header redaction for the trace
  channel).
- `ParsedRequest` and `RequestSummary` each gained `body_len` (RFC 050 —
  a non-JSON request body's presence and length, never its content).

Both additions predate `#[non_exhaustive]` landing, so a struct literal
written against an older version of either type would already have
needed updating for this reason alone, independent of the
`#[non_exhaustive]` change above. This is exactly the class of break
`#[non_exhaustive]` now exists to prevent recurring.

## Library: error enums may be reshaped

**Deferred to 6.0.0, not yet designed** — this is RFC 041, which this
page cannot give you specifics for, because RFC 041 itself is deferred
pending that design work. `ConfigError`, `WorkspaceError`, and related
error types are not `#[non_exhaustive]` today (unlike the five structs
above, this is a known, explicitly separate question — see RFC 052's
own Unresolved questions), so a caller matching on one of these
exhaustively is in the same position struct-literal construction is: it
compiles today and may not after 6.0.0.

**Honest gap:** we don't yet know whether RFC 041 will add
`#[non_exhaustive]` alone, restructure the variants, or both. If you
match exhaustively on `apimock_config::ConfigError` or
`apimock_config::WorkspaceError` today, treat that as a code path worth
revisiting when RFC 041 is written, not something this page can tell you
how to fix yet.

## What isn't changing

Worth stating plainly, since a migration page can read as longer than it
is: exit codes (`0`/`1`/`2`, set in RFC 049) are not changing. Stream
discipline (diagnostics to stderr, machine-readable output to stdout) is
not changing. `validate`'s own diagnostics, severities, and exit codes
are not changing — only `--format json`'s wrapping shape around them is
new. Nothing about how a mock server matches or responds to requests is
changing.
