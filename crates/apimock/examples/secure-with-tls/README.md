# Serve over TLS

`listener.tls` with `port` omitted: the single configured port serves
HTTPS only, no plaintext HTTP fallback.

`cert.pem` / `key.pem` here are a throwaway self-signed test
certificate (`CN=test`, valid 2026-2036) - the same one this project's
own test suite uses. It's fine for local mocking; never reuse a
checked-in cert/key for anything that needs to be actually secure.

## Run it

```sh
cd crates/apimock/examples/secure-with-tls
apimock
```

## Try it

`curl` needs `-k` (or `--insecure`) since the certificate is
self-signed and not in any trust store:

```sh
$ curl -k https://127.0.0.1:3443/health
ok, over https
```

## Dual HTTP + HTTPS

Setting `listener.tls.port` instead serves plaintext HTTP on
`listener.port` and HTTPS on `listener.tls.port` simultaneously - not
shown here, to keep this example to one port and one behaviour.
