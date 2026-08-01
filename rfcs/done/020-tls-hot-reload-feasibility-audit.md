# RFC 020 — TLS hot-reload feasibility audit

**Status.** Implemented (v5.11.0)
**Tracks.** RFC 003 Unresolved §1 — auditing whether the
apimock-server TLS stack supports certificate hot-reload, and
adjusting `ReloadHint::for_key` for the `TlsEnabled` /
`TlsCertFile` / `TlsKeyFile` keys accordingly.
**Touches.** `apimock-server` (`tls.rs` and `server.rs` —
potentially adds a dynamic `CertResolver`), `apimock-config`
(`view.rs::ReloadHint::for_key` — possible reclassification of
TLS keys from `restart()` to `reload()`), documentation
(operational guidance for TLS rotation).

## Summary

RFC 003 conservatively classified TLS-related root settings
(`TlsEnabled`, `TlsCertFile`, `TlsKeyFile`) as requiring
`HardRestart` (full process restart). Its Unresolved §1 flagged
that some TLS stacks support cert hot-reload without rebinding the
listener, and that the conservative classification should be
revisited after a server-crate audit.

This RFC is that audit. It walks through the apimock-server TLS
stack (rustls + hyper + tokio-rustls), determines feasibility,
and recommends one of three outcomes:

- **Outcome A — Implement hot-reload.** Add a dynamic
  `CertResolver` to the TLS config; reclassify the three keys to
  `SoftReload`. Implementation work: ~80–150 lines.
- **Outcome B — Confirm restart-required.** No code change;
  document why and close the question. RFC 020 is then withdrawn
  to `archive/` with the audit findings preserved.
- **Outcome C — Partial.** `TlsEnabled` toggle still requires
  restart (binding TLS to a previously plain HTTP listener changes
  the socket setup); `TlsCertFile` / `TlsKeyFile` rotation
  becomes `SoftReload`. Implementation work: ~80 lines.

The audit and recommendation below favour **Outcome C** as the
most useful for the operational reality (rotating certs is common;
toggling TLS on/off is rare).

## Motivation

TLS certificate rotation is a routine operational task:
Let's Encrypt certs expire every 90 days; cloud KMS-issued certs
rotate monthly; private CAs typically every 6–12 months. Each
rotation under the v5.10 model requires restarting apimock,
dropping in-flight connections and (worse) requiring the operator
to remember the rotation and act on it.

If hot-reload is feasible, classifying `TlsCert*` as `SoftReload`
unlocks:

- GUI workflow: "I changed the cert path; click Apply; no
  restart needed."
- Operational scripting: `apimock-control reload` after each
  certbot deploy hook.
- Better honesty about what the server can actually do — the
  current `HardRestart` classification is defensive, not factual.

If hot-reload is *not* feasible (e.g. the apimock-server TLS stack
locks the cert at bind time and rebinding is required), the audit
documents *why* so the next person doesn't redo this work in
v5.12.

## Audit findings — TLS stack

### Current implementation

`apimock-server::tls` exposes two functions:

```rust
pub fn load_certs(file_path: &str) -> ServerResult<Vec<CertificateDer<'static>>>;
pub fn load_private_key(file_path: &str) -> ServerResult<PrivateKeyDer<'static>>;
```

These are called once at server startup (in
`apimock-server::server::serve` or equivalent) to construct a
`rustls::ServerConfig`, which is wrapped in `Arc<ServerConfig>` and
passed to `tokio-rustls::TlsAcceptor`. The acceptor lives for the
lifetime of the server.

Cert reload requires either:

1. Replacing the `Arc<ServerConfig>` referenced by the acceptor, or
2. Using a `rustls::server::ResolvesServerCert` trait object
   (dynamic cert resolver) on the existing config.

### Path 1: Swap `Arc<ServerConfig>`

`TlsAcceptor::from(Arc<ServerConfig>)` takes the config by Arc.
Swapping the Arc itself requires the acceptor to hold an
`Arc<ArcSwap<ServerConfig>>` indirection — straightforward with the
`arc-swap` crate, but every accept call must dereference through it,
and the swap point must be safe for in-progress handshakes.

