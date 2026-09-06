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

**There is no workaround today — not even from source.** An earlier
version of this page suggested an embedder could construct a `Server`
directly via the `apimock-server` crate and reach `ServerHandle` from
there. That was never tried before it was written, and it doesn't
compile.

`ServerHandle` is `#[non_exhaustive]` (RFC 052), which blocks exactly
this: an out-of-crate struct literal. Confirmed directly — a throwaway
crate depending on `apimock-server` and attempting

```rust
let handle = apimock_server::ServerHandle {
    http_addr: None,
    https_addr: None,
    cert_reloader: None,
};
```

fails with:

```
error[E0639]: cannot create non-exhaustive struct using struct expression
  --> src/main.rs:5:18
   |
5  |       let handle = ServerHandle {
   |  __________________^
6  | |         http_addr: None,
7  | |         https_addr: None,
8  | |         cert_reloader: None,
9  | |     };
   | |_____^
```

And `#[non_exhaustive]` is the *only* obstacle worth naming, not the
whole story: even setting it aside, nothing in this crate's public API
constructs or returns a `ServerHandle` for anything to call
`reload_tls_certs` on — no `ServerHandle::new`, no `From` impl, no
method on `Server` that hands one out. `server.rs` builds a
`ReloadableCertResolver` internally for the HTTPS listener and drops
it, with a comment acknowledging the wiring to expose it was left
unfinished (see "Why you can't reach it", above). A from-source
embedder is in exactly the same position as a CLI user: restart the
process to rotate a certificate.

If you need this working, that is a real gap to raise, not something
to build around — the library-level pieces (`ReloadableCertResolver`,
`ServerHandle::reload_tls_certs`) are real and tested; what's missing
is the wiring that would let any caller, embedder included, actually
obtain a handle.
