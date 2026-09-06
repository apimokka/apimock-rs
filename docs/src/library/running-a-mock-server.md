# Running a mock server

## Starting one

`apimock_server::server::Server` is the entry point:

| Method | Shape |
|---|---|
| `Server::new(Config)` | `async fn -> ServerResult<Self>` |
| `Server::start(&self)` | `async fn` |
| `Server::bind_http(&self)` | `async fn -> ServerResult<Option<TcpListener>>` |
| `Server::bind_https(&self)` | `async fn -> ServerResult<Option<(TcpListener, TlsAcceptor)>>` |
| `Server::serve_http(&self, TcpListener)` | `async fn` |
| `Server::serve_https(&self, TcpListener, TlsAcceptor)` | `async fn` |

`Config` comes from the workspace you already have —
`Workspace::config()` returns `&Config`.

**`start()` is the simple path**; the `bind_*`/`serve_*` pair is the
split version, which is what you want if you need the bound address
*before* serving begins. For a GUI that displays "listening on
127.0.0.1:3001", or that binds port 0 and must discover the real port,
**bind first, read the address, then serve.**

This is a **tokio** async API. Your GUI framework's event loop and a
tokio runtime have to coexist; that is your architectural decision, not
ours, but it is the first real one you will make.

## Port 0 is supported and useful

`-p 0` binds an ephemeral port; the CLI's own tests rely on it. For a
GUI, this is how you avoid "port 3001 already in use" as a first-run
experience. Bind, read the actual `SocketAddr`, show it.

## Reacting to configuration changes — ⚠️ unproven

`apimock_server::control` exists for the lifecycle a GUI has and the
CLI does not:

| Type | Purpose |
|---|---|
| `ServerControl` | `new()` — the control handle |
| `ServerHandle` | `http_addr`, `https_addr`, `cert_reloader`, `reload_tls_certs(cert, key)` |
| `ServerState` | Server lifecycle state |
| `ReloadHint` | `None` / `Reload` / `Restart` |

**`ReloadHint` is the useful idea here.** It converts to and from
`apimock_config::view::ReloadHint`, so the config layer can tell you
*how much* a given edit costs: nothing, a reload, or a full restart.
That lets your GUI apply most edits without dropping the listener, and
only warn about the ones that need a restart.

> ⚠️ **Nothing in this repository uses `ServerControl` or
> `ServerHandle`.** The CLI calls `Server::start` and never reloads —
> it is a one-shot process. These types are unit-tested within
> `apimock-server` and have no real consumer.
>
> Treat the reload path as **designed but unexercised**. Prove it does
> what you need early, in a spike, rather than discovering its shape
> late. If `ReloadHint::Reload` turns out not to be applicable without
> a restart in some case, that is a finding we want.

## TLS

`ServerHandle::reload_tls_certs(&str, &str)` reloads certificates
**without restarting** — RFC 020/021's hot reload. There is a user
guide at `docs/src/guides/reload-tls-certificates-without-restart.md`.

This path *is* exercised (there are TLS reload tests), unlike the rest
of `control`.

## The live match feed — ⚠️ unproven

`apimock_server::trace` is the "watch requests arrive and see which
rule answered" channel. There is even a CLI guide for the concept
(`docs/src/guides/watch-matches-live.md`).

| Type | Purpose |
|---|---|
| `TraceEmitter` | `emit(id, seq, RequestSummary, Outcome)`; carries an `Arc<TraceConfig>` |
| `TraceConfig` | `capture_body`, `max_body_bytes`, `header_allowlist`, `header_denylist`, `header_redaction` |
| `HeaderRedactionMode` | How headers are redacted before emission |
| `Outcome` | What happened to the request |
| `RequestSummary` | The request, summarised |

**`TraceConfig`'s redaction settings are not decoration.** RFC 051 and
RFC 040 added them because a live request feed will otherwise show
`Authorization` headers and request bodies on screen — and a GUI is
more likely than a CLI to have that on a shared screen, or in a
screenshot in a bug report. **Decide your defaults deliberately.** We
would suggest starting redacted and letting the user opt in.

**RFC 073 fixed two things worth knowing if you built against an
earlier version.** Before it, every event's `outcome` reported
`Miss { status: 0 }` regardless of what actually happened — a matched
rule, a middleware response and a genuine 404 were indistinguishable.
Every response path now emits the outcome that actually occurred
(`Matched`, the new `Middleware`, `Fallback`, or `Miss`) — if you match
on `Outcome` exhaustively, the new `Middleware` variant needs handling.
Separately, `header_denylist`/`header_allowlist`/`header_redaction` now
also govern query-string parameter values and JSON body object keys
(recursively), not headers alone — the same redaction, applied
everywhere a name-value pair can leave the process, so a captured body
(`capture_body`) reaching your subscriber is redacted the same way a
verbose console log line would be.

`AppState::new(Config, LoadedMiddlewares, TraceEmitter)` is where the
emitter is wired in.

> ⚠️ `TraceEmitter` has no consumer outside `apimock-server`'s own
> internals. The subscription side — how you *receive* what `emit`
> sends — is the part you should establish first. If it is awkward,
> say so; a live feed nobody has consumed is the definition of an
> unexercised design.

## What the CLI does, as a reference

`apimock get` answers "what would the server return for this request?"
**without starting a server**, by running the same dispatch the server
uses. If your GUI wants a "preview this rule's response" button, that
is the model to copy — and `apimock-server`'s response construction is
reachable directly.

`crates/apimock/src/cmd/get.rs` is the reference implementation, and
its tests assert it agrees with a real server
(`get_agrees_with_server.rs`). Worth reading before you build a preview
feature.
