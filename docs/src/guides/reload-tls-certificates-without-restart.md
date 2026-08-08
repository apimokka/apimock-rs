# Reload TLS certificates without restart

**Not currently possible via the `apimock` CLI.** The mechanism exists
in the `apimock-server` library and is unit-tested, but nothing wires
it up to a running server started from the CLI — this page documents
that state honestly rather than a workflow you can actually follow
today.

## What exists

`ReloadableCertResolver` (`crates/apimock-server/src/tls.rs`) holds the
active certificate behind a lock and can swap it for a freshly-read
one via `reload_from_paths(cert_path, key_path)` — a single atomic
pointer swap, no socket rebind, no new listener. In-flight TLS
handshakes that started before the swap complete with the old
certificate; anything after gets the new one. A failed reload (bad
path, unparseable PEM) leaves the previous certificate in place and
returns an error rather than breaking TLS.

`ServerHandle::reload_tls_certs(cert_path, key_path)`
(`crates/apimock-server/src/control.rs`) is the public entry point that
would trigger this from outside the server.

## Why you can't reach it

`ServerHandle` is never constructed anywhere in this repository — not
by the `apimock` CLI, not by any example, not by any test. The HTTPS
listener does build a `ReloadableCertResolver` internally
(`Server::https_start`, `crates/apimock-server/src/server.rs`), which
is why TLS itself works and stays up — but the handle needed to call
`reload_from_paths` from outside is discarded immediately after, with
a comment in the source acknowledging the wiring was left unfinished.

Restarting the `apimock` process is, today, the only way to rotate a
certificate.

## If you need this now

The library API above is real and tested — an embedder willing to
construct a `Server` directly via the `apimock-server` crate (rather
than running the `apimock` binary) could reach `ServerHandle` and
`reload_tls_certs` itself. That's a from-source integration, not
something the shipped CLI exposes.