Concretely: a new connection mid-rotation might use the old cert
(harmless), but a partially-complete TLS handshake using the old
cert won't be interrupted (also fine — TLS handshake state holds
the cert independently once accepted).

**Verdict:** feasible. Adds one dependency (`arc-swap`) and a level
of indirection at accept time. Estimated cost: 60 lines, mostly
glue.

### Path 2: Dynamic `CertResolver`

`rustls::ServerConfig::with_cert_resolver` accepts a
`Box<dyn ResolvesServerCert>`. The resolver is consulted per
handshake (it inspects the ClientHello and returns a `CertifiedKey`).
A custom resolver can read from an `Arc<Mutex<CertifiedKey>>` or
similar that the reload path swaps.

**Verdict:** also feasible, slightly more idiomatic rustls. Cost:
~80 lines (resolver impl + reload path). No new dependency.

### Recommendation

Path 2 (dynamic resolver) is the cleaner shape — it's what
rustls's design intends for this use case. Path 1 works but uses
`Arc` swapping in a way that's slightly off the rustls happy path.

### What rotates atomically

With either path: the **pair** of (cert, key) must be loaded
atomically. The reload code reads cert and key from disk, parses
both, validates that the key matches the cert, and only then
publishes the new `CertifiedKey` to the resolver. A failed parse on
either file leaves the previous cert active and surfaces a
diagnostic.

### What does not hot-reload

- **`TlsEnabled` toggle.** Going from `false` to `true` (or vice
  versa) requires re-creating the listener: bare TCP vs TLS-wrapped
  TCP are different `TcpListener` setups. This stays `HardRestart`.
- **Listener address/port.** Already `HardRestart`; not part of
  TLS.

## Guide-level explanation

After this RFC (Outcome C):

- Rotating TLS certs in-place — change `tls_cert_file`,
  `tls_key_file`, click Apply in the GUI (or `reload` in CLI) —
  takes effect within ~1 second without dropping connections.
- Toggling TLS on or off still requires a full restart.
- The GUI reload hint distinguishes the two: changing a cert path
  produces "reload"; toggling `tls_enabled` produces "restart".

Behavioural promises:

- In-progress requests using the old cert complete normally.
- New connections opened after the reload use the new cert.
- A reload with malformed cert / key returns an error diagnostic;
  the previous cert continues to serve until a successful reload.

## Reference-level explanation

### Server-side changes (Outcome C)

1. `apimock-server::tls` gains a `ReloadableCertResolver`:

   ```rust
   pub struct ReloadableCertResolver {
       inner: Arc<ArcSwap<CertifiedKey>>,
   }

   impl ResolvesServerCert for ReloadableCertResolver {
       fn resolve(&self, _: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
           Some(self.inner.load_full())
       }
   }

   impl ReloadableCertResolver {
       pub fn new(initial: CertifiedKey) -> Self { /* … */ }
       pub fn reload(&self, cert: Vec<CertificateDer<'static>>, key: PrivateKeyDer<'static>)
           -> Result<(), TlsReloadError> { /* validate + swap */ }
   }
   ```

2. `ServerConfig` is built `with_cert_resolver(Arc::new(resolver))`
   instead of `with_single_cert(…)`.

3. A new control-plane endpoint or signal (`reload_tls`) triggers
   the resolver to re-read from disk. Surface choice (REST endpoint
   on control port, signal handler, or RPC method) is implementation
   detail; the simplest is a method on the existing server handle
   that an apply-result with `reload()` hint can invoke.

### Config-side changes

`ReloadHint::for_key` reclassifies:

```rust
TlsEnabled               => Self::restart(),     // unchanged
TlsCertFile | TlsKeyFile => Self::reload(),       // was: restart()
```

Updates to RFC 003's reload table in `view.rs` rustdoc, and to the
`docs/src/advanced-topics/listener/https-support.md` page.

### Validation

`TlsCertFile` / `TlsKeyFile` validation in
`apimock-config::workspace::validate` already checks file existence.
Add: parse-validity check (call `load_certs` / `load_private_key`
in a "dry-run" mode that returns parsed values without applying).
Failed parse surfaces as a validation diagnostic, blocking save
with a clear error.

