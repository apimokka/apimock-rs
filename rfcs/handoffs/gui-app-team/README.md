# Handoff to the GUI app team — apimock 6.0.0 as a library

**For.** A team building a GUI application on top of apimock's Rust
library API, to help Web API developers.
**Against.** apimock **6.0.0**, released 2026-08-28.
**Not a CLI integration.** You link the crates directly. The threat
model already names you as a distinct actor for exactly this reason.

---

## Read this first: what is proven, and what you are the first user of

This matters more than anything else in the package, so it is not
buried in § 6.

apimock's library API was **designed** for a GUI — that intent is in
`apimock-config`'s own crate documentation and in RFC 004, 006, 016,
024 and others. But "designed for" is not "used by". Some of it is
exercised end to end by the CLI, with tests. Some of it has **never had
a real consumer**, and you will be the first.

| Surface | Status |
|---|---|
| `Workspace::load` | **Proven** — `apimock set`, `apimock validate` |
| `Workspace::apply` (all `EditCommand`s) | **Proven** — `apimock set rule` |
| `Workspace::save` | **Proven** — `apimock set rule` |
| `Workspace::validate` | **Proven** — `apimock validate` |
| `Workspace::snapshot` | **Proven** — `apimock validate`, `apimock get --why` |
| `Workspace::preview_changes` | **Proven** — `apimock set --dry-run` |
| `Workspace::has_external_changes` / `sync_from_disk` | ⚠️ **No consumer.** Defined, unit-tested inside `apimock-config`, never driven by a real application |
| `apimock_server::control::{ServerControl, ServerHandle, ServerState}` | ⚠️ **No consumer.** The CLI does not use them — it calls `Server::start` directly |
| `apimock_server::trace::TraceEmitter` (live match feed) | ⚠️ **No consumer** outside the server's own internals |

**What that means for you, practically:**

The proven rows have been shaped by a real caller hitting real edges —
RFC 057's `set` found and fixed several. The ⚠️ rows have not. Expect
missing conveniences, awkward signatures, and behaviour that is
*correct* but not *ergonomic*, because nobody has yet had to live with
them.

**Please report those as findings rather than working around them
silently.** An API nobody has used is a design in draft, and you are
the review. A workaround in your codebase leaves the next consumer to
rediscover the same thing.

## What to read, in order

| # | Document | Why |
|---|---|---|
| 1 | [`01-crates-and-architecture.md`](./01-crates-and-architecture.md) | Which of the four crates you depend on, and what each owns |
| 2 | [`02-editing-configuration.md`](./02-editing-configuration.md) | The `Workspace` model — load, edit, validate, save. Your core loop |
| 3 | [`03-running-a-mock-server.md`](./03-running-a-mock-server.md) | Starting a server, reacting to config changes, the live match feed |
| 4 | [`04-api-stability.md`](./04-api-stability.md) | What we promise not to break, and the gate that enforces it |
| 5 | [`05-security.md`](./05-security.md) | The threat model already names you. What it assumes you do |
| 6 | [`06-known-gaps.md`](./06-known-gaps.md) | Honest limits, open questions, and what we would like from you |

## The one-paragraph summary

Depend on **`apimock-config`** and **`apimock-server`**;
`apimock-routing` comes along and you will use its types. Load a
workspace with `Workspace::load`, render it from `snapshot()`, mutate it
with `apply(EditCommand)`, show problems from `validate()`, preview with
`preview_changes()`, and commit with `save()`. Run the mock with
`apimock_server::server::Server`. Everything you touch is covered by an
additive-only API gate as of 6.0.0, so a signature that compiles today
will keep compiling across 6.x.

## Conventions in this package

- **Every API shape quoted here came from the checked-in public-API
  baselines** (`crates/*/public-api.txt`), not from memory. Those files
  are generated from the crates and gated in CI — see
  [`04-api-stability.md`](./04-api-stability.md). If this package and a
  baseline ever disagree, **the baseline is right and this package is
  stale** — tell us.
- Where something is **unverified or uncertain, it says so.** We would
  rather hand you a known gap than a confident guess; this project has
  been bitten by the latter more than once.
