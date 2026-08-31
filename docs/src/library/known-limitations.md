# Known limitations

Stated here so you meet them in documentation rather than in a
debugger.

## The public module surface is wider than it was designed to be

`apimock-server` has **16 of 16** top-level modules declared `pub mod`;
`apimock-routing`, **6 of 6**. So `apimock_server::http_util`,
`::dyn_route`, `::response_handler`, `apimock_routing::util::glob` and
many others are public API.

None of it was designed as an external surface. It is public because
nothing ever narrowed it — which was only noticed when 6.0.0's API
baseline put the whole surface in one file for the first time.

**What this means in practice:**

- It is genuinely public and genuinely gated, so depending on it will
  not break within 6.x.
- **Narrowing it is a candidate for 7.0.** If you build on
  `apimock_server::http_util`, you may be building on something
  intended to become private.
- If you depend on one of these modules,
  [say so](https://github.com/apimokka/apimock-rs/issues). A module a
  real consumer needs is an argument for keeping it public
  deliberately — which is a better outcome than narrowing it blind.

## Three surfaces have no proven consumer

Introduced on the [section index](./README.md); the specifics:

**`Workspace::has_external_changes()` / `sync_from_disk()`** — for the
"file changed underneath a long-lived session" case, which is
essentially a library-consumer problem. The CLI never needs them: it
loads, edits and exits inside one invocation. Unresolved:

- Whether `sync_from_disk()` preserves unsaved in-memory edits or
  discards them. This changes what you can offer a user, so establish
  it before designing around it.
- What granularity `has_external_changes()` reports at — whole
  workspace or per file.
- Whether it is cheap enough to poll, or wants a filesystem watcher in
  front of it.

**`apimock_server::control`** — `ServerControl`, `ServerHandle`,
`ServerState`, `ReloadHint`. `ReloadHint` is the valuable idea: it
converts to and from `apimock_config::view::ReloadHint`, so the config
layer can say whether an edit costs nothing, a reload, or a restart.
Unresolved: whether `ReloadHint::Reload` genuinely avoids a restart in
every case it claims.

**`apimock_server::trace::TraceEmitter`** — the live match feed.
`emit()` is called by the server's internals; the *subscription* side
has no consumer, and how a caller receives what `emit` sends is the
part to establish first.

## `[guard]` is a stub

`apimock_routing`'s `Guard` is an empty struct with a `// todo:`. The
[rule-set schema](../reference/rule-set-schema.md) documents it as not
yet doing anything. There is nothing behind it to build against.

## `validate`'s severity axis is effectively single-valued

The [CLI reference](../reference/cli-reference.md) records why:
`Workspace::load` rejects every condition that could become a
`Severity::Error`, so a configuration either loads clean or fails to
load. Nothing anywhere constructs a `Severity::Warning`.

`ValidationReport` carries a real severity type, but in practice one
value. If you build a warnings-versus-errors distinction in your UI,
you will be the first thing that needs warnings to exist — which is a
reasonable thing to want, and worth raising as a request rather than
working around.

## Validation is implemented twice

`apimock-config`'s node validation is a second implementation of
`apimock-routing`'s `Respond::validate`. The duplication is deliberate:
a structured `(severity, message, target_id)` triple is what a GUI can
render inline, and the routing crate's `log::error!`-and-return-`bool`
form is not.

They diverged once during 6.0.0's development — one copy did not learn
about a new field, and validation reported a false error on every
affected rule. There is now a test asserting the two agree.

**If `validate()` ever disagrees with whether a configuration actually
loads, that is a defect in apimock**, not something to work around.

## This section is the library documentation

The rest of `docs/` is written for someone running the CLI. There is no
deeper library guide behind this section — rustdoc on the crates, and
these pages.

If you find yourself writing internal notes to explain this API to your
own team, those notes are the documentation that is missing. They would
be welcome.
