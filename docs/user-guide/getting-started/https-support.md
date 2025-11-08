# HTTPS support

HTTPS protocol is supported.
You can build each of:

- A single server to listen to only HTTPS.
- Servers to listen to both HTTP and HTTPS.

## Which port(s) are used ?

The root configuration has two options on port number:

- `listener.port`
- `listener.tls.port`

How they work is:

| `listener.tls.port` condition | result |
| --- | --- |
| Not specified (omitted) | `listener.port` is used as HTTPS port. |
| Specified | `listener.tls.port` is used as HTTPS port. `listener.port` is as HTTP. |

## Configuration example

```diff
  [listener]
  ip_address = "127.0.0.1"
  port = 3001
+ [listener.tls]
+ cert = "./cert.pem"
+ key = "./key.pem"
+ # port = 3002 # if omit, only https listener will start. if specified, both http and https listeners will start.
```

### `openssl` command lines to generate private key and certificate

Note that this is an example to generate self-signed certificates for testing.

```sh
openssl genrsa -out key.pem 2048
openssl req -x509 -sha256 -days 365 -key key.pem -out cert.pem -subj "/CN=l
ocalhost"
```
