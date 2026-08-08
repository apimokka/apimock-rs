# Serve over HTTPS

```toml
[listener]
ip_address = "127.0.0.1"
port = 3443

[listener.tls]
cert = "./cert.pem"
key = "./key.pem"
# port omitted -> this single port serves HTTPS only
```

With `listener.tls.port` omitted, the port in `[listener]` becomes
HTTPS-only — no plaintext HTTP listener starts at all. Set
`listener.tls.port` to a *different* number instead, and both start:
plain HTTP on `listener.port`, HTTPS on `listener.tls.port`.

**`cert`/`key` paths are resolved against the process's current
directory, not against `apimock.toml`'s own location** — run `apimock`
from the directory containing the PEM files, or use absolute paths.
See [`apimock.toml` root settings](../reference/apimock-toml-root-settings.md#listenertls)
for the full field list.

Testing against a self-signed certificate needs `curl -k`
(`--insecure`) since it isn't in any trust store:

```sh
curl -k https://127.0.0.1:3443/health
```

A worked, verified example — including a throwaway self-signed test
certificate safe to reuse for local mocking — is
[`crates/apimock/examples/secure-with-tls/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/secure-with-tls).

For rotating a certificate without restarting the process, see
[Reload TLS certificates without restart](./reload-tls-certificates-without-restart.md).
