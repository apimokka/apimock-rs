# Using apimock as a library

Most of this documentation is about running `apimock` as a command.
This section is for building **on** it — linking the crates into your
own application rather than driving the binary.

The obvious case is a GUI: a long-lived session that loads a
configuration, lets someone edit it, validates as they go, and runs a
mock server against the result. The library API was shaped for that,
and the [threat model](../reference/threat-model.md) names a GUI
application as one of apimock's actors.

## Where to start

| Page | Covers |
|---|---|
| [Crates and architecture](./crates-and-architecture.md) | Which of the four crates to depend on |
| [Editing a configuration](./editing-configuration.md) | The `Workspace` model — load, edit, validate, save |
| [Running a mock server](./running-a-mock-server.md) | Starting a server, reloading, the live match feed |
| [API stability](./api-stability.md) | What is promised across 6.x, and what enforces it |
| [Known limitations](./known-limitations.md) | Surfaces without a proven consumer, and other honest gaps |

## In one paragraph

Depend on **`apimock-config`** and **`apimock-server`**;
`apimock-routing` comes along and you will name its types. Load with
`Workspace::load`, render from `snapshot()`, mutate with
`apply(EditCommand)`, surface problems from `validate()`, preview with
`preview_changes()`, commit with `save()`. Run the mock with
`apimock_server::server::Server`. Everything here is covered by an
additive-only API gate as of 6.0.0, so what compiles today keeps
compiling across 6.x.

## Read this before relying on anything

Parts of this API were **designed** for a library consumer but have
**never had one**. Some are exercised end to end by the CLI, with
tests; some have no caller anywhere in the project.

| Surface | Status |
|---|---|
| `Workspace::load` / `apply` / `save` / `validate` / `snapshot` / `preview_changes` | **Proven** — `apimock set` and `apimock validate` drive these |
| `Workspace::has_external_changes` / `sync_from_disk` | **No consumer.** Unit-tested inside `apimock-config`; never driven by an application |
| `apimock_server::control::{ServerControl, ServerHandle, ServerState}` | **No consumer.** The CLI calls `Server::start` directly and never reloads |
| `apimock_server::trace::TraceEmitter` | **No consumer** outside the server's own internals |

The proven rows have been shaped by a real caller meeting real edges.
The others have not — expect missing conveniences and signatures that
are correct without being comfortable.

[Known limitations](./known-limitations.md) goes into what specifically
is unresolved about each.

## A note on accuracy

Every API shape quoted in this section came from the checked-in
public-API baselines (`crates/*/public-api.txt`), which are generated
from the crates and gated in CI. **If this documentation and a baseline
disagree, the baseline is correct** — please
[open an issue](https://github.com/apimokka/apimock-rs/issues).
