# CORS (Cross-Origin Resource Sharing)

## Default CORS variables in response

| Header | Value |
| --- | --- |
| `Access-Control-Allow-Methods` | `GET, POST, PUT, DELETE, OPTIONS` |
| `Access-Control-Allow-Headers` | `*` |
| `Access-Control-Max-Age` | `86400` |

## Variation due to request

If `Origin` is got from request and looks like authorized request including `Cookie` or `Authorization` header:

| Header | Value |
| --- | --- |
| `Access-Control-Allow-Origin` | Request `Origin` value |
| `Vary` | `Origin` |
| `Access-Control-Allow-Credentials` | `true` |

Else:

| Header | Value |
| --- | --- |
| `Access-Control-Allow-Origin` | `*` |
| `Vary` | `*` |
| `Access-Control-Allow-Credentials` | (not set) |