### Failure modes

- Reload called with unreadable cert file → diagnostic, previous
  cert remains active.
- Reload called with mismatched cert/key pair → diagnostic,
  previous cert remains active.
- Reload called during heavy traffic → swap is per-handshake; no
  observable behaviour change beyond newer connections seeing the
  new cert.

### Test plan

- Unit: build a `ReloadableCertResolver`, swap, verify resolved
  cert changes.
- Integration: spin up a TLS-enabled server, make a TLS request,
  reload with a different cert pair, make another request,
  verify the second request sees the new cert (via cert
  fingerprint comparison).
- Failure: reload with malformed cert, verify diagnostic surfaces
  and the previous cert continues.

## Drawbacks

1. **Adds operational complexity surface.** "Hot reload" is more
   moving parts than "restart"; failure modes (partial reload,
   stuck old cert) need clear diagnostics.
2. **One new dependency (`arc-swap` if Path 1 chosen; none for Path
   2 if using `RwLock` / `Mutex`).** Small.
3. **Behaviour change for the `TlsCert*` keys.** Operators who
   scripted around `restart_required` may see scripts no longer
   trigger restart for cert rotation. This is the desired
   behaviour, but flag it in the CHANGELOG.

## Rationale and alternatives

**Outcome A — Implement hot-reload for both `TlsEnabled` and
`TlsCert*`.** Largest scope. Requires re-bindable listener for
TLS toggle, which crosses into tokio listener restart territory —
nontrivial.

**Outcome B — Withdraw RFC, document restart-required as
fundamental.** Cheapest. Loses real operational value
(cert rotation is the common case and IS hot-reloadable).

**Outcome C (this RFC's recommendation) — Hot-reload for
`TlsCert*` only; `TlsEnabled` stays `HardRestart`.** Best
cost/value: handles the common rotation case, avoids the listener-
rebinding complexity of toggling TLS on/off.

If reviewers prefer B (because the audit work itself was the goal
and shipping new TLS code is too risky for v5.11), this RFC moves
to `archive/` with status `Withdrawn — audit confirmed conservative
restart-required classification; see audit findings for details`.
The audit findings are preserved as the RFC's body.

## Prior art

- nginx's `nginx -s reload` rotates certs without dropping
  connections by spawning new workers with the new cert and
  draining old ones. A different model (process-pool) but the
  same operational guarantee.
- Caddy's auto-cert rotation uses rustls under the hood with a
  custom `ResolvesServerCert` that loads from its cert cache. The
  pattern this RFC proposes mirrors Caddy's approach.
- Linkerd's data-plane proxies rotate mTLS identity certs every
  24h via a similar `ArcSwap`-based mechanism.

## Unresolved questions

1. **Control surface for triggering reload.** ✅ **Resolved.**
   Function call on the server handle, surfaced through the
   workspace `apply()` result. Matches the existing reload-hint
   flow; signal handler (`SIGHUP`) can be added later if operators
   ask for it.
2. **Should the GUI auto-trigger reload on a TLS cert-path apply?**
   Keep user-in-the-loop for v5.11; the apply-result hint is the
   contract. Auto-trigger can be added as an opt-in later if user
   feedback supports it.
3. **`TlsEnabled` toggle without restart — is it really impossible,
   or just hard?** ✅ **Resolved (deferred).** Outcome C explicitly
   keeps `TlsEnabled` as `HardRestart`. The cost (interrupted
   in-flight connections + listener rebind machinery) outweighs the
   benefit for a setting that's typically configured once per
   deployment. Re-evaluate only if real user demand appears.
4. **Encrypted-key passphrase.** RFC 003 Unresolved §3 flagged
   this. Not in scope here; passphrase-protected keys are a
   separate feature.

## Future possibilities

- Auto-reload watch — file-system watcher on the cert file path
  triggers a reload automatically when the file changes. Useful for
  certbot deploy hooks. Out of scope but a natural follow-up.
- mTLS (client cert verification) reload — same resolver pattern
  applies to the client cert validator.
- A `reload-tls` CLI subcommand (`apimock reload-tls`) for
  scripted use, parallel to `apimock match-test` (RFC 015).
